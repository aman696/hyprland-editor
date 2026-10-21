//! Dedicated cgroup v2 resource domains obtained from an explicit live directory FD.
//! Never enables parent controllers, changes a shared limit or moves a named PID.
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::{self, Read},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    time::{Duration, Instant},
};
const CGROUP2_MAGIC: libc::c_long = 0x6367_7270;
const MAX_FILE_BYTES: u64 = 4096;
const REQUIRED: [&str; 3] = ["cpu", "memory", "pids"];
pub mod launch;

#[derive(Debug)]
pub enum Error {
    Unavailable(&'static str),
    UnresolvedCreation {
        name: String,
        error: Box<Error>,
    },
    PreparationCleanup {
        name: String,
        preparation: Box<Error>,
        cleanup: Box<Error>,
    },
    Io {
        operation: &'static str,
        error: io::Error,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => out.write_str(reason),
            Self::UnresolvedCreation {name,error} => write!(out,"Resource creation {name} could not be identified: {error}; manual/provider cleanup may be required"),
            Self::PreparationCleanup {name,preparation,cleanup} => write!(out,"Resource preparation failed: {preparation}; cleanup of {name} also failed: {cleanup}; provider cleanup may be required"),
            Self::Io { operation, error } => write!(out, "{operation}: {error}"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::UnresolvedCreation { error, .. } => Some(error.as_ref()),
            Self::PreparationCleanup { preparation, .. } => Some(preparation.as_ref()),
            Self::Unavailable(_) => None,
        }
    }
}
type Result<T> = std::result::Result<T, Error>;
fn io_error(operation: &'static str) -> Error {
    Error::Io {
        operation,
        error: io::Error::last_os_error(),
    }
}
fn dup(fd: BorrowedFd<'_>) -> Result<OwnedFd> {
    // SAFETY: duplicating the live borrowed descriptor creates a new owned reference.
    let raw = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
    if raw < 0 {
        return Err(io_error("Duplicate delegation descriptor"));
    }
    // SAFETY: successful fcntl returned a fresh descriptor owned by this function.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
fn stat(fd: BorrowedFd<'_>) -> Result<libc::stat> {
    let mut value = std::mem::MaybeUninit::uninit();
    // SAFETY: fstat initializes this output on success and retains no pointer.
    if unsafe { libc::fstat(fd.as_raw_fd(), value.as_mut_ptr()) } < 0 {
        return Err(io_error("Read resource descriptor identity"));
    }
    // SAFETY: successful fstat initialized the complete structure.
    Ok(unsafe { value.assume_init() })
}
fn validate_root(fd: BorrowedFd<'_>) -> Result<()> {
    let info = stat(fd)?;
    if info.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(Error::Unavailable(
            "Delegation must be a directory descriptor",
        ));
    }
    let mut fs = std::mem::MaybeUninit::uninit();
    // SAFETY: fstatfs initializes this output on success and retains no pointer.
    if unsafe { libc::fstatfs(fd.as_raw_fd(), fs.as_mut_ptr()) } < 0 {
        return Err(io_error("Read delegation filesystem type"));
    }
    // SAFETY: successful fstatfs initialized the complete structure.
    if unsafe { fs.assume_init() }.f_type != CGROUP2_MAGIC {
        return Err(Error::Unavailable("Delegation must belong to cgroup v2"));
    }
    // SAFETY: geteuid has no pointer or ownership precondition.
    if info.st_uid != unsafe { libc::geteuid() } || info.st_mode & 0o022 != 0 {
        return Err(Error::Unavailable(
            "Delegation must be owned by this user and not writable by other users",
        ));
    }
    Ok(())
}
fn open(fd: BorrowedFd<'_>, name: &CStr, flags: i32) -> Result<OwnedFd> {
    // Names are fixed control files or an internally generated single component.
    // SAFETY: fd/name stay live; openat follows no symlink and inherits no authority.
    let raw = unsafe {
        libc::openat(
            fd.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if raw < 0 {
        return Err(io_error("Open dedicated resource control"));
    }
    // SAFETY: successful openat returned a fresh descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
fn read(fd: BorrowedFd<'_>, name: &CStr) -> Result<String> {
    let file = File::from(open(fd, name, libc::O_RDONLY)?);
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Io {
            operation: "Read resource control",
            error,
        })?;
    if bytes.len() > MAX_FILE_BYTES as usize {
        return Err(Error::Unavailable(
            "Resource control reply exceeds its bound",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| Error::Unavailable("Resource control reply is not ASCII text"))
}
fn words(text: &str) -> Result<Vec<String>> {
    text.split_ascii_whitespace()
        .map(|v| {
            if v.is_empty()
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit())
            {
                return Err(Error::Unavailable("Malformed resource controller list"));
            }
            Ok(v.into())
        })
        .collect()
}
#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub domain_type: String,
    pub available: Vec<String>,
    pub enabled_for_children: Vec<String>,
    pub missing: Vec<String>,
    /// These are read-only observations, not resource or lab readiness authority.
    pub live_limits_verified: bool,
    pub compositor_verified: bool,
}
impl Observation {
    fn at(fd: BorrowedFd<'_>) -> Result<Self> {
        validate_root(fd)?;
        let domain_type = read(fd, c"cgroup.type")?.trim().to_string();
        let available = words(&read(fd, c"cgroup.controllers")?)?;
        let enabled_for_children = words(&read(fd, c"cgroup.subtree_control")?)?;
        let missing = REQUIRED
            .iter()
            .filter(|name| {
                !available.iter().any(|v| v == **name)
                    || !enabled_for_children.iter().any(|v| v == **name)
            })
            .map(|v| v.to_string())
            .collect();
        Ok(Self {
            domain_type,
            available,
            enabled_for_children,
            missing,
            live_limits_verified: false,
            compositor_verified: false,
        })
    }
}
/// An explicit, user-owned live delegation FD. It cannot be deserialized or
/// recreated from a stored observation. Construction does not write anything.
#[derive(Debug)]
pub struct Delegation {
    fd: OwnedFd,
}
impl Delegation {
    pub fn from_directory(directory: BorrowedFd<'_>) -> Result<Self> {
        let fd = dup(directory)?;
        validate_root(fd.as_fd())?;
        Ok(Self { fd })
    }
    pub fn observe(&self) -> Result<Observation> {
        Observation::at(self.fd.as_fd())
    }
    /// Creates only a fresh private leaf after verifying parent delegation.
    /// It never turns on a shared parent's controllers or changes its limits.
    pub fn create(&self, limits: Limits) -> Result<Domain> {
        limits.validate()?;
        let report = self.observe()?;
        if report.domain_type != "domain" || !report.missing.is_empty() {
            return Err(Error::Unavailable("CPU, memory and process controllers must already be delegated to children of a normal domain"));
        }
        // Do not create underneath a populated shared resource node.
        if !read(self.fd.as_fd(), c"cgroup.procs")?.trim().is_empty() {
            return Err(Error::Unavailable(
                "Delegation root must not contain live processes",
            ));
        }
        let parent = dup(self.fd.as_fd())?;
        let mut directory = None;
        for _ in 0..4 {
            let mut random = [0u8; 16];
            // SAFETY: getrandom writes only within this fixed live buffer.
            let count = unsafe { libc::getrandom(random.as_mut_ptr().cast(), random.len(), 0) };
            if count < 0 {
                return Err(io_error("Generate private resource identity"));
            }
            if count != random.len() as isize {
                return Err(Error::Unavailable("Incomplete private resource identity"));
            }
            let name = CString::new(format!(
                "hye-lab-{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ))
            .expect("Fixed hex name");
            // SAFETY: one generated name relative to the held parent; no path traversal.
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } == 0 {
                directory = Some(name);
                break;
            }
            if io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err(io_error("Create dedicated resource domain"));
            }
        }
        let name = directory.ok_or(Error::Unavailable(
            "Could not allocate a unique resource domain",
        ))?;
        let fd = match open(parent.as_fd(), &name, libc::O_RDONLY | libc::O_DIRECTORY) {
            Ok(fd) => fd,
            Err(error) => {
                // No opened identity exists. Never remove a substituted pathname
                // on this error path or pretend that cleanup was verified.
                return Err(Error::UnresolvedCreation {
                    name: name.to_string_lossy().into_owned(),
                    error: Box::new(error),
                });
            }
        };

        let domain = Domain {
            parent,
            name,
            fd,
            kill: None,
            join: None,
            removed: false,
            limits,
        };
        domain.prepare(limits)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub memory_bytes: u64,
    pub pids: u32,
    pub cpu_quota_us: u32,
    pub cpu_period_us: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            memory_bytes: 512 * 1024 * 1024,
            pids: 64,
            cpu_quota_us: 100_000,
            cpu_period_us: 100_000,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<()> {
        if !self.memory_bytes.is_multiple_of(1024 * 1024)
            || !(16 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&self.memory_bytes)
            || !(1..=256).contains(&self.pids)
            || !(1_000..=1_000_000).contains(&self.cpu_period_us)
            || self.cpu_quota_us < 1_000
            || self.cpu_quota_us > self.cpu_period_us
        {
            return Err(Error::Unavailable(
                "Preview resource limits exceed the supported bounded profile",
            ));
        }
        Ok(())
    }
}
/// A newly created, configured leaf. This is an owned OS capability, not a
/// serialized readiness claim. There is intentionally no API to attach a PID.
#[derive(Debug)]
pub struct Domain {
    parent: OwnedFd,
    name: CString,
    fd: OwnedFd,
    kill: Option<OwnedFd>,
    join: Option<OwnedFd>,
    removed: bool,
    limits: Limits,
}
fn write_value(fd: BorrowedFd<'_>, value: &[u8]) -> Result<()> {
    // cgroup control writes are single kernel transactions. Never retry a suffix
    // as a new command or accept a partially written value.
    // SAFETY: buffer and fd are live; write retains no pointer.
    let count = unsafe { libc::write(fd.as_raw_fd(), value.as_ptr().cast(), value.len()) };
    if count < 0 {
        return Err(io_error("Write dedicated resource control"));
    }
    if count != value.len() as isize {
        return Err(Error::Io {
            operation: "Write dedicated resource control",
            error: io::Error::new(
                io::ErrorKind::WriteZero,
                "Partial resource command was refused",
            ),
        });
    }
    Ok(())
}
impl Domain {
    fn verify_launch_member(&self, pid: libc::pid_t) -> Result<()> {
        validate_root(self.fd.as_fd())?;
        let members = read(self.fd.as_fd(), c"cgroup.procs")?;
        if members.trim() != pid.to_string() {
            return Err(Error::Unavailable("Launch resource membership changed"));
        }
        for (name, expected) in [
            (c"memory.max", self.limits.memory_bytes.to_string()),
            (c"memory.swap.max", "0".into()),
            (c"memory.oom.group", "1".into()),
            (c"pids.max", self.limits.pids.to_string()),
            (
                c"cpu.max",
                format!("{} {}", self.limits.cpu_quota_us, self.limits.cpu_period_us),
            ),
        ] {
            if read(self.fd.as_fd(), name)?.trim() != expected {
                return Err(Error::Unavailable("Launch resource limits changed"));
            }
        }
        Ok(())
    }
    fn prepare(mut self, limits: Limits) -> Result<Self> {
        if let Err(preparation) = self.configure(limits) {
            let name = self.name.to_string_lossy().into_owned();
            return match self.cleanup(Duration::from_secs(1)) {
                Ok(()) => Err(preparation),
                Err(cleanup) => Err(Error::PreparationCleanup {
                    name,
                    preparation: Box::new(preparation),
                    cleanup: Box::new(cleanup),
                }),
            };
        }
        Ok(self)
    }
    fn configure(&mut self, limits: Limits) -> Result<()> {
        validate_root(self.fd.as_fd())?;
        if !read(self.fd.as_fd(), c"cgroup.procs")?.trim().is_empty() {
            return Err(Error::Unavailable(
                "New resource domain is unexpectedly populated",
            ));
        }
        self.kill = Some(open(self.fd.as_fd(), c"cgroup.kill", libc::O_WRONLY)?);
        self.join = Some(open(self.fd.as_fd(), c"cgroup.procs", libc::O_WRONLY)?);
        // This is a fresh, already-empty leaf. Do not write cgroup.kill as a
        // capability probe: affected kernels snapshot the source kill_seq for
        // CLONE_INTO_CGROUP and spuriously kill children of a previously killed
        // target (upstream fix 8e359920216689b3b79e0fe8961a77fe312a511f).
        // Real stop/error cleanup still writes the held kill control. Domains
        // are never reused after cleanup; placement/readback must still pass.
        if self.populated()? {
            return Err(Error::Unavailable(
                "Fresh resource subtree is unexpectedly populated",
            ));
        }
        for (name, value) in [
            (c"memory.max", limits.memory_bytes.to_string()),
            (c"memory.swap.max", "0".into()),
            (c"memory.oom.group", "1".into()),
            (c"pids.max", limits.pids.to_string()),
            (
                c"cpu.max",
                format!("{} {}", limits.cpu_quota_us, limits.cpu_period_us),
            ),
        ] {
            write_value(
                open(self.fd.as_fd(), name, libc::O_WRONLY)?.as_fd(),
                value.as_bytes(),
            )?;
            if read(self.fd.as_fd(), name)?.trim() != value {
                return Err(Error::Unavailable(
                    "Kernel resource limit readback does not match the prepared profile",
                ));
            }
        }
        // Opening the future join FD is not proof that migration will succeed.
        // The launch module uses clone3 creation into this held leaf, then
        // verifies stopped membership/limits before private gate release.
        Ok(())
    }
    fn populated(&self) -> Result<bool> {
        let events = read(self.fd.as_fd(), c"cgroup.events")?;
        let mut value = None;
        for row in events.lines() {
            let fields = row.split_ascii_whitespace().collect::<Vec<_>>();
            if fields.first() == Some(&"populated") {
                if value.is_some() || fields.len() != 2 {
                    return Err(Error::Unavailable("Ambiguous resource population event"));
                }
                value = Some(match fields[1] {
                    "0" => false,
                    "1" => true,
                    _ => return Err(Error::Unavailable("Invalid resource population event")),
                });
            }
        }
        value.ok_or(Error::Unavailable("Missing resource population event"))
    }
    fn remove(&mut self) -> Result<()> {
        if self.removed {
            return Ok(());
        }
        let current = open(
            self.parent.as_fd(),
            &self.name,
            libc::O_RDONLY | libc::O_DIRECTORY,
        )?;
        let info = stat(current.as_fd())?;
        let owned = stat(self.fd.as_fd())?;
        if info.st_dev != owned.st_dev || info.st_ino != owned.st_ino {
            return Err(Error::Unavailable(
                "Resource directory identity changed; refusing to remove it",
            ));
        }
        // SAFETY: matching owned directory at one generated name, nonrecursive removal.
        if unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } < 0
        {
            return Err(io_error("Remove empty dedicated resource domain"));
        }
        self.removed = true;
        Ok(())
    }
    /// Kill this dedicated subtree, wait for kernel emptiness and check the
    /// currently observed directory identity before requesting removal.
    /// Timeout/error is never reported as Cleaned. Provider exclusivity and
    /// concurrent-race teardown certification remain required.
    pub fn cleanup(mut self, timeout: Duration) -> Result<()> {
        if timeout > Duration::from_secs(5) {
            return Err(Error::Unavailable(
                "Resource teardown timeout exceeds five seconds",
            ));
        }
        if let Some(kill) = &self.kill {
            write_value(kill.as_fd(), b"1")?;
        }
        let start = Instant::now();
        while self.populated()? {
            if start.elapsed() >= timeout {
                return Err(Error::Unavailable(
                    "Dedicated resource subtree did not become empty before the deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.remove()
    }
}
impl Drop for Domain {
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        // Emergency best effort only. The explicit cleanup result is needed to
        // claim teardown; Drop cannot turn a partial cleanup into that evidence.
        if let Some(kill) = &self.kill {
            let _ = write_value(kill.as_fd(), b"1");
        }
        if self.populated().is_ok_and(|value| !value) {
            let _ = self.remove();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            loop {
                let path = std::env::temp_dir().join(format!(
                    "hye-cgroup-failure-{}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match std::fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("{e}"),
                }
            }
        }
        // Test-only construction bypasses the real cgroup filesystem guard to
        // exercise POSIX identity/error logic. This is not positive cgroup proof.
        fn unconfigured_domain(&self, events: &str) -> Domain {
            std::fs::create_dir(self.0.join("owned")).unwrap();
            std::fs::write(self.0.join("owned/cgroup.events"), events).unwrap();
            let parent = File::open(&self.0).unwrap();
            let fd = File::open(self.0.join("owned")).unwrap();
            Domain {
                parent: parent.into(),
                name: CString::new("owned").unwrap(),
                fd: fd.into(),
                kill: None,
                join: None,
                removed: false,
                limits: Limits::default(),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn failed_preparation_keeps_cleanup_failure_and_resource_identity_visible() {
        let fixture = Fixture::new();
        let domain = fixture.unconfigured_domain("populated 0\n");
        let error = domain.prepare(Limits::default()).unwrap_err();
        match error {
            Error::PreparationCleanup {
                name,
                preparation,
                cleanup,
            } => {
                assert_eq!(name, "owned");
                assert!(preparation.to_string().contains("cgroup v2"));
                assert!(cleanup.to_string().contains("Remove empty dedicated"));
            }
            other => panic!("Lost preparation or cleanup evidence: {other}"),
        }
        assert!(fixture.0.join("owned/cgroup.events").exists());
    }
    #[test]
    fn substituted_directory_is_not_removed_by_the_observed_identity_guard() {
        let fixture = Fixture::new();
        let mut domain = fixture.unconfigured_domain("populated 0\nfrozen 0\n");
        std::fs::rename(fixture.0.join("owned"), fixture.0.join("captured")).unwrap();
        std::fs::create_dir(fixture.0.join("owned")).unwrap();
        std::fs::write(fixture.0.join("owned/canary"), "untouched").unwrap();
        assert!(domain
            .remove()
            .unwrap_err()
            .to_string()
            .contains("identity changed"));
        drop(domain);
        assert_eq!(
            std::fs::read_to_string(fixture.0.join("owned/canary")).unwrap(),
            "untouched"
        );
        assert!(fixture.0.join("captured/cgroup.events").exists());
    }
    #[test]
    fn missing_duplicate_and_invalid_population_never_claim_an_empty_subtree() {
        for events in [
            "frozen 0\n",
            "populated 0\npopulated 1\n",
            "populated 2\n",
            "populated\n",
        ] {
            let fixture = Fixture::new();
            let domain = fixture.unconfigured_domain(events);
            assert!(domain.populated().is_err());
            assert!(domain.cleanup(Duration::ZERO).is_err());
            assert!(fixture.0.join("owned/cgroup.events").exists());
        }
    }
    #[test]
    fn populated_or_failed_directory_removal_is_not_reported_as_cleaned() {
        for events in ["populated 1\n", "populated 0\n"] {
            let fixture = Fixture::new();
            let domain = fixture.unconfigured_domain(events);
            // A regular directory with a control fixture cannot be rmdir'ed;
            // kernel cgroup pseudo-files differ. Failure must remain a failure.
            assert!(domain.cleanup(Duration::ZERO).is_err());
            assert!(fixture.0.join("owned/cgroup.events").exists());
        }
    }
    #[test]
    fn controller_text_and_reply_sizes_are_bounded_without_clipping() {
        assert_eq!(
            words("cpu memory pids\n").unwrap(),
            vec!["cpu", "memory", "pids"]
        );
        assert!(words("cpu mémo pids").is_err());
        let fixture = Fixture::new();
        std::fs::write(
            fixture.0.join("oversized"),
            "x".repeat(MAX_FILE_BYTES as usize + 1),
        )
        .unwrap();
        let parent = File::open(&fixture.0).unwrap();
        assert!(read(parent.as_fd(), c"oversized")
            .unwrap_err()
            .to_string()
            .contains("bound"));
    }
}

#[cfg(test)]
mod kernel_tests;
