//! Gated creation of a trusted constructor, already inside its resource leaf.
//!
//! This is NOT compositor execution authority or a sandbox profile. The caller
//! must independently approve the exact constructor and every explicitly passed
//! descriptor. A sealed executable identifies bytes; it does not authenticate
//! them. No GUI/runtime launch is connected until the remaining gates exist.
use super::*;
use std::os::unix::fs::MetadataExt;

const MAX_PASSED: usize = 1024;
const MAX_ARGS: usize = 256;
const MAX_ARG_BYTES: usize = 64 * 1024;
const INTO_CGROUP: u64 = 1 << 33;
const PIDFD: u64 = 0x1000;
const SEALS: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn group_error(error: Error) -> io::Error {
    io::Error::other(error.to_string())
}
fn duplicate(fd: BorrowedFd<'_>, minimum: i32) -> io::Result<OwnedFd> {
    // SAFETY: a live descriptor and scalar duplication flags; owns a fresh FD.
    let value = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, minimum) };
    if value < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl returned a fresh descriptor owned by this function.
    Ok(unsafe { OwnedFd::from_raw_fd(value) })
}
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    // SAFETY: pipe2 fills exactly two slots, owned here after success.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful pipe2 returned two fresh owned descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}
fn gate() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    // SAFETY: creates only an unnamed private pair, with fresh ownership.
    if unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}
