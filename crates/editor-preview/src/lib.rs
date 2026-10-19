//! Private preview supervision. Data reports never confer execution authority.
//! Actual compositor/frame/input readiness remains a separate, unfinished gate.
#![cfg(target_os = "linux")]
pub mod cgroup;
pub mod channel;
pub mod frame;
