//! Private nonblocking frame/receipt channel. Only one cooperative frame is in
//! flight. Kernel queue/resource containment remains a separate lab obligation.
use crate::frame;
use std::{
    io, mem,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    process::Child,
};
const MAGIC: &[u8; 8] = b"HYEIPC01";
const BYTES: usize = 24;
const FRAME: u8 = 1;
const ACCEPTED: u8 = 2;
const DROPPED: u8 = 3;
// Linux include/linux/socket.h (v6.12): SCM_PIDFD is an FD-bearing record.
// https://github.com/torvalds/linux/blob/v6.12/include/linux/socket.h
const SCM_PIDFD: i32 = 0x04;
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn unavailable(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, message)
}
/// A live direct child (or this process), not an identity restored from JSON.
/// Executable, namespace and resource-profile verification belongs to supervision.
pub struct Peer {
    pin: OwnedFd,
    pid: libc::pid_t,
    uid: libc::uid_t,
    gid: libc::gid_t,
}
impl Peer {
    fn pin(pid: libc::pid_t) -> io::Result<Self> {
        // No elevated installation/credential ambiguity is supported. Kernel
        // default SCM_CREDENTIALS carries real IDs, checked against this user.
        // SAFETY: credential getters have no pointer or ownership preconditions.
        if unsafe { libc::getuid() != libc::geteuid() || libc::getgid() != libc::getegid() } {
            return Err(invalid(
                "Preview transport requires matching real and effective credentials",
            ));
        }
        // SAFETY: fixed syscall/flags, no pointer arguments.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: pidfd_open returned a fresh descriptor.
        let pin = unsafe { OwnedFd::from_raw_fd(fd as i32) };
        let peer = Self {
            pin,
            pid,
            // SAFETY: these credential getters have no preconditions.
            uid: unsafe { libc::geteuid() },
            gid: unsafe { libc::getegid() },
        };
        peer.live()?;
        Ok(peer)
    }
    pub fn current() -> io::Result<Self> {
        // SAFETY: getpid has no preconditions.
        Self::pin(unsafe { libc::getpid() })
    }
    /// Bind a newly launched worker to its explicitly expected, still-visible
    /// direct parent. PID namespace mappings need separate profile validation.
    pub fn parent(expected: u32) -> io::Result<Self> {
        let pid = i32::try_from(expected)
            .map_err(|_| invalid("Parent PID exceeds its representation"))?;
        // SAFETY: getppid has no preconditions.
        if pid <= 0 || unsafe { libc::getppid() } != pid {
            return Err(invalid("Preview parent differs from the expected launcher"));
        }
        let peer = Self::pin(pid)?;
        // SAFETY: detect adoption/reparenting during the pin operation.
        if unsafe { libc::getppid() } != pid {
            return Err(unavailable("Preview parent changed during binding"));
        }
        peer.live()?;
        Ok(peer)
    }
    pub fn child(child: &mut Child) -> io::Result<Self> {
        let pid = i32::try_from(child.id())
            .map_err(|_| invalid("Child PID exceeds its representation"))?;
        let peer = Self::pin(pid)?;
        // An unreaped direct child retains its PID. Reject termination/reaping,
        // including external waiters; a numeric PID alone is never authority.
        if child.try_wait()?.is_some() {
            return Err(unavailable("Preview peer already exited"));
        }
        peer.live()?;
        Ok(peer)
    }
    fn live(&self) -> io::Result<()> {
        let mut poll = libc::pollfd {
            fd: self.pin.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one live pollfd output with zero timeout.
        let result = unsafe { libc::poll(&mut poll, 1, 0) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        if result != 0 || poll.revents != 0 {
            return Err(unavailable("Pinned preview peer is no longer live"));
        }
        Ok(())
    }
    fn matches(&self, cred: libc::ucred) -> bool {
        cred.pid == self.pid && cred.uid == self.uid && cred.gid == self.gid
    }
}
/// An explicitly handed-off endpoint. No pathname-based connection is exposed.
/// Inherited descriptors must retain an unnamed AF_UNIX SOCK_SEQPACKET peer.
pub struct Endpoint {
    fd: OwnedFd,
}
fn option(fd: BorrowedFd<'_>, name: i32) -> io::Result<i32> {
    let mut value = 0i32;
    let mut length = mem::size_of_val(&value) as libc::socklen_t;
    // SAFETY: fixed socket option, output pointers live for this call.
    if unsafe {
        libc::getsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            name,
            (&mut value as *mut i32).cast(),
            &mut length,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    if length as usize != mem::size_of_val(&value) {
        return Err(invalid("Unexpected channel option length"));
    }
    Ok(value)
}
fn set_option(fd: BorrowedFd<'_>, name: i32, value: i32) -> io::Result<()> {
    // SAFETY: scalar fixed socket option with correct value length.
    if unsafe {
        libc::setsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            name,
            (&value as *const i32).cast(),
            mem::size_of_val(&value) as libc::socklen_t,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn disable_optional(fd: BorrowedFd<'_>, name: i32) -> io::Result<()> {
    match set_option(fd, name, 0) {
        Ok(()) if option(fd, name)? == 0 => Ok(()),
        Ok(()) => Err(invalid("Unexpected enabled ancillary socket option")),
        Err(error) if error.raw_os_error() == Some(libc::ENOPROTOOPT) => Ok(()),
        Err(error) => Err(error),
    }
}
impl Endpoint {
    pub fn pair() -> io::Result<(Self, Self)> {
        let mut fds = [-1; 2];
        // SAFETY: two-descriptor output, fixed local unnamed socket type/flags.
        if unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
                fds.as_mut_ptr(),
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful socketpair returned two fresh owned descriptors.
        let (left, right) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        Ok((Self::inherited(left)?, Self::inherited(right)?))
    }
    pub fn inherited(fd: OwnedFd) -> io::Result<Self> {
        if option(fd.as_fd(), libc::SO_TYPE)? != libc::SOCK_SEQPACKET
            || option(fd.as_fd(), libc::SO_DOMAIN)? != libc::AF_UNIX
        {
            return Err(invalid(
                "Preview channel must be a local sequenced-packet socket",
            ));
        }
        for peer in [false, true] {
            // SAFETY: zeroed sockaddr_un is a valid output buffer.
            let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
            let mut length = mem::size_of_val(&address) as libc::socklen_t;
            // SAFETY: fixed output buffer and descriptor; no paths are opened.
            let result = unsafe {
                if peer {
                    libc::getpeername(
                        fd.as_raw_fd(),
                        (&mut address as *mut libc::sockaddr_un).cast(),
                        &mut length,
                    )
                } else {
                    libc::getsockname(
                        fd.as_raw_fd(),
                        (&mut address as *mut libc::sockaddr_un).cast(),
                        &mut length,
                    )
                }
            };
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            if length as usize != mem::size_of::<libc::sa_family_t>()
                || address.sun_family as i32 != libc::AF_UNIX
            {
                return Err(invalid("Preview channel endpoints must be unnamed"));
            }
        }
        // A small kernel queue supplements, but does not replace, the one-credit
        // protocol and the required resource domain. No hard FD-count claim.
        set_option(fd.as_fd(), libc::SO_SNDBUF, 4096)?;
        // Inherited sockets may have enabled unsolicited ancillary records.
        // Disable them; the parser also closes unexpected known PIDFD records.
        disable_optional(fd.as_fd(), libc::SO_PASSPIDFD)?;
        disable_optional(fd.as_fd(), libc::SO_PASSSEC)?;
        set_option(fd.as_fd(), libc::SO_PASSCRED, 1)?;
        if option(fd.as_fd(), libc::SO_SNDBUF)? > 8192
            || option(fd.as_fd(), libc::SO_PASSCRED)? != 1
        {
            return Err(invalid(
                "Kernel channel options do not match the supported profile",
            ));
        }
        // SAFETY: fcntl gets/sets scalar flags on this live owned socket.
        let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }
    pub fn into_descriptor(self) -> OwnedFd {
        self.fd
    }
    pub fn sender(self, peer: Peer) -> io::Result<Sender> {
        peer.live()?;
        Ok(Sender {
            channel: Bound {
                fd: Some(self.fd),
                peer,
            },
            serial: 0,
            pending: None,
        })
    }
    pub fn receiver(self, peer: Peer, frames: frame::Receiver) -> io::Result<Receiver> {
        peer.live()?;
        Ok(Receiver {
            channel: Bound {
                fd: Some(self.fd),
                peer,
            },
            frames,
            serial: 0,
            pending: None,
        })
    }
}
impl AsFd for Endpoint {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
struct Bound {
    fd: Option<OwnedFd>,
    peer: Peer,
}
impl Bound {
    fn descriptor(&self) -> io::Result<BorrowedFd<'_>> {
        self.peer.live()?;
        self.fd
            .as_ref()
            .map(AsFd::as_fd)
            .ok_or_else(|| unavailable("Preview channel is closed"))
    }
    fn fail(&mut self) {
        if let Some(fd) = self.fd.take() {
            // SAFETY: shuts down only this owned private socket endpoint.
            unsafe {
                libc::shutdown(fd.as_raw_fd(), libc::SHUT_RDWR);
            }
        }
    }
    fn send(&mut self, packet: Packet, descriptor: Option<BorrowedFd<'_>>) -> io::Result<()> {
        let result = self
            .descriptor()
            .and_then(|fd| send(fd, packet, descriptor));
        if result.as_ref().is_err_and(|e| {
            !matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            )
        }) {
            self.fail();
        }
        result
    }
    fn receive(&mut self) -> io::Result<(Packet, Option<OwnedFd>)> {
        let result = (|| {
            let message = receive(self.descriptor()?)?;
            if !self.peer.matches(message.credential) {
                return Err(invalid(
                    "Preview message came from an unexpected process or credentials",
                ));
            }
            self.peer.live()?;
            Ok((message.packet, message.descriptor))
        })();
        if result.as_ref().is_err_and(|e| {
            !matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            )
        }) {
            self.fail();
        }
        result
    }
}
impl Drop for Bound {
    fn drop(&mut self) {
        self.fail();
    }
}
#[derive(Clone, Copy)]
struct Packet {
    kind: u8,
    serial: u64,
}
impl Packet {
    fn bytes(self) -> [u8; BYTES] {
        let mut bytes = [0; BYTES];
        bytes[..8].copy_from_slice(MAGIC);
        bytes[8] = self.kind;
        bytes[16..].copy_from_slice(&self.serial.to_le_bytes());
        bytes
    }
    fn parse(bytes: &[u8; BYTES]) -> io::Result<Self> {
        let packet = Self {
            kind: bytes[8],
            serial: u64::from_le_bytes(bytes[16..].try_into().expect("Fixed slice")),
        };
        if packet.serial == 0
            || ![FRAME, ACCEPTED, DROPPED].contains(&packet.kind)
            || packet.bytes() != *bytes
        {
            return Err(invalid("Invalid preview packet header"));
        }
        Ok(packet)
    }
}
fn send(fd: BorrowedFd<'_>, packet: Packet, descriptor: Option<BorrowedFd<'_>>) -> io::Result<()> {
    let mut bytes = packet.bytes();
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    // usize storage supplies cmsghdr alignment; all ancillary padding is zeroed.
    let mut control = [0usize; 4];
    // SAFETY: zeroed msghdr with explicitly initialized live pointer fields below.
    let mut message: libc::msghdr = unsafe { mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    if let Some(descriptor) = descriptor {
        message.msg_control = control.as_mut_ptr().cast();
        // SAFETY: one FD fits the aligned control buffer on supported Linux ABIs.
        unsafe {
            message.msg_controllen = libc::CMSG_SPACE(mem::size_of::<i32>() as u32) as usize;
            let header = libc::CMSG_FIRSTHDR(&message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(mem::size_of::<i32>() as u32) as usize;
            libc::CMSG_DATA(header)
                .cast::<i32>()
                .write_unaligned(descriptor.as_raw_fd());
        }
    }
    // SAFETY: pointers above remain live; atomic nonblocking seqpacket send.
    let count = unsafe {
        libc::sendmsg(
            fd.as_raw_fd(),
            &message,
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        )
    };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    if count as usize != BYTES {
        return Err(invalid("Partial preview packet send"));
    }
    Ok(())
}
struct Message {
    packet: Packet,
    credential: libc::ucred,
    descriptor: Option<OwnedFd>,
}
fn receive(fd: BorrowedFd<'_>) -> io::Result<Message> {
    let mut bytes = [0u8; BYTES];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut control = [0usize; 16];
    // SAFETY: zeroed msghdr with bounded live buffers assigned immediately below.
    let mut message: libc::msghdr = unsafe { mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    // SAFETY: both requested ancillary records fit fixed aligned storage.
    message.msg_controllen = unsafe {
        libc::CMSG_SPACE((2 * mem::size_of::<i32>()) as u32)
            + libc::CMSG_SPACE(mem::size_of::<libc::ucred>() as u32)
    } as usize;
    // SAFETY: initialized bounded message; received FDs are CLOEXEC atomically.
    let count = unsafe {
        libc::recvmsg(
            fd.as_raw_fd(),
            &mut message,
            libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
        )
    };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut descriptors = Vec::new();
    let mut credential = None;
    let mut bad = false;
    // Kernel-generated ancillary records are within the supplied fixed buffer.
    // Collect *all* returned rights before any refusal so RAII closes them.
    // SAFETY: CMSG traversal uses kernel-populated lengths in this live message.
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            let base = libc::CMSG_LEN(0) as usize;
            let length = (*header).cmsg_len.saturating_sub(base);
            if (*header).cmsg_level == libc::SOL_SOCKET
                && [libc::SCM_RIGHTS, SCM_PIDFD].contains(&(*header).cmsg_type)
            {
                bad |= (*header).cmsg_type != libc::SCM_RIGHTS
                    || !length.is_multiple_of(mem::size_of::<i32>());
                for index in 0..length / mem::size_of::<i32>() {
                    descriptors.push(OwnedFd::from_raw_fd(
                        libc::CMSG_DATA(header)
                            .cast::<i32>()
                            .add(index)
                            .read_unaligned(),
                    ));
                }
            } else if (*header).cmsg_level == libc::SOL_SOCKET
                && (*header).cmsg_type == libc::SCM_CREDENTIALS
                && length == mem::size_of::<libc::ucred>()
                && credential.is_none()
            {
                credential = Some(
                    libc::CMSG_DATA(header)
                        .cast::<libc::ucred>()
                        .read_unaligned(),
                );
            } else {
                bad = true;
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if count == 0 {
        return Err(unavailable("Preview peer disconnected"));
    }
    if count as usize != BYTES
        || bad
        || message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
        || descriptors.len() > 1
    {
        return Err(invalid("Invalid or truncated preview packet envelope"));
    }
    Ok(Message {
        packet: Packet::parse(&bytes)?,
        credential: credential
            .ok_or_else(|| invalid("Preview message lacks kernel sender credentials"))?,
        descriptor: descriptors.pop(),
    })
}
/// Raw pixel acceptance or rate dropping only; never compositor readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Receipt {
    Accepted,
    Dropped,
}
/// One in-flight frame. Calling send without consuming its receipt is refused.
pub struct Sender {
    channel: Bound,
    serial: u64,
    pending: Option<u64>,
}
impl Sender {
    /// Poll integration only; readiness/credentials are checked again on IO.
    pub fn poll_descriptor(&self) -> io::Result<BorrowedFd<'_>> {
        self.channel.descriptor()
    }
    /// Observe the pinned process independently of socket EOF/duplicate holders.
    pub fn peer_process_descriptor(&self) -> BorrowedFd<'_> {
        self.channel.peer.pin.as_fd()
    }
    pub fn stop(&mut self) {
        self.channel.fail();
        self.pending = None;
    }
    pub fn can_send(&self) -> bool {
        self.pending.is_none() && self.channel.descriptor().is_ok()
    }
    pub fn try_send(&mut self, frame: OwnedFd) -> io::Result<()> {
        if let Err(error) = self.channel.peer.live() {
            self.channel.fail();
            return Err(error);
        }
        if self.channel.fd.is_none() {
            return Err(unavailable("Preview channel is closed"));
        }
        if !self.can_send() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "A preview receipt is still pending",
            ));
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.stop();
            return Err(invalid("Preview packet sequence exhausted"));
        };
        self.channel.send(
            Packet {
                kind: FRAME,
                serial,
            },
            Some(frame.as_fd()),
        )?;
        self.serial = serial;
        self.pending = Some(serial);
        Ok(())
    }
    pub fn try_receipt(&mut self) -> io::Result<Receipt> {
        let (packet, descriptor) = self.channel.receive()?;
        if descriptor.is_some()
            || self.pending != Some(packet.serial)
            || ![ACCEPTED, DROPPED].contains(&packet.kind)
        {
            self.channel.fail();
            return Err(invalid("Unexpected preview receipt or attached descriptor"));
        }
        self.pending = None;
        Ok(if packet.kind == ACCEPTED {
            Receipt::Accepted
        } else {
            Receipt::Dropped
        })
    }
}
/// Processes one frame per call. A pending receipt must flush before another read.
pub struct Receiver {
    channel: Bound,
    frames: frame::Receiver,
    serial: u64,
    pending: Option<Packet>,
}
impl Receiver {
    pub fn latest(&self) -> Option<&frame::Frame> {
        self.frames.latest()
    }
    pub fn retained_bytes(&self) -> usize {
        self.frames.retained_bytes()
    }
    /// Explicit GUI/session teardown releases buffers and shuts down the peer.
    pub fn stop(&mut self) {
        self.frames.clear();
        self.pending = None;
        self.channel.fail();
    }
    /// Observe the pinned process independently of socket EOF/duplicate holders.
    pub fn peer_process_descriptor(&self) -> BorrowedFd<'_> {
        self.channel.peer.pin.as_fd()
    }
    /// Poll integration only; readiness/credentials are checked again on IO.
    pub fn poll_descriptor(&self) -> io::Result<BorrowedFd<'_>> {
        self.channel.descriptor()
    }
    pub fn flush_receipt(&mut self) -> io::Result<()> {
        let result = self.flush_one();
        if result.as_ref().is_err_and(|e| {
            !matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            )
        }) {
            self.stop();
        }
        result
    }
    fn flush_one(&mut self) -> io::Result<()> {
        if let Some(packet) = self.pending {
            self.channel.send(packet, None)?;
            self.pending = None;
        }
        Ok(())
    }
    pub fn try_receive(&mut self) -> io::Result<Receipt> {
        let result = self.receive_one();
        if result.as_ref().is_err_and(|e| {
            !matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            )
        }) {
            self.stop();
        }
        result
    }
    fn receive_one(&mut self) -> io::Result<Receipt> {
        self.flush_receipt()?;
        let (packet, descriptor) = self.channel.receive()?;
        if packet.kind != FRAME
            || self.serial.checked_add(1) != Some(packet.serial)
            || descriptor.is_none()
        {
            self.channel.fail();
            return Err(invalid(
                "Unexpected preview frame packet or missing descriptor",
            ));
        }
        let result = self.frames.accept(descriptor.expect("Checked descriptor"));
        let receipt = match result {
            Ok(()) => Receipt::Accepted,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Receipt::Dropped,
            Err(error) => {
                self.channel.fail();
                return Err(error);
            }
        };
        self.serial = packet.serial;
        self.pending = Some(Packet {
            kind: if receipt == Receipt::Accepted {
                ACCEPTED
            } else {
                DROPPED
            },
            serial: packet.serial,
        });
        match self.flush_receipt() {
            Ok(()) => (),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests;
