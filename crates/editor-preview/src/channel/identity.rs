//! 64-bit pidfs identity; legacy anonymous-inode pidfds are deliberately refused.
//! Source: Linux v6.17 fs/pidfs.c, include/uapi/linux/magic.h.
use super::{invalid, Peer};
use std::{
    io, mem,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
};
const PID_FS_MAGIC: libc::c_long = 0x5049_4446;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    device: libc::dev_t,
    inode: libc::ino_t,
}
impl Identity {
    pub(super) fn read(fd: BorrowedFd<'_>) -> io::Result<Self> {
        if usize::BITS != 64 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Process identity requires a 64-bit pidfs profile",
            ));
        }
        let mut fs = mem::MaybeUninit::uninit();
        // SAFETY: fstatfs initializes this complete output on success.
        if unsafe { libc::fstatfs(fd.as_raw_fd(), fs.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful fstatfs initialized the structure.
        if unsafe { fs.assume_init() }.f_type != PID_FS_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Descriptor does not use namespace-stable pidfs identity",
            ));
        }
        // Reject directory/O_PATH and thread-scoped handles. SCM_PIDFD supplies
        // process handles; a thread handle has different lifetime semantics.
        // SAFETY: scalar flag query on this borrowed live descriptor.
        let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_ACCMODE != libc::O_RDWR || flags & (libc::O_PATH | libc::O_EXCL) != 0 {
            return Err(invalid("Expected a normal process pidfd"));
        }
        let mut info = mem::MaybeUninit::uninit();
        // SAFETY: fstat initializes the complete output on success.
        if unsafe { libc::fstat(fd.as_raw_fd(), info.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful fstat initialized the structure.
        let info = unsafe { info.assume_init() };
        if info.st_ino < 2 {
            return Err(invalid("Invalid process inode identity"));
        }
        Ok(Self {
            device: info.st_dev,
            inode: info.st_ino,
        })
    }
}
impl Peer {
    /// Explicit live process capability supplied by the trusted launcher. No
    /// numeric PID, stored report or path can substitute for this descriptor.
    pub fn from_pidfd(descriptor: BorrowedFd<'_>) -> io::Result<Self> {
        let pin = descriptor.try_clone_to_owned()?;
        let identity = Identity::read(pin.as_fd())?;
        // SAFETY: credential getters have no preconditions.
        if unsafe { libc::getuid() != libc::geteuid() || libc::getgid() != libc::getegid() } {
            return Err(invalid("Ambiguous caller credentials"));
        }
        let peer = Self {
            pin,
            pid: 0,
            uid: unsafe { libc::geteuid() },
            gid: unsafe { libc::getegid() },
            identity: Some(identity),
        };
        peer.live()?;
        Ok(peer)
    }
    pub fn current_pidfs() -> io::Result<Self> {
        let peer = Self::current()?;
        Self::from_pidfd(peer.pin.as_fd())
    }
    pub fn child_pidfs(child: &mut std::process::Child) -> io::Result<Self> {
        let peer = Self::child(child)?;
        Self::from_pidfd(peer.pin.as_fd())
    }
    /// Bootstrap capability, not serialized identity. Keep it out of untrusted
    /// compositor processes; broker signal/exec authority still needs policy.
    pub fn process_descriptor(&self) -> BorrowedFd<'_> {
        self.pin.as_fd()
    }
}
