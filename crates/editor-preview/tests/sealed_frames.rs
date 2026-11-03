//! Real Linux sealing and bounded pixel reads; no compositor/rendering certificate.
use hyprland_editor_preview::frame::{self, Dimensions, Receiver, Session};
use std::{
    fs::File,
    io::{self, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn raw(bytes: &[u8], seals: i32, length: Option<u64>) -> OwnedFd {
    // SAFETY: fixed name/flags, fresh owned descriptor on success.
    let fd = unsafe {
        libc::memfd_create(
            c"hye-frame-fixture".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    assert!(fd >= 0);
    // SAFETY: descriptor was freshly created above.
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(bytes).unwrap();
    if let Some(length) = length {
        file.set_len(length).unwrap();
    }
    // SAFETY: live fixture descriptor and fixed scalar flags.
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, seals) },
        0
    );
    file.into()
}
const ALL: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
fn bytes(fd: OwnedFd) -> Vec<u8> {
    use std::os::unix::fs::FileExt;
    let file = File::from(fd);
    let mut bytes = vec![0; file.metadata().unwrap().len() as usize];
    file.read_exact_at(&mut bytes, 0).unwrap();
    bytes
}
#[test]
fn sealed_pixels_are_exact_immutable_and_independent_of_the_descriptor_offset() {
    let session = Session::new().unwrap();
    let size = Dimensions {
        width: 2,
        height: 1,
    };
    let pixels = [0, 0, 0, 0, 255, 128, 1, 255];
    let fd = frame::seal_rgba(session, 7, 1, size, &pixels).unwrap();
    // SAFETY: intentional mutation attempts against this owned fixture descriptor.
    assert_eq!(unsafe { libc::ftruncate(fd.as_raw_fd(), 0) }, -1);
    // SAFETY: single live byte, no retained pointer; seals must reject the write.
    assert_eq!(
        unsafe { libc::pwrite(fd.as_raw_fd(), b"x".as_ptr().cast(), 1, 64) },
        -1
    );
    let mut receiver = Receiver::new(session, 7).unwrap();
    receiver.accept(fd).unwrap();
    let frame = receiver.latest().unwrap();
    assert_eq!(frame.pixels(), &pixels);
    assert_eq!(frame.size, size);
    assert_eq!(frame.sequence, 1);
    assert_eq!(frame.generation, 7);
}
#[test]
fn wrong_session_generation_sequence_format_stride_and_length_never_replace_valid_pixels() {
    let session = Session::new().unwrap();
    let size = Dimensions {
        width: 1,
        height: 1,
    };
    let good = bytes(frame::seal_rgba(session, 9, 1, size, &[1, 2, 3, 4]).unwrap());
    let mut receiver = Receiver::new(session, 9).unwrap();
    receiver.accept(raw(&good, ALL, None)).unwrap();
    let next = bytes(frame::seal_rgba(session, 9, 2, size, &[5, 6, 7, 8]).unwrap());
    // Every bit in every fixed/correlated/size header field is checked. Sequence
    // is variable by design, so it is exercised separately as zero/replay below.
    for offset in (0..64).filter(|v| !(32..40).contains(v)) {
        for bit in 0..8 {
            let mut corrupt = next.clone();
            corrupt[offset] ^= 1 << bit;
            assert_eq!(
                receiver
                    .accept(raw(&corrupt, ALL, None))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidData,
                "byte {offset} bit {bit}"
            );
            assert_eq!(receiver.latest().unwrap().pixels(), &[1, 2, 3, 4]);
        }
    }
    for sequence in [0u64, 1] {
        let mut corrupt = next.clone();
        corrupt[32..40].copy_from_slice(&sequence.to_le_bytes());
        assert_eq!(
            receiver
                .accept(raw(&corrupt, ALL, None))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
    for length in [63, 64, 67, 69, (64 + frame::MAX_FRAME_BYTES + 1) as u64] {
        assert_eq!(
            receiver
                .accept(raw(&next, ALL, Some(length)))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
    assert_eq!(receiver.latest().unwrap().sequence, 1);
}
#[test]
fn mutable_descriptors_and_non_regular_objects_are_refused_before_pixel_reads() {
    let session = Session::new().unwrap();
    let good = bytes(
        frame::seal_rgba(
            session,
            1,
            1,
            Dimensions {
                width: 1,
                height: 1,
            },
            &[0; 4],
        )
        .unwrap(),
    );
    let mut receiver = Receiver::new(session, 1).unwrap();
    for missing in [
        ALL,
        libc::F_SEAL_WRITE,
        libc::F_SEAL_GROW,
        libc::F_SEAL_SHRINK,
        libc::F_SEAL_SEAL,
    ] {
        assert!(receiver.accept(raw(&good, ALL & !missing, None)).is_err());
    }
    let (left, _) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(receiver.accept(left.into()).is_err());
    assert!(receiver
        .accept(File::open("/dev/null").unwrap().into())
        .is_err());
    assert!(receiver.latest().is_none());
    assert_eq!(receiver.retained_bytes(), 0);
}
#[test]
fn retention_bound_keeps_only_two_buffers_and_clear_does_not_allow_replay() {
    let session = Session::new().unwrap();
    let mut receiver = Receiver::new(session, 1).unwrap();
    let size = Dimensions {
        width: 1280,
        height: 720,
    };
    let pixels = vec![0x7f; frame::MAX_FRAME_BYTES];
    receiver
        .accept(frame::seal_rgba(session, 1, 1, size, &pixels).unwrap())
        .unwrap();
    for sequence in [2, 3, 4] {
        std::thread::sleep(Duration::from_millis(70));
        receiver
            .accept(frame::seal_rgba(session, 1, sequence, size, &pixels).unwrap())
            .unwrap();
        assert_eq!(receiver.latest().unwrap().sequence, sequence);
        assert_eq!(
            receiver.retained_bytes(),
            frame::MAX_RETAINED_FRAMES * frame::MAX_FRAME_BYTES
        );
    }
    receiver.clear();
    assert!(receiver.latest().is_none());
    assert_eq!(receiver.retained_bytes(), 0);
    assert_eq!(
        receiver
            .accept(frame::seal_rgba(session, 1, 4, size, &pixels).unwrap())
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
#[test]
fn producer_and_receiver_refuse_unbounded_or_ambiguous_profiles() {
    let session = Session::new().unwrap();
    assert!(Receiver::new(session, 0).is_err());
    for size in [
        Dimensions {
            width: 0,
            height: 1,
        },
        Dimensions {
            width: 1,
            height: 0,
        },
        Dimensions {
            width: u32::MAX,
            height: u32::MAX,
        },
        Dimensions {
            width: 1280,
            height: 1280,
        },
    ] {
        assert!(frame::seal_rgba(session, 1, 1, size, &[]).is_err());
    }
    let size = Dimensions {
        width: 1,
        height: 1,
    };
    for (generation, sequence, pixels) in [
        (0, 1, &[0u8; 4][..]),
        (1, 0, &[0u8; 4][..]),
        (1, 1, &[0u8; 3][..]),
        (1, 1, &[0u8; 5][..]),
    ] {
        assert!(frame::seal_rgba(session, generation, sequence, size, pixels).is_err());
    }
    let portrait = Dimensions {
        width: 720,
        height: 1280,
    };
    assert!(frame::seal_rgba(session, 1, 1, portrait, &vec![0; frame::MAX_FRAME_BYTES]).is_ok());
}
