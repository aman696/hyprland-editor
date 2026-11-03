use super::*;
use std::{
    cell::Cell,
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
};

/// The trusted fixture signals that its interpreter is running, then sleeps
/// without creating descendants. Only its explicitly inherited pipe is used.
fn sleeper() -> (Command, File, File) {
    let mut pipe = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) },
        0
    );
    let read = unsafe { File::from_raw_fd(pipe[0]) };
    let write = unsafe { File::from_raw_fd(pipe[1]) };
    let fd = write.as_raw_fd();
    let mut command = Command::new("/usr/bin/python3");
    command.env_clear().args([
        "-I",
        "-c",
        "import os,time;os.write(63,b'R');time.sleep(30)",
    ]);
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(fd, 63) < 0 || libc::fcntl(63, libc::F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    (command, read, write)
}
fn ready(mut read: File, write: File) {
    drop(write);
    let mut poll = libc::pollfd {
        fd: read.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(
        unsafe { libc::poll(&mut poll, 1, 3000) },
        1,
        "Fixture readiness timed out"
    );
    let mut marker = [0];
    read.read_exact(&mut marker).unwrap();
    assert_eq!(marker, [b'R']);
}
fn reaped(pid: u32) {
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
#[test]
fn precancelled_parser_request_never_reaches_executable_selection_or_spawn() {
    let cancelled = AtomicBool::new(true);
    assert!(run_cancellable(Request::Probe, &cancelled)
        .err()
        .unwrap()
        .contains("cancelled"));
    let result = execute(
        Command::new("/no-such-parser-executable"),
        vec![],
        &cancelled,
        |_| panic!("spawned after cancellation"),
    );
    assert!(result.err().unwrap().contains("cancelled"));
}
#[test]
fn cancellation_kills_a_running_owned_child_reaps_it_and_joins_pipe_threads() {
    let (command, read, write) = sleeper();
    let cancelled = AtomicBool::new(false);
    let pid = Cell::new(0);
    let cancellation = Cell::new(None);
    let result = execute(command, vec![], &cancelled, |child| {
        pid.set(child);
        ready(read, write);
        cancellation.set(Some(Instant::now()));
        cancelled.store(true, Ordering::Release);
    });
    assert!(result.err().unwrap().contains("cancelled"));
    assert!(cancellation.get().unwrap().elapsed() < Duration::from_secs(2));
    reaped(pid.get());
}
#[test]
fn wall_deadline_kills_and_reaps_the_owned_sleeping_child() {
    let (command, read, write) = sleeper();
    let pid = Cell::new(0);
    let start = Instant::now();
    let result = execute(command, vec![], &AtomicBool::new(false), |child| {
        pid.set(child);
        ready(read, write);
    });
    assert!(result.err().unwrap().contains("wall-clock"));
    assert!(start.elapsed() < Duration::from_secs(6));
    reaped(pid.get());
}
#[test]
fn unwinding_after_spawn_still_kills_and_reaps_the_owned_child() {
    let (command, read, write) = sleeper();
    let pid = Cell::new(0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(command, vec![], &AtomicBool::new(false), |child| {
            pid.set(child);
            ready(read, write);
            panic!("fixture unwind");
        })
    }));
    assert!(result.is_err());
    assert!(pid.get() > 0);
    reaped(pid.get());
}
