//! Closed syscall policies for two distinct worker roles; requests never supply syscall lists.
fn statement(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}
fn jump(k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code: 0x15,
        jt: 0,
        jf: 1,
        k,
    }
}
const BASE: &[libc::c_long] = &[
    libc::SYS_read,
    libc::SYS_write,
    libc::SYS_close,
    libc::SYS_fstat,
    libc::SYS_brk,
    libc::SYS_mmap,
    libc::SYS_mprotect,
    libc::SYS_munmap,
    libc::SYS_mremap,
    libc::SYS_madvise,
    libc::SYS_futex,
    libc::SYS_clock_gettime,
    libc::SYS_getrandom,
    libc::SYS_rt_sigaction,
    libc::SYS_rt_sigprocmask,
    libc::SYS_rt_sigreturn,
    libc::SYS_sched_yield,
    libc::SYS_getpid,
    libc::SYS_gettid,
    libc::SYS_sigaltstack,
    libc::SYS_exit,
    libc::SYS_exit_group,
    libc::SYS_restart_syscall,
];
fn install(allowed: &[libc::c_long]) -> Result<(), String> {
    #[cfg(target_arch = "x86_64")]
    let arch = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    let arch = 0xc00000b7;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return Err("Worker syscall isolation is unavailable on this architecture".into());
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        // Validate the syscall ABI before the allowlist, including x32 rejection on x86-64.
        let mut filter = vec![
            statement(0x20, 4),
            libc::sock_filter {
                code: 0x15,
                jt: 1,
                jf: 0,
                k: arch,
            },
            statement(0x06, 0x80000000),
            statement(0x20, 0),
        ];
        for syscall in allowed {
            filter.push(jump(*syscall as u32));
            filter.push(statement(0x06, 0x7fff0000));
        }
        filter.push(statement(0x06, 0x00050000 | libc::EPERM as u32));
        let program = libc::sock_fprog {
            len: filter.len() as u16,
            filter: filter.as_mut_ptr(),
        };
        if unsafe { libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) } != 0 {
            return Err("Cannot install worker syscall isolation".into());
        }
    }
    Ok(())
}
pub(super) fn parser() -> Result<(), String> {
    install(BASE)
}
fn capture_syscalls() -> Vec<libc::c_long> {
    let mut allowed = BASE.to_vec();
    allowed.extend([
        libc::SYS_openat,
        libc::SYS_readlinkat,
        libc::SYS_statx,
        libc::SYS_lseek,
        libc::SYS_pread64,
        libc::SYS_readv,
        libc::SYS_getuid,
        libc::SYS_geteuid,
    ]);
    #[cfg(target_arch = "x86_64")]
    allowed.extend([libc::SYS_open, libc::SYS_readlink, libc::SYS_newfstatat]);
    #[cfg(target_arch = "aarch64")]
    allowed.push(libc::SYS_newfstatat);
    allowed
}
pub(super) fn capture() -> Result<(), String> {
    install(&capture_syscalls())
}
pub(super) fn capability_reply(capture: bool) -> Result<(), String> {
    let mut allowed = if capture {
        capture_syscalls()
    } else {
        BASE.to_vec()
    };
    // Only send on the already-held private channel; socket creation/connect/receive are denied.
    allowed.extend([libc::SYS_sendmsg, libc::SYS_shutdown]);
    install(&allowed)
}
