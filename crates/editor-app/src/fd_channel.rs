//! One bounded message and at most one O_PATH capability per private Unix stream direction.
//! The descriptor travels with a marker byte; the remaining bytes are bounded JSON, never code.
use std::{
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::net::UnixStream,
    },
};
#[derive(Debug)]
pub(super) struct Message {
    pub bytes: Vec<u8>,
    pub descriptor: Option<File>,
}
pub(super) fn send(
    stream: &mut UnixStream,
    bytes: &[u8],
    descriptor: Option<&File>,
) -> Result<(), String> {
    let mut marker = [0xa5_u8];
    let mut iov = libc::iovec {
        iov_base: marker.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let size = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) } as usize;
    // usize storage supplies alignment for cmsghdr; zeroing includes ancillary padding.
    let mut control = vec![0_usize; size.div_ceil(std::mem::size_of::<usize>())];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    if let Some(file) = descriptor {
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = size;
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&msg);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as usize;
            libc::CMSG_DATA(header)
                .cast::<i32>()
                .write_unaligned(file.as_raw_fd());
        }
    }
    loop {
        let n = unsafe { libc::sendmsg(stream.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) };
        if n == 1 {
            break;
        }
        let e = std::io::Error::last_os_error();
        if n < 0 && e.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(format!("Cannot send capture capability: {e}"));
    }
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let count = unsafe {
            libc::write(
                stream.as_raw_fd(),
                remaining.as_ptr().cast(),
                remaining.len(),
            )
        };
        if count > 0 {
            remaining = &remaining[count as usize..];
            continue;
        }
        let error = std::io::Error::last_os_error();
        if count < 0 && error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(format!("Cannot send capture data: {error}"));
    }
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())
}
pub(super) fn receive(stream: &mut UnixStream, maximum: usize) -> Result<Message, String> {
    let mut marker = [0_u8];
    let mut iov = libc::iovec {
        iov_base: marker.as_mut_ptr().cast(),
        iov_len: 1,
    };
    // Two FDs worth of capacity lets us close and reject an extra descriptor, not leak it.
    let size = unsafe { libc::CMSG_SPACE((2 * std::mem::size_of::<i32>()) as u32) } as usize;
    let mut control = vec![0_usize; size.div_ceil(std::mem::size_of::<usize>())];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = size;
    let count = loop {
        let count = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut msg, libc::MSG_CMSG_CLOEXEC) };
        if count >= 0 {
            break count;
        }
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::Interrupted {
            return Err(e.to_string());
        }
    };
    let mut descriptors = Vec::new();
    let mut invalid = false;
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&msg);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let base = libc::CMSG_LEN(0) as usize;
                let bytes = (*header).cmsg_len.saturating_sub(base);
                invalid |= !bytes.is_multiple_of(std::mem::size_of::<i32>());
                for index in 0..bytes / std::mem::size_of::<i32>() {
                    let fd = libc::CMSG_DATA(header)
                        .cast::<i32>()
                        .add(index)
                        .read_unaligned();
                    descriptors.push(File::from_raw_fd(fd));
                }
            } else {
                invalid = true;
            }
            header = libc::CMSG_NXTHDR(&msg, header);
        }
    }
    if count != 1
        || marker != [0xa5]
        || invalid
        || msg.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0
        || descriptors.len() > 1
    {
        return Err("Invalid or truncated capture capability envelope".into());
    }
    let mut bytes = Vec::new();
    stream
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > maximum {
        return Err("Capture capability message exceeds its byte budget".into());
    }
    Ok(Message {
        bytes,
        descriptor: descriptors.pop(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_descriptor_round_trip_is_cloexec_and_bytes_are_bounded() {
        let (mut a, mut b) = UnixStream::pair().unwrap();
        let directory = std::fs::OpenOptions::new().read(true).open("/tmp").unwrap();
        let expected = directory.metadata().unwrap();
        send(&mut a, b"{}", Some(&directory)).unwrap();
        let result = receive(&mut b, 2).unwrap();
        assert_eq!(result.bytes, b"{}");
        let received = result.descriptor.unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_eq!(received.metadata().unwrap().ino(), expected.ino());
        assert_eq!(received.metadata().unwrap().dev(), expected.dev());
        assert_ne!(
            unsafe { libc::fcntl(received.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
        let (mut a, mut b) = UnixStream::pair().unwrap();
        send(&mut a, b"long", None).unwrap();
        assert!(receive(&mut b, 3).is_err());
    }
    #[test]
    fn malformed_marker_multiple_and_truncated_descriptors_are_refused() {
        use std::io::Write;
        let (mut a, mut b) = UnixStream::pair().unwrap();
        a.write_all(b"wrong").unwrap();
        a.shutdown(std::net::Shutdown::Write).unwrap();
        assert!(receive(&mut b, 16).unwrap_err().contains("envelope"));
        for count in [2, 4] {
            let file = File::open("/tmp").unwrap();
            let (a, mut b) = UnixStream::pair().unwrap();
            let mut marker = [0xa5_u8];
            let mut iov = libc::iovec {
                iov_base: marker.as_mut_ptr().cast(),
                iov_len: 1,
            };
            let size =
                unsafe { libc::CMSG_SPACE((count * std::mem::size_of::<i32>()) as u32) } as usize;
            let mut control = vec![0_usize; size.div_ceil(std::mem::size_of::<usize>())];
            let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = control.as_mut_ptr().cast();
            msg.msg_controllen = size;
            unsafe {
                let header = libc::CMSG_FIRSTHDR(&msg);
                (*header).cmsg_level = libc::SOL_SOCKET;
                (*header).cmsg_type = libc::SCM_RIGHTS;
                (*header).cmsg_len =
                    libc::CMSG_LEN((count * std::mem::size_of::<i32>()) as u32) as usize;
                for index in 0..count {
                    libc::CMSG_DATA(header)
                        .cast::<i32>()
                        .add(index)
                        .write_unaligned(file.as_raw_fd());
                }
                assert_eq!(libc::sendmsg(a.as_raw_fd(), &msg, libc::MSG_NOSIGNAL), 1);
            }
            a.shutdown(std::net::Shutdown::Write).unwrap();
            assert!(receive(&mut b, 16).unwrap_err().contains("envelope"));
        }
    }
}
