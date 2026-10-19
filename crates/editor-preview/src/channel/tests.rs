use super::*;
fn pipe() -> (OwnedFd, OwnedFd) {
    let mut fds = [-1; 2];
    // SAFETY: fixed two-FD output and atomic fixture flags.
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
        0
    );
    // SAFETY: successful pipe2 produced two fresh owned descriptors.
    unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
}
fn eof(fd: BorrowedFd<'_>) {
    // Other test threads can fork while a CLOEXEC writer is live. Its
    // temporary inherited copy closes at exec, not atomically at fork.
    // Require bounded EOF instead of assuming an instantaneous transition.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let mut byte = [0u8; 1];
        // SAFETY: one live output byte on this nonblocking fixture pipe.
        let count = unsafe { libc::read(fd.as_raw_fd(), byte.as_mut_ptr().cast(), 1) };
        if count == 0 {
            return;
        }
        let error = io::Error::last_os_error();
        assert!(
            count < 0
                && matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ),
            "Unexpected pipe data/error: {count}, {error}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "Transferred writer remained open after the teardown deadline"
        );
        let mut event = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: fixed live descriptor, bounded wait avoids busy polling.
        let count = unsafe { libc::poll(&mut event, 1, 10) };
        assert!(count >= 0 || io::Error::last_os_error().kind() == io::ErrorKind::Interrupted);
    }
}