fn nonblocking(fd: BorrowedFd<'_>) -> io::Result<()> {
    // SAFETY: scalar flag operations on a live descriptor.
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn deadline(timeout: Duration) -> io::Result<Instant> {
    if timeout.is_zero() || timeout > Duration::from_secs(5) {
        return Err(invalid(
            "Launch deadline must be positive and at most five seconds",
        ));
    }
    Ok(Instant::now() + timeout)
}
fn unprivileged() -> io::Result<()> {
    let (mut real, mut effective, mut saved) = (0, 0, 0);
    // SAFETY: scalar credential outputs owned here, with no identity changes.
    if unsafe { libc::getresuid(&mut real, &mut effective, &mut saved) } < 0 {
        return Err(io::Error::last_os_error());
    }
    if real == 0 || real != effective || real != saved {
        return Err(invalid(
            "The constructor launcher requires an ordinary non-root identity",
        ));
    }
    if unsafe { libc::getresgid(&mut real, &mut effective, &mut saved) } < 0 {
        return Err(io::Error::last_os_error());
    }
    if real == 0 || real != effective || real != saved {
        return Err(invalid(
            "The constructor launcher refuses retained group privilege",
        ));
    }
    #[repr(C)]
    struct Header {
        version: u32,
        pid: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Caps {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }
    let header = Header {
        version: 0x20080522,
        pid: 0,
    };
    let mut caps = [Caps {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    // SAFETY: Linux capability ABI v3 has exactly these two output words.
    if unsafe { libc::syscall(libc::SYS_capget, &header, caps.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    if caps
        .iter()
        .any(|c| c.effective != 0 || c.permitted != 0 || c.inheritable != 0)
    {
        return Err(invalid(
            "The constructor launcher refuses retained capabilities",
        ));
    }
    Ok(())
}

/// Explicit live files and arguments for an independently approved constructor.
/// Environment is empty. Passed descriptors become 3, 4, ... in input order;
/// stdio is supplied explicitly. No pathname or ambient descriptor is selected.
pub struct Plan {
    arguments: Vec<CString>,
    executable: OwnedFd,
    inputs: Vec<OwnedFd>,
}
impl Plan {
    pub fn new(
        executable: BorrowedFd<'_>,
        arguments: &[CString],
        stdio: [BorrowedFd<'_>; 3],
        passed: &[BorrowedFd<'_>],
    ) -> io::Result<Self> {
        if arguments.is_empty()
            || arguments.len() > MAX_ARGS
            || arguments[0].is_empty()
            || arguments
                .iter()
                .map(|a| a.as_bytes_with_nul().len())
                .sum::<usize>()
                > MAX_ARG_BYTES
            || passed.len() > MAX_PASSED
        {
            return Err(invalid(
                "Constructor arguments/descriptor count exceed their bounds",
            ));
        }
        let entry = File::from(executable.try_clone_to_owned()?);
        let info = entry.metadata()?;
        // SAFETY: query immutable-file seals on this held descriptor.
        let seals = unsafe { libc::fcntl(entry.as_raw_fd(), libc::F_GET_SEALS) };
        if !info.is_file()
            || info.len() == 0
            || info.len() > 64 * 1024 * 1024
            || info.mode() & 0o111 == 0
            || info.mode() & 0o7000 != 0
            || seals < 0
            || seals & SEALS != SEALS
        {
            return Err(invalid(
                "Constructor must be an executable, fully sealed regular file",
            ));
        }
        let minimum = (passed.len() + 6) as i32;
        let inputs = stdio
            .into_iter()
            .chain(passed.iter().copied())
            .map(|fd| duplicate(fd, minimum))
            .collect::<io::Result<Vec<_>>>()?;
        Ok(Self {
            arguments: arguments.to_vec(),
            executable: duplicate(entry.as_fd(), minimum)?,
            inputs,
        })
    }
}

/// A direct child with an atomically allocated pidfd. Never accepts a caller PID.
#[derive(Debug)]
struct Process {
    pid: libc::pid_t,
    fd: OwnedFd,
    outcome: Option<Outcome>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub code: i32,
    pub signal: Option<i32>,
}
impl Process {
    fn signal(&self, signal: i32) -> io::Result<()> {
        // SAFETY: only the live pidfd, no raw/reusable PID is signalled.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.fd.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn event(&self, flags: i32) -> io::Result<Option<libc::siginfo_t>> {
        // A zero si_pid distinguishes WNOHANG's no-event case on all supported kernels.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: the live pidfd identifies our direct child; output is initialized.
        if unsafe {
            libc::waitid(
                libc::P_PIDFD,
                self.fd.as_raw_fd() as u32,
                &mut info,
                flags | libc::WNOHANG,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: si_pid is the waitid child-event field.
        Ok((unsafe { info.si_pid() } != 0).then_some(info))
    }
    fn try_wait(&mut self) -> io::Result<Option<Outcome>> {
        if self.outcome.is_some() {
            return Ok(self.outcome);
        }
        if let Some(info) = self.event(libc::WEXITED)? {
            // SAFETY: successful waitid initialized its child-status fields.
            let status = unsafe { info.si_status() };
            self.outcome = Some(Outcome {
                code: status,
                signal: (info.si_code != libc::CLD_EXITED).then_some(status),
            });
        }
        Ok(self.outcome)
    }
}

/// Held resources and a private release gate. Creation returns only after the
/// kernel stop and exact resource readback. Start does not mean renderer Ready.
#[derive(Debug)]
pub struct Session {
    domain: Option<Domain>,
    process: Option<Process>,
    release: Option<File>,
    errors: Option<File>,
    started: bool,
    // PR_SET_PDEATHSIG follows the creating thread. Keep this owner on its
    // dedicated supervisor thread until teardown, rather than moving it out.
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Session {
    pub fn spawn(domain: Domain, plan: Plan, timeout: Duration) -> io::Result<Self> {
        let mut session = Self {
            domain: Some(domain),
            process: None,
            release: None,
            errors: None,
            started: false,
            _thread: std::marker::PhantomData,
        };
        let result = session.create(plan, timeout);
        if let Err(error) = result {
            return Err(session.fail(error));
        }
        Ok(session)
    }
    fn create(&mut self, plan: Plan, timeout: Duration) -> io::Result<()> {
        let end = deadline(timeout)?;
        unprivileged()?;
        let (error_read, error_write) = pipe()?;
        let (release_read, release_write) = gate()?;
        nonblocking(error_read.as_fd())?;
        nonblocking(release_write.as_fd())?;
        let count = plan.inputs.len() as i32;
        let minimum = count + 3;
        // Every source is above the complete destination table, so dup3 cannot
        // overwrite a later source. The child closes everything beyond it.
        let executable = duplicate(plan.executable.as_fd(), minimum)?;
        let errors = duplicate(error_write.as_fd(), minimum)?;
        let release = duplicate(release_read.as_fd(), minimum)?;
        let argv: Vec<_> = plan
            .arguments
            .iter()
            .map(|s| s.as_ptr())
            .chain([std::ptr::null()])
            .collect();
        let mut pidfd = -1i32;
        let parent = unsafe { libc::getpid() };
        let mut args: libc::clone_args = unsafe { std::mem::zeroed() };
        args.flags = INTO_CGROUP | PIDFD;
        args.pidfd = (&mut pidfd as *mut i32) as u64;
        args.exit_signal = libc::SIGCHLD as u64;
        args.cgroup = self.domain.as_ref().unwrap().fd.as_raw_fd() as u64;
        let signals = SignalMask::block()?;
        // SAFETY: no VM/files/signal-table sharing, default copied stack, live
        // parent pidfd output and held cgroup. In the child only the fixed,
        // allocation-free syscall sequence below may run before exec/_exit.
        let pid = unsafe {
            libc::syscall(
                libc::SYS_clone3,
                &args,
                std::mem::size_of::<libc::clone_args>(),
            )
        };
        if pid == 0 {
            unsafe {
                child(
                    &plan,
                    &argv,
                    executable.as_raw_fd(),
                    errors.as_raw_fd(),
                    release.as_raw_fd(),
                    parent,
                )
            }
        }
        let error = (pid < 0).then(io::Error::last_os_error);
        drop(signals);
        if let Some(error) = error {
            return Err(error);
        }
        if pidfd < 0 {
            // Kernel ABI violation: do not manufacture an owned FD or reuse a
            // raw PID for authority. This kernel cannot meet the profile.
            return Err(io::Error::other("clone3 did not return its required pidfd"));
        }
        self.process = Some(Process {
            pid: pid as i32,
            fd: unsafe { OwnedFd::from_raw_fd(pidfd) },
            outcome: None,
        });
        self.release = Some(File::from(release_write));
        self.errors = Some(File::from(error_read));
        drop((error_write, release_read, executable, errors, release, plan));
        loop {
            let process = self.process.as_ref().unwrap();
            if let Some(info) = process.event(libc::WSTOPPED | libc::WEXITED | libc::WNOWAIT)? {
                if info.si_code != libc::CLD_STOPPED || unsafe { info.si_status() } != libc::SIGSTOP
                {
                    return Err(io::Error::other(
                        "Constructor exited before its private launch gate",
                    ));
                }
                self.domain
                    .as_ref()
                    .unwrap()
                    .verify_launch_member(process.pid)
                    .map_err(group_error)?;
                return Ok(());
            }
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Constructor did not reach its launch gate",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    /// Releases only this session's pipe and live process. Even an unsolicited
    /// SIGCONT cannot authorize execution while the private pipe remains closed.
    pub fn start(&mut self, timeout: Duration) -> io::Result<()> {
        let result = self.start_inner(timeout);
        result.map_err(|error| self.fail(error))
    }
    fn start_inner(&mut self, timeout: Duration) -> io::Result<()> {
        let end = deadline(timeout)?;
        if self.started || self.release.is_none() {
            return Err(invalid("Launch gate has already been consumed"));
        }
        let process = self
            .process
            .as_ref()
            .ok_or_else(|| invalid("Launch child is unavailable"))?;
        self.domain
            .as_ref()
            .ok_or_else(|| invalid("Resource domain is unavailable"))?
            .verify_launch_member(process.pid)
            .map_err(group_error)?;
        let release = self.release.take().unwrap();
        // MSG_NOSIGNAL keeps a dead child from sending SIGPIPE to the parent,
        // even if the embedding process uses SIGPIPE's default disposition.
        let count = unsafe {
            libc::send(
                release.as_raw_fd(),
                b"G".as_ptr().cast(),
                1,
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count != 1 {
            return Err(io::Error::other("Incomplete private release message"));
        }
        process.signal(libc::SIGCONT)?;
        let mut record = Vec::new();
        let errors = self.errors.as_mut().unwrap();
        loop {
            let mut bytes = [0; 9];
            match errors.read(&mut bytes) {
                Ok(0) if record.is_empty() => {
                    self.started = true;
                    return Ok(());
                }
                Ok(0) => {
                    return Err(io::Error::other(format!(
                        "Constructor setup/exec refused: {record:?}"
                    )))
                }
                Ok(count) => {
                    record.extend_from_slice(&bytes[..count]);
                    if record.len() > 8 {
                        return Err(io::Error::other("Invalid constructor error record"));
                    }
                }
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Constructor exec handshake exceeded its deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn try_wait(&mut self) -> io::Result<Option<Outcome>> {
        self.process
            .as_mut()
            .ok_or_else(|| invalid("Launch child is unavailable"))?
            .try_wait()
    }
    /// Explicit cleanup is required for a Cleaned claim. Failure retains the
    /// session's process pin and a resource identity/error; Drop is best effort.
    pub fn stop(&mut self, timeout: Duration) -> io::Result<()> {
        let end = deadline(timeout)?;
        self.release.take();
        if let Some(domain) = &self.domain {
            if let Some(kill) = &domain.kill {
                write_value(kill.as_fd(), b"1").map_err(group_error)?;
            }
        }
        if let Some(process) = &mut self.process {
            if process.outcome.is_none() {
                if let Err(error) = process.signal(libc::SIGKILL) {
                    if error.raw_os_error() != Some(libc::ESRCH) {
                        return Err(error);
                    }
                }
                while process.try_wait()?.is_none() {
                    if Instant::now() >= end {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "Owned launch child was not reaped before the deadline",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
        if let Some(domain) = self.domain.take() {
            domain
                .cleanup(end.saturating_duration_since(Instant::now()))
                .map_err(group_error)?;
        }
        self.errors.take();
        Ok(())
    }
    fn fail(&mut self, error: io::Error) -> io::Error {
        let identity = self
            .domain
            .as_ref()
            .map(|d| d.name.to_string_lossy().into_owned());
        match self.stop(Duration::from_secs(1)) {
            Ok(()) => error,
            Err(cleanup) => io::Error::other(format!(
                "{error}; launch cleanup for {identity:?} failed: {cleanup}"
            )),
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // Bounded emergency attempt only; no success certificate is inferred.
        let _ = self.stop(Duration::from_millis(100));
    }
}

struct SignalMask(libc::sigset_t);
impl SignalMask {
    fn block() -> io::Result<Self> {
        let mut set = unsafe { std::mem::zeroed() };
        let mut old = unsafe { std::mem::zeroed() };
        // SAFETY: these are fully owned signal-set buffers; pthread_sigmask
        // changes only this calling thread. Parent restoration is RAII-owned.
        unsafe { libc::sigfillset(&mut set) };
        let error = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &set, &mut old) };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error));
        }
        Ok(Self(old))
    }
}
impl Drop for SignalMask {
    fn drop(&mut self) {
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &self.0, std::ptr::null_mut()) };
    }
}

// This function never returns through Rust destructors/unwinding. All pointer
// arrays/FDs were prepared before clone; no allocation, formatting or locks here.
unsafe fn child(
    plan: &Plan,
    argv: &[*const libc::c_char],
    executable: i32,
    errors: i32,
    release: i32,
    parent: libc::pid_t,
) -> ! {
    unsafe fn failed(fd: i32, phase: i32) -> ! {
        let record = [phase, unsafe { *libc::__errno_location() }];
        unsafe {
            libc::write(fd, record.as_ptr().cast(), 8);
            libc::_exit(127)
        }
    }
    unsafe {
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) < 0
            || libc::getppid() != parent
        {
            failed(errors, 1);
        }
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0
            || libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) < 0
        {
            failed(errors, 2);
        }
        if libc::setpgid(0, 0) < 0 {
            failed(errors, 3);
        }
        libc::umask(0o077);
        let core = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::setrlimit(libc::RLIMIT_CORE, &core) < 0 {
            failed(errors, 4);
        }
        if libc::chdir(c"/".as_ptr()) < 0 {
            failed(errors, 13);
        }
        for (index, source) in plan.inputs.iter().enumerate() {
            if libc::dup3(source.as_raw_fd(), index as i32, 0) < 0 {
                failed(errors, 5);
            }
        }
        let count = plan.inputs.len() as i32;
        if libc::dup3(executable, count, libc::O_CLOEXEC) < 0
            || libc::dup3(errors, count + 1, libc::O_CLOEXEC) < 0
            || libc::dup3(release, count + 2, libc::O_CLOEXEC) < 0
        {
            failed(errors, 6);
        }
        let errors = count + 1;
        if libc::syscall(libc::SYS_close_range, (count + 3) as u32, u32::MAX, 0) < 0 {
            failed(errors, 7);
        }
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_DFL;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in 1..=64 {
            if signal == libc::SIGKILL || signal == libc::SIGSTOP {
                continue;
            }
            // glibc reserves two internal signals and reports EINVAL for them.
            if libc::sigaction(signal, &action, std::ptr::null_mut()) < 0
                && *libc::__errno_location() != libc::EINVAL
            {
                failed(errors, 8);
            }
        }
        let mut empty = std::mem::zeroed();
        libc::sigemptyset(&mut empty);
        if libc::sigprocmask(libc::SIG_SETMASK, &empty, std::ptr::null_mut()) < 0 {
            failed(errors, 9);
        }
        if libc::kill(libc::getpid(), libc::SIGSTOP) < 0 {
            failed(errors, 10);
        }
        let mut go = [0u8; 1];
        loop {
            let result = libc::read(count + 2, go.as_mut_ptr().cast(), 1);
            if result == 1 && go == [b'G'] {
                break;
            }
            if result < 0 && *libc::__errno_location() == libc::EINTR {
                continue;
            }
            failed(errors, 11);
        }
        libc::close(count + 2);
        let env = [std::ptr::null::<libc::c_char>()];
        libc::syscall(
            libc::SYS_execveat,
            count,
            c"".as_ptr(),
            argv.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        failed(errors, 12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sealed(bytes: &[u8], mode: u32, seals: i32) -> File {
        let raw = unsafe {
            libc::memfd_create(
                c"hye-trusted-launch-fixture".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING | libc::MFD_EXEC,
            )
        };
        assert!(raw >= 0);
        let mut file = unsafe { File::from_raw_fd(raw) };
        file.write_all(bytes).unwrap();
        assert_eq!(unsafe { libc::fchmod(raw, mode) }, 0);
        assert_eq!(unsafe { libc::fcntl(raw, libc::F_ADD_SEALS, seals) }, 0);
        file
    }
    fn arguments() -> Vec<CString> {
        vec![CString::new("trusted-fixture").unwrap()]
    }
    fn null() -> File {
        File::options()
            .read(true)
            .write(true)
            .open("/dev/null")
            .unwrap()
    }
    #[test]
    fn constructor_plan_requires_bounded_arguments_and_a_sealed_executable() {
        let null = null();
        let standard = [null.as_fd(); 3];
        assert!(Plan::new(null.as_fd(), &arguments(), standard, &[]).is_err());
        for (bytes, mode, seals) in [
            (b"entry".as_slice(), 0o500, SEALS & !libc::F_SEAL_WRITE),
            (b"entry", 0o400, SEALS),
            (b"", 0o500, SEALS),
        ] {
            let file = sealed(bytes, mode, seals);
            assert!(Plan::new(file.as_fd(), &arguments(), standard, &[]).is_err());
        }
        let file = sealed(b"inert constructor bytes; never executed", 0o500, SEALS);
        assert!(Plan::new(file.as_fd(), &[], standard, &[]).is_err());
        assert!(Plan::new(file.as_fd(), &[CString::new("").unwrap()], standard, &[]).is_err());
        assert!(Plan::new(
            file.as_fd(),
            &vec![CString::new("a").unwrap(); MAX_ARGS + 1],
            standard,
            &[]
        )
        .is_err());
        assert!(Plan::new(
            file.as_fd(),
            &[CString::new(vec![b'a'; MAX_ARG_BYTES]).unwrap()],
            standard,
            &[]
        )
        .is_err());
        assert!(Plan::new(
            file.as_fd(),
            &arguments(),
            standard,
            &vec![null.as_fd(); MAX_PASSED + 1]
        )
        .is_err());
        let plan = Plan::new(file.as_fd(), &arguments(), standard, &[null.as_fd()]).unwrap();
        assert_eq!(plan.inputs.len(), 4);
        assert!(plan.inputs.iter().all(|fd| fd.as_raw_fd() >= 7));
        assert!(
            unsafe { libc::fcntl(plan.executable.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC
                != 0
        );
    }
    #[test]
    fn closed_private_gate_reports_an_error_without_sigpipe() {
        if std::env::var_os("HYE_LAUNCH_SIGPIPE_FIXTURE").is_none() {
            use std::process::{Command, Stdio};
            let mut process = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "cgroup::launch::tests::closed_private_gate_reports_an_error_without_sigpipe",
                    "--test-threads=1",
                ])
                .env_clear()
                .env("HYE_LAUNCH_SIGPIPE_FIXTURE", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let end = Instant::now() + Duration::from_secs(3);
            loop {
                if let Some(status) = process.try_wait().unwrap() {
                    assert!(status.success());
                    return;
                }
                if Instant::now() >= end {
                    let _ = process.kill();
                    let _ = process.wait();
                    panic!("SIGPIPE fixture did not exit");
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        // Isolated trusted test process only, never the parallel parent harness.
        assert_ne!(
            unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) },
            libc::SIG_ERR
        );
        let (read, write) = gate().unwrap();
        drop(read);
        let result = unsafe {
            libc::send(
                write.as_raw_fd(),
                b"G".as_ptr().cast(),
                1,
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        };
        assert_eq!(result, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPIPE));
        assert!(deadline(Duration::ZERO).is_err());
        assert!(deadline(Duration::from_secs(6)).is_err());
        assert!(deadline(Duration::from_secs(1)).is_ok());
    }
    #[test]
    fn explicit_descriptor_lifetimes_close_with_the_plan_without_ambient_ownership() {
        let file = sealed(b"inert entry", 0o500, SEALS);
        let null = null();
        let (read, write) = pipe().unwrap();
        nonblocking(read.as_fd()).unwrap();
        let mut reader = File::from(read);
        let plan = Plan::new(
            file.as_fd(),
            &arguments(),
            [null.as_fd(); 3],
            &[write.as_fd()],
        )
        .unwrap();
        drop(write);
        report_empty(&mut reader);
        drop(plan);
        // Another parallel test can fork between duplication and Drop. Its
        // temporary CLOEXEC copy closes at exec, so prove bounded EOF rather
        // than assuming no other test is in that fork-to-exec interval.
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            match reader.read(&mut [0; 1]) {
                Ok(count) => {
                    assert_eq!(count, 0);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("{error}"),
            }
            assert!(
                Instant::now() < end,
                "Explicit writer descriptor remained live"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) } & SEALS,
            SEALS
        );
    }
    fn report_empty(report: &mut File) {
        let mut bytes = [0; 12];
        assert_eq!(
            report.read(&mut bytes).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
    fn report_started(report: &mut File, pid: i32) {
        let end = Instant::now() + Duration::from_secs(3);
        let mut record = Vec::new();
        while record.len() < 12 {
            let mut bytes = [0; 12];
            match report.read(&mut bytes) {
                Ok(0) => panic!("Trusted constructor exited without its record"),
                Ok(count) => record.extend_from_slice(&bytes[..count]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("{e}"),
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(record.len(), 12);
        let fields: Vec<_> = record
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(fields, [0x48594547, pid as u32, 1]);
    }
    #[test]
    #[ignore = "Requires a fresh explicitly delegated ephemeral CI hierarchy and trusted static executable"]
    fn gated_constructor_stays_private_until_verified_release_and_cleans_up() {
        let root = File::open(
            std::env::var_os("HYE_CGROUP_TEST_ROOT")
                .expect("Explicit private CI delegation required"),
        )
        .unwrap();
        let delegation = Delegation::from_directory(root.as_fd()).unwrap();
        let fixture = std::fs::read(
            std::env::var_os("HYE_LAUNCH_TEST_EXECUTABLE")
                .expect("Trusted static fixture required"),
        )
        .unwrap();
        assert!(fixture.len() <= 64 * 1024 * 1024);
        let executable = sealed(&fixture, 0o500, SEALS);
        let null = null();
        for mode in ["exit", "wait", "cancel", "changed-limits", "bad-exec"] {
            let domain = delegation.create(Limits::default()).unwrap();
            let name = domain.name.clone();
            let (read, write) = pipe().unwrap();
            nonblocking(read.as_fd()).unwrap();
            let mut report = File::from(read);
            // Test-only path disclosure to a trusted static probe; the probe
            // checks actual controls without inheriting a cgroup write FD.
            let path = std::path::PathBuf::from(std::env::var_os("HYE_CGROUP_TEST_ROOT").unwrap())
                .join(name.to_str().unwrap());
            let args = [
                CString::new("trusted-launch-probe").unwrap(),
                CString::new(path.as_os_str().as_encoded_bytes()).unwrap(),
                CString::new(if mode == "exit" { "exit" } else { "wait" }).unwrap(),
            ];
            let bad = sealed(b"not an executable", 0o500, SEALS);
            let entry = if mode == "bad-exec" {
                bad.as_fd()
            } else {
                executable.as_fd()
            };
            let plan = Plan::new(entry, &args, [null.as_fd(); 3], &[write.as_fd()]).unwrap();
            let mut session = Session::spawn(domain, plan, Duration::from_secs(3)).unwrap();
            let pid = session.process.as_ref().unwrap().pid;
            drop(write);
            report_empty(&mut report);
            // An early external continue cannot bypass the private release FD.
            session
                .process
                .as_ref()
                .unwrap()
                .signal(libc::SIGCONT)
                .unwrap();
            std::thread::sleep(Duration::from_millis(20));
            report_empty(&mut report);
            if mode == "changed-limits" {
                write_value(
                    open(
                        session.domain.as_ref().unwrap().fd.as_fd(),
                        c"pids.max",
                        libc::O_WRONLY,
                    )
                    .unwrap()
                    .as_fd(),
                    b"63",
                )
                .unwrap();
                assert!(session
                    .start(Duration::from_secs(3))
                    .unwrap_err()
                    .to_string()
                    .contains("limits changed"));
            } else if mode == "bad-exec" {
                assert!(session
                    .start(Duration::from_secs(3))
                    .unwrap_err()
                    .to_string()
                    .contains("setup/exec refused"));
            } else if mode != "cancel" {
                session.start(Duration::from_secs(3)).unwrap();
                report_started(&mut report, pid);
                if mode == "exit" {
                    let end = Instant::now() + Duration::from_secs(3);
                    loop {
                        if let Some(outcome) = session.try_wait().unwrap() {
                            assert_eq!(
                                outcome,
                                Outcome {
                                    code: 0,
                                    signal: None
                                }
                            );
                            break;
                        }
                        assert!(Instant::now() < end);
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            }
            session.stop(Duration::from_secs(3)).unwrap();
            assert!(session.process.as_ref().unwrap().outcome.is_some());
            assert!(!path.exists());
            let mut byte = [0; 1];
            assert_eq!(report.read(&mut byte).unwrap(), 0);
            println!("gated_launch {mode} membership_verified=true private_release=true reaped=true leaf_removed=true");
        }
    }
}
