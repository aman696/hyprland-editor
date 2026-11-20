//! Explicit reference-test image, never selected by GUI or saved data.
use hyprland_editor_core::lua::replay::{self, Program};
use std::io::{Read, Write};
#[allow(dead_code)]
#[path = "../../editor-app/src/process_policy.rs"]
mod process_policy;
fn main() {
    let result = (|| -> Result<replay::Report, String> {
        for (resource, max) in [
            (libc::RLIMIT_AS, 512 * 1024 * 1024),
            (libc::RLIMIT_CPU, 2),
            (libc::RLIMIT_CORE, 0),
            (libc::RLIMIT_STACK, 8 * 1024 * 1024),
        ] {
            let limit = libc::rlimit {
                rlim_cur: max,
                rlim_max: max,
            };
            if unsafe { libc::setrlimit(resource, &limit) } != 0 {
                return Err("Reference resource limits unavailable".into());
            }
        }
        if unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } != 0
            || unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
        {
            return Err("Reference isolation unavailable".into());
        }
        process_policy::parser()?;
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Reference input failed")?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("Reference input too large".into());
        }
        let program: Program =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid reference program")?;
        replay::replay(&program)
    })();
    match result {
        Ok(report) => {
            let bytes = serde_json::to_vec(&report).unwrap();
            assert!(bytes.len() <= 4 * 1024 * 1024);
            std::io::stdout().write_all(&bytes).unwrap();
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