fn raw(fd: BorrowedFd<'_>, bytes: &[u8], rights: &[BorrowedFd<'_>]) {
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let space = unsafe { libc::CMSG_SPACE((rights.len() * mem::size_of::<i32>()) as u32) } as usize;
    let mut control = vec![0usize; space.div_ceil(mem::size_of::<usize>())];
    // SAFETY: zeroed message initialized below with bounded fixture buffers.
    let mut message: libc::msghdr = unsafe { mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    if !rights.is_empty() {
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = space;
        // SAFETY: correctly sized aligned storage and initialized FD inputs.
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len =
                libc::CMSG_LEN((rights.len() * mem::size_of::<i32>()) as u32) as usize;
            for (index, fd) in rights.iter().enumerate() {
                libc::CMSG_DATA(header)
                    .cast::<i32>()
                    .add(index)
                    .write_unaligned(fd.as_raw_fd());
            }
        }
    }
    // SAFETY: one atomic fixture packet; no pointer retained by sendmsg.
    assert_eq!(
        unsafe {
            libc::sendmsg(
                fd.as_raw_fd(),
                &message,
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        },
        bytes.len() as isize
    );
}
#[test]
fn malformed_packets_truncated_rights_and_missing_credentials_close_every_writer() {
    for case in 0..7 {
        let session = frame::Session::new().unwrap();
        let (left, right) = Endpoint::pair().unwrap();
        if case == 6 {
            set_option(right.as_fd(), libc::SO_PASSCRED, 0).unwrap();
        }
        let (read, write) = pipe();
        let good = Packet {
            kind: FRAME,
            serial: 1,
        }
        .bytes();
        let mut bytes = good.to_vec();
        match case {
            0 => {
                bytes.pop();
            }
            1 => bytes.push(0),
            2 => bytes.clear(),
            _ => (),
        }
        let count = match case {
            3 => 0,
            4 => 2,
            5 => 32,
            _ => 1,
        };
        raw(left.as_fd(), &bytes, &vec![write.as_fd(); count]);
        drop(write);
        let mut receiver = right
            .receiver(
                Peer::current().unwrap(),
                frame::Receiver::new(session, 1).unwrap(),
            )
            .unwrap();
        assert!(receiver.try_receive().is_err(), "case {case}");
        assert!(receiver.poll_descriptor().is_err());
        assert!(receiver.latest().is_none());
        eof(read.as_fd());
    }
}
#[test]
fn every_packet_header_bit_change_is_rejected_without_leaking_a_received_descriptor() {
    for offset in 0..BYTES {
        for bit in 0..8 {
            let (left, right) = Endpoint::pair().unwrap();
            let (read, write) = pipe();
            let mut bytes = Packet {
                kind: FRAME,
                serial: 1,
            }
            .bytes();
            bytes[offset] ^= 1 << bit;
            raw(left.as_fd(), &bytes, &[write.as_fd()]);
            drop(write);
            let mut receiver = right
                .receiver(
                    Peer::current().unwrap(),
                    frame::Receiver::new(frame::Session::new().unwrap(), 1).unwrap(),
                )
                .unwrap();
            assert!(receiver.try_receive().is_err(), "byte {offset} bit {bit}");
            assert!(receiver.poll_descriptor().is_err());
            eof(read.as_fd());
        }
    }
}
#[test]
fn receipt_roles_sequences_and_unexpected_rights_fail_closed_but_valid_drops_return_credit() {
    for case in 0..5 {
        let (left, right) = Endpoint::pair().unwrap();
        let (read, write) = pipe();
        let mut sender = left.sender(Peer::current().unwrap()).unwrap();
        // A real packet/FD transfer establishes the pending transport serial.
        let frame = frame::seal_rgba(
            frame::Session::new().unwrap(),
            1,
            1,
            frame::Dimensions {
                width: 1,
                height: 1,
            },
            &[0; 4],
        )
        .unwrap();
        sender.try_send(frame).unwrap();
        let inbound = receive(right.as_fd()).unwrap();
        assert!(inbound.descriptor.is_some());
        let packet = Packet {
            kind: if case == 0 {
                DROPPED
            } else if case == 1 {
                FRAME
            } else {
                ACCEPTED
            },
            serial: if case == 2 { 2 } else { 1 },
        };
        let mut bytes = packet.bytes().to_vec();
        if case == 3 {
            bytes[9] = 1;
        }
        let write_ref = write.as_fd();
        raw(
            right.as_fd(),
            &bytes,
            if case == 4 {
                std::slice::from_ref(&write_ref)
            } else {
                &[]
            },
        );
        drop(write);
        if case == 0 {
            assert_eq!(sender.try_receipt().unwrap(), Receipt::Dropped);
            assert!(sender.can_send());
        } else {
            assert!(sender.try_receipt().is_err());
            assert!(!sender.can_send());
            assert!(sender.poll_descriptor().is_err());
        }
        eof(read.as_fd());
    }
}
#[test]
fn incoming_rights_are_cloexec_and_endpoints_are_nonblocking_with_sender_credentials() {
    let (left, right) = Endpoint::pair().unwrap();
    for endpoint in [&left, &right] {
        assert_eq!(option(endpoint.as_fd(), libc::SO_PASSCRED).unwrap(), 1);
        assert!(option(endpoint.as_fd(), libc::SO_SNDBUF).unwrap() <= 8192);
        // SAFETY: scalar flags read from owned live endpoints.
        assert_ne!(
            unsafe { libc::fcntl(endpoint.as_fd().as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
            0
        );
    }
    let (read, write) = pipe();
    send(
        left.as_fd(),
        Packet {
            kind: FRAME,
            serial: 1,
        },
        Some(write.as_fd()),
    )
    .unwrap();
    drop(write);
    let message = receive(right.as_fd()).unwrap();
    assert!(Peer::current().unwrap().matches(message.credential));
    let descriptor = message.descriptor.unwrap();
    // SAFETY: scalar flags read from the actual received capability.
    assert_ne!(
        unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    drop(descriptor);
    eof(read.as_fd());
}
#[test]
fn blocked_receipts_prevent_more_frame_reads_and_stop_closes_queued_rights() {
    let (left, right) = Endpoint::pair().unwrap();
    let mut saturated = false;
    for _ in 0..128 {
        match send(
            right.as_fd(),
            Packet {
                kind: ACCEPTED,
                serial: 99,
            },
            None,
        ) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                saturated = true;
                break;
            }
            Err(e) => panic!("Unexpected queue result: {e}"),
        }
    }
    assert!(
        saturated,
        "Supported small socket queue did not saturate within the fixture bound"
    );
    let session = frame::Session::new().unwrap();
    let payload = frame::seal_rgba(
        session,
        1,
        1,
        frame::Dimensions {
            width: 1,
            height: 1,
        },
        &[7; 4],
    )
    .unwrap();
    send(
        left.as_fd(),
        Packet {
            kind: FRAME,
            serial: 1,
        },
        Some(payload.as_fd()),
    )
    .unwrap();
    drop(payload);
    let mut receiver = right
        .receiver(
            Peer::current().unwrap(),
            frame::Receiver::new(session, 1).unwrap(),
        )
        .unwrap();
    assert_eq!(receiver.try_receive().unwrap(), Receipt::Accepted);
    assert!(receiver.pending.is_some());
    let (read, write) = pipe();
    send(
        left.as_fd(),
        Packet {
            kind: FRAME,
            serial: 2,
        },
        Some(write.as_fd()),
    )
    .unwrap();
    drop(write);
    assert_eq!(
        receiver.try_receive().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(receiver.serial, 1);
    assert_eq!(receiver.latest().unwrap().pixels(), &[7; 4]);
    let mut drained = false;
    for _ in 0..128 {
        match receive(left.as_fd()) {
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                drained = true;
                break;
            }
            Err(e) => panic!("Unexpected receipt drain: {e}"),
        }
    }
    assert!(drained);
    receiver.flush_receipt().unwrap();
    assert!(receiver.pending.is_none());
    receiver.stop();
    assert_eq!(receiver.retained_bytes(), 0);
    assert!(receiver.latest().is_none());
    eof(read.as_fd());
}
#[test]
fn unexpected_pidfd_records_are_closed_and_inheritance_disables_them() {
    const MARKER: &str = "HYE_PIDFD_CMSG_FIXTURE";
    if std::env::var_os(MARKER).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "channel::tests::unexpected_pidfd_records_are_closed_and_inheritance_disables_them",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env(MARKER, "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Private pidfd fixture exceeded its deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("PIDFD record path:") || text.contains("PIDFD not supported:"));
        for line in text.lines().filter(|line| {
            line.contains("PIDFD record path:") || line.contains("PIDFD not supported:")
        }) {
            println!("{line}");
        }
        return;
    }
    let (left, right) = Endpoint::pair().unwrap();
    match set_option(right.as_fd(), libc::SO_PASSPIDFD, 1) {
        Err(e) if e.raw_os_error() == Some(libc::ENOPROTOOPT) => {
            // This older kernel cannot emit the record. Still verify that
            // inheritance handles only that exact unsupported-option error.
            Endpoint::inherited(right.into_descriptor()).unwrap();
            println!("PIDFD not supported: inherited endpoint validated");
        }
        Err(e) => panic!("Cannot exercise PIDFD records: {e}"),
        Ok(()) => {
            let right = Endpoint::inherited(right.into_descriptor()).unwrap();
            assert_eq!(option(right.as_fd(), libc::SO_PASSPIDFD).unwrap(), 0);
            // Simulate an unrequested option enabled through a duplicate.
            set_option(right.as_fd(), libc::SO_PASSPIDFD, 1).unwrap();
            // SAFETY: private child-only soft descriptor limit; no host change.
            let mut limits: libc::rlimit = unsafe { mem::zeroed() };
            assert_eq!(
                unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits) },
                0
            );
            limits.rlim_cur = limits.rlim_cur.min(64);
            assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limits) }, 0);
            let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
            for _ in 0..256 {
                send(
                    left.as_fd(),
                    Packet {
                        kind: ACCEPTED,
                        serial: 1,
                    },
                    None,
                )
                .unwrap();
                assert_eq!(
                    receive(right.as_fd()).err().unwrap().kind(),
                    io::ErrorKind::InvalidData
                );
            }
            assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
            println!("PIDFD record path: 256 refusals, descriptor count unchanged");
        }
    }
}
#[test]
fn named_abstract_sockets_are_not_accepted_as_private_inherited_pairs() {
    // SAFETY: fixed socket arguments; no host address is contacted.
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };
    assert!(raw >= 0);
    // SAFETY: fresh descriptor returned above.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    // SAFETY: zeroed fixed sockaddr buffer populated within its bounds.
    let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let name = format!("hye-channel-fixture-{}", std::process::id());
    for (slot, byte) in address.sun_path[1..].iter_mut().zip(name.bytes()) {
        *slot = byte as libc::c_char;
    }
    // SAFETY: unique test-only abstract address, disappears when fd closes.
    assert_eq!(
        unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                (mem::size_of::<libc::sa_family_t>() + 1 + name.len()) as libc::socklen_t,
            )
        },
        0
    );
    assert!(Endpoint::inherited(fd)
        .err()
        .unwrap()
        .to_string()
        .contains("unnamed"));
}
