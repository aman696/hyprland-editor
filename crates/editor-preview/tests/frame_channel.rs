//! Kernel IPC and a real, privately launched Rust child. No compositor or host socket.
use hyprland_editor_preview::{
    channel::{Endpoint, Peer, Receipt},
    frame::{self, Dimensions, Session},
};
use std::{
    io,
    os::{
        fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
fn retry<T>(mut action: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let start = Instant::now();
    loop {
        match action() {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) && start.elapsed() < Duration::from_secs(5) =>
            {
                std::thread::sleep(Duration::from_millis(2))
            }
            result => return result,
        }
    }
}
fn pixels(session: Session, sequence: u64) -> OwnedFd {
    frame::seal_rgba(
        session,
        5,
        sequence,
        Dimensions {
            width: 2,
            height: 1,
        },
        &[1, 2, 3, 4, 5, 6, 7, 8],
    )
    .unwrap()
}
#[test]
fn one_credit_receipt_and_poll_descriptors_keep_synthetic_frame_bytes_exact() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pair().unwrap();
    let mut sender = left.sender(Peer::current().unwrap()).unwrap();
    let mut receiver = right
        .receiver(
            Peer::current().unwrap(),
            frame::Receiver::new(session, 5).unwrap(),
        )
        .unwrap();
    assert!(sender.can_send());
    assert_eq!(
        receiver.try_receive().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    sender.try_send(pixels(session, 1)).unwrap();
    assert!(!sender.can_send());
    assert_eq!(
        sender.try_send(pixels(session, 2)).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    let mut event = libc::pollfd {
        fd: receiver.poll_descriptor().unwrap().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one initialized pollfd and no blocking wait.
    assert_eq!(unsafe { libc::poll(&mut event, 1, 0) }, 1);
    assert_ne!(event.revents & libc::POLLIN, 0);
    assert_eq!(receiver.try_receive().unwrap(), Receipt::Accepted);
    assert_eq!(sender.try_receipt().unwrap(), Receipt::Accepted);
    assert!(sender.can_send());
    assert_eq!(
        receiver.latest().unwrap().pixels(),
        &[1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_eq!(receiver.latest().unwrap().sequence, 1);
    sender.try_send(pixels(session, 2)).unwrap();
    let result = receiver.try_receive().unwrap();
    assert_eq!(sender.try_receipt().unwrap(), result);
    assert!(sender.can_send());
    assert_eq!(
        receiver.latest().unwrap().sequence,
        if result == Receipt::Accepted { 2 } else { 1 }
    );
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn high(fd: std::os::fd::BorrowedFd<'_>) -> OwnedFd {
    // SAFETY: duplicate a held fixture descriptor outside fixed child targets.
    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
    assert!(duplicate >= 64);
    // SAFETY: fcntl returned a fresh owned descriptor.
    unsafe { OwnedFd::from_raw_fd(duplicate) }
}
fn spawn(mode: &str, socket: Endpoint, frame: OwnedFd) -> Process {
    let socket = high(socket.as_fd());
    let frame = high(frame.as_fd());
    let anchor = if mode.starts_with("pidfs") {
        Some(high(Peer::current_pidfs().unwrap().process_descriptor()))
    } else {
        None
    };
    let anchor_fd = anchor.as_ref().map(AsRawFd::as_raw_fd);
    let socket_fd = socket.as_raw_fd();
    let frame_fd = frame.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.env_clear().env("LC_ALL", "C");
    command
        .args([
            "--exact",
            "credentials_and_sealed_frames_cross_a_real_child_with_one_inflight_receipt",
            "--nocapture",
        ])
        .env("HYE_FRAME_CHANNEL_FIXTURE", mode)
        .env("HYE_FRAME_CHANNEL_PARENT", std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: before exec, use only async-signal-safe dup2; both source FDs are
    // held and distinct from targets 3/4. Every other fixture FD is CLOEXEC.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(socket_fd, 3) < 0 || libc::dup2(frame_fd, 4) < 0 {
                return Err(io::Error::last_os_error());
            }
            if let Some(anchor) = anchor_fd {
                if libc::dup2(anchor, 5) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Process(command.spawn().unwrap())
}
fn worker(mode: &str) {
    // SAFETY: this private test launcher transfers unique owned FDs 3/4 at exec.
    let (endpoint, payload) = unsafe { (OwnedFd::from_raw_fd(3), OwnedFd::from_raw_fd(4)) };
    let mut sender = if mode.starts_with("pidfs") {
        // SAFETY: this role's private launcher transferred one owned anchor at5.
        let anchor = unsafe { OwnedFd::from_raw_fd(5) };
        Endpoint::pidfs_inherited(endpoint)
            .unwrap()
            .sender(Peer::from_pidfd(anchor.as_fd()).unwrap())
            .unwrap()
    } else {
        let parent = std::env::var("HYE_FRAME_CHANNEL_PARENT")
            .unwrap()
            .parse()
            .unwrap();
        Endpoint::inherited(endpoint)
            .unwrap()
            .sender(Peer::parent(parent).unwrap())
            .unwrap()
    };
    if mode == "wait" {
        std::thread::sleep(Duration::from_secs(5));
        panic!("Parent did not finish its private fixture");
    }
    sender.try_send(payload).unwrap();
    let result = retry(|| sender.try_receipt());
    if mode == "wrong-peer" || mode == "pidfs-wrong" {
        assert!(result.is_err());
    } else {
        assert_eq!(result.unwrap(), Receipt::Accepted);
    }
}
#[test]
fn credentials_and_sealed_frames_cross_a_real_child_with_one_inflight_receipt() {
    if let Ok(mode) = std::env::var("HYE_FRAME_CHANNEL_FIXTURE") {
        worker(&mode);
        return;
    }
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pair().unwrap();
    let mut child = spawn("send", right, pixels(session, 1));
    let peer = Peer::child(&mut child.0).unwrap();
    let mut receiver = left
        .receiver(peer, frame::Receiver::new(session, 5).unwrap())
        .unwrap();
    assert_eq!(retry(|| receiver.try_receive()).unwrap(), Receipt::Accepted);
    assert_eq!(
        receiver.latest().unwrap().pixels(),
        &[1, 2, 3, 4, 5, 6, 7, 8]
    );
    let status = retry(|| {
        child
            .0
            .try_wait()?
            .ok_or_else(|| io::Error::from(io::ErrorKind::WouldBlock))
    })
    .unwrap();
    assert!(status.success());
}
#[test]
fn inherited_socket_creator_credentials_do_not_authorize_a_different_live_sender() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pair().unwrap();
    let mut child = spawn("wrong-peer", right, pixels(session, 1));
    // Socketpair SO_PEERCRED still names this creator, but each kernel message
    // must identify the actual child. Selecting the creator must fail closed.
    let mut receiver = left
        .receiver(
            Peer::current().unwrap(),
            frame::Receiver::new(session, 5).unwrap(),
        )
        .unwrap();
    let error = retry(|| receiver.try_receive()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("unexpected process"));
    assert!(receiver.poll_descriptor().is_err());
    assert!(receiver.latest().is_none());
    let status = retry(|| {
        child
            .0
            .try_wait()?
            .ok_or_else(|| io::Error::from(io::ErrorKind::WouldBlock))
    })
    .unwrap();
    assert!(status.success());
}
#[test]
fn dead_child_process_pin_refuses_the_channel_and_cannot_be_restored_from_a_pid() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pair().unwrap();
    let mut child = spawn("wait", right, pixels(session, 1));
    let peer = Peer::child(&mut child.0).unwrap();
    let mut receiver = left
        .receiver(peer, frame::Receiver::new(session, 5).unwrap())
        .unwrap();
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let mut pin_event = libc::pollfd {
        fd: receiver.peer_process_descriptor().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one held process descriptor, zero-timeout observation only.
    assert_eq!(unsafe { libc::poll(&mut pin_event, 1, 0) }, 1);
    assert_ne!(pin_event.revents & libc::POLLIN, 0);
    assert_eq!(
        receiver.try_receive().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(receiver.poll_descriptor().is_err());
    assert!(Peer::child(&mut child.0).is_err());
    assert!(Peer::parent(std::process::id()).is_err());
}
#[test]
fn ordinary_files_stream_sockets_and_closed_peers_cannot_become_frame_endpoints() {
    assert!(Endpoint::inherited(std::fs::File::open("/dev/null").unwrap().into()).is_err());
    let (stream, _) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(Endpoint::inherited(stream.into()).is_err());
    let (left, right) = Endpoint::pair().unwrap();
    let mut receiver = left
        .receiver(
            Peer::current().unwrap(),
            frame::Receiver::new(Session::new().unwrap(), 1).unwrap(),
        )
        .unwrap();
    drop(right);
    assert_eq!(
        receiver.try_receive().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(receiver.poll_descriptor().is_err());
}

#[test]
fn valid_pixels_are_released_on_explicit_stop_disconnect_or_invalid_next_frame() {
    for scenario in 0..3 {
        let session = Session::new().unwrap();
        let (left, right) = Endpoint::pair().unwrap();
        let mut sender = left.sender(Peer::current().unwrap()).unwrap();
        let mut receiver = right
            .receiver(
                Peer::current().unwrap(),
                frame::Receiver::new(session, 5).unwrap(),
            )
            .unwrap();
        sender.try_send(pixels(session, 1)).unwrap();
        assert_eq!(receiver.try_receive().unwrap(), Receipt::Accepted);
        assert_eq!(sender.try_receipt().unwrap(), Receipt::Accepted);
        assert_eq!(receiver.retained_bytes(), 8);
        match scenario {
            0 => receiver.stop(),
            1 => {
                sender.stop();
                assert_eq!(
                    receiver.try_receive().unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
            }
            _ => {
                sender.try_send(pixels(Session::new().unwrap(), 2)).unwrap();
                assert_eq!(
                    receiver.try_receive().unwrap_err().kind(),
                    io::ErrorKind::InvalidData
                );
            }
        }
        assert!(receiver.latest().is_none());
        assert_eq!(receiver.retained_bytes(), 0);
        assert!(receiver.poll_descriptor().is_err());
        assert!(sender.try_send(pixels(session, 3)).is_err());
    }
}
#[test]
fn sender_observes_a_dead_pinned_child_and_closes_instead_of_waiting_for_a_receipt() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pair().unwrap();
    let mut child = spawn("wait", right, pixels(session, 1));
    let peer = Peer::child(&mut child.0).unwrap();
    let mut sender = left.sender(peer).unwrap();
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(!sender.can_send());
    assert_eq!(
        sender.try_send(pixels(session, 1)).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(sender.poll_descriptor().is_err());
}

#[test]
fn pidfs_profile_and_explicit_anchor_transfer_real_child_frames_and_receipts() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pidfs_pair().unwrap();
    let mut child = spawn("pidfs", right, pixels(session, 1));
    let mut receiver = left
        .receiver(
            Peer::child_pidfs(&mut child.0).unwrap(),
            frame::Receiver::new(session, 5).unwrap(),
        )
        .unwrap();
    assert_eq!(retry(|| receiver.try_receive()).unwrap(), Receipt::Accepted);
    assert_eq!(
        receiver.latest().unwrap().pixels(),
        &[1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert!(retry(|| child
        .0
        .try_wait()?
        .ok_or_else(|| io::Error::from(io::ErrorKind::WouldBlock)))
    .unwrap()
    .success());
}
#[test]
fn pidfs_auth_refuses_another_live_process_even_with_matching_user_credentials() {
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pidfs_pair().unwrap();
    let mut child = spawn("pidfs-wrong", right, pixels(session, 1));
    let mut receiver = left
        .receiver(
            Peer::current_pidfs().unwrap(),
            frame::Receiver::new(session, 5).unwrap(),
        )
        .unwrap();
    assert_eq!(
        retry(|| receiver.try_receive()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(receiver.latest().is_none());
    assert!(receiver.poll_descriptor().is_err());
    assert!(retry(|| child
        .0
        .try_wait()?
        .ok_or_else(|| io::Error::from(io::ErrorKind::WouldBlock)))
    .unwrap()
    .success());
}
#[test]
fn identity_profiles_and_plain_or_dead_descriptors_cannot_replace_process_capabilities() {
    let (left, _) = Endpoint::pidfs_pair().unwrap();
    assert!(left.sender(Peer::current().unwrap()).is_err());
    let (left, _) = Endpoint::pair().unwrap();
    assert!(left.sender(Peer::current_pidfs().unwrap()).is_err());
    for path in ["/dev/null", "/proc/self", "/tmp"] {
        assert!(Peer::from_pidfd(std::fs::File::open(path).unwrap().as_fd()).is_err());
    }
    let session = Session::new().unwrap();
    let (left, right) = Endpoint::pidfs_pair().unwrap();
    let mut child = spawn("wait", right, pixels(session, 1));
    let peer = Peer::child_pidfs(&mut child.0).unwrap();
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(Peer::from_pidfd(peer.process_descriptor()).is_err());
    drop(left);
}
#[test]
fn pidfs_process_identity_rejects_thread_scoped_handles() {
    // SAFETY: fixed thread ID/flag queries; successful pidfd_open creates ownership.
    let raw = unsafe {
        libc::syscall(
            libc::SYS_pidfd_open,
            libc::syscall(libc::SYS_gettid),
            libc::O_EXCL,
        )
    };
    assert!(
        raw >= 0,
        "This tested pidfs profile requires thread-scoped pidfd support"
    );
    let thread = unsafe { OwnedFd::from_raw_fd(raw as i32) };
    assert_eq!(
        Peer::from_pidfd(thread.as_fd()).err().unwrap().kind(),
        io::ErrorKind::InvalidData
    );
}
