//! Opt-in CI roles. The provider creates a fresh PID namespace before dropping
//! privilege; no compositor, user namespace or private mount/proc is certified.
use super::*;
use std::{
    io::Read,
    time::{Duration, Instant},
};
const ROLE: &str = "HYE_PIDNS_CHANNEL_ROLE";
fn inherited(name: &str) -> OwnedFd {
    let raw = std::env::var(name).unwrap().parse::<i32>().unwrap();
    assert!(raw >= 0);
    // SAFETY: validates the number and duplicates it into new owned authority.
    let copied = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 3) };
    assert!(copied >= 0);
    unsafe { OwnedFd::from_raw_fd(copied) }
}
fn retry<T>(mut action: impl FnMut() -> io::Result<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match action() {
            Ok(value) => return value,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(2))
            }
            Err(e) => panic!("Private namespace channel failed: {e}"),
        }
    }
}
fn unprivileged() {
    assert!(unsafe { libc::getuid() } > 0);
    assert_eq!(unsafe { libc::getuid() }, unsafe { libc::geteuid() });
    assert_eq!(unsafe { libc::getgid() }, unsafe { libc::getegid() });
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    assert_eq!(
        unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) },
        1
    );
    let mut status = String::new();
    std::fs::File::open("/proc/self/status")
        .unwrap()
        .take(16385)
        .read_to_string(&mut status)
        .unwrap();
    assert!(status.len() <= 16384);
    for name in ["CapPrm:", "CapEff:", "CapBnd:", "CapAmb:"] {
        let value = status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap()
            .trim();
        assert_eq!(u64::from_str_radix(value, 16).unwrap(), 0);
    }
}
#[test]
#[ignore = "requires the dedicated ephemeral CI namespace provider; never auto-select a host session"]
fn pidfs_frame_and_receipt_cross_a_private_pid_namespace() {
    unprivileged();
    let role = std::env::var(ROLE).expect("Explicit trusted CI role required");
    let endpoint = Endpoint::pidfs_inherited(inherited("HYE_PIDNS_DATA_FD")).unwrap();
    if role == "gui" {
        let bootstrap = inherited("HYE_PIDNS_BOOTSTRAP_FD");
        let session = frame::Session::new().unwrap();
        let pixels = frame::seal_rgba(
            session,
            1,
            1,
            frame::Dimensions {
                width: 1,
                height: 1,
            },
            &[9, 8, 7, 255],
        )
        .unwrap();
        retry(|| {
            send(
                bootstrap.as_fd(),
                Packet {
                    kind: FRAME,
                    serial: 1,
                },
                Some(pixels.as_fd()),
            )
        });
        drop(pixels);
        let message = retry(|| receive(bootstrap.as_fd()));
        assert_eq!(message.packet.kind, FRAME);
        assert_eq!(message.packet.serial, 1);
        assert_eq!(message.credential.uid, 0);
        assert_eq!(message.credential.pid, unsafe { libc::getppid() });
        let pin = message
            .descriptor
            .expect("Live process anchor from the trusted provider");
        let peer = Peer::from_pidfd(pin.as_fd()).unwrap();
        drop(pin);
        let mut receiver = endpoint
            .receiver(peer, frame::Receiver::new(session, 1).unwrap())
            .unwrap();
        assert_eq!(retry(|| receiver.try_receive()), Receipt::Accepted);
        assert_eq!(receiver.latest().unwrap().pixels(), &[9, 8, 7, 255]);
        let final_message = retry(|| receive(bootstrap.as_fd()));
        assert_eq!(final_message.packet.kind, ACCEPTED);
        assert_eq!(final_message.packet.serial, 2);
        assert!(final_message.descriptor.is_none());
        assert_eq!(final_message.credential.uid, 0);
        assert_eq!(final_message.credential.pid, unsafe { libc::getppid() });
        receiver.stop();
        assert_eq!(receiver.retained_bytes(), 0);
        println!("HYE_PIDNS_RESULT GUI accepted exact pixels and released buffers");
    } else {
        assert_eq!(role, "broker");
        assert_eq!(unsafe { libc::getpid() }, 1);
        assert_eq!(unsafe { libc::getppid() }, 0);
        let anchor = inherited("HYE_PIDNS_ANCHOR_FD");
        let payload = inherited("HYE_PIDNS_FRAME_FD");
        let mut sender = endpoint
            .sender(Peer::from_pidfd(anchor.as_fd()).unwrap())
            .unwrap();
        drop(anchor);
        sender.try_send(payload).unwrap();
        assert_eq!(retry(|| sender.try_receipt()), Receipt::Accepted);
        assert!(sender.can_send());
        sender.stop();
        println!("HYE_PIDNS_RESULT Broker PID1/parent0 accepted outer-namespace receipt");
    }
}
