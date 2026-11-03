use super::*;
use std::{
    io::{Read, Seek, SeekFrom},
    os::fd::AsFd,
};
pub(super) fn source(bytes: &[u8]) -> File {
    let raw = unsafe { libc::memfd_create(c"hye-image-test".as_ptr(), libc::MFD_CLOEXEC) };
    assert!(raw >= 0);
    let mut file = unsafe { File::from_raw_fd(raw) };
    file.write_all(bytes).unwrap();
    file
}
fn spec(path: &str, bytes: &[u8], executable: bool) -> FileSpec {
    FileSpec {
        destination: path.into(),
        bytes: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
        executable,
    }
}
fn manifest() -> Manifest {
    Manifest {
        format: 1,
        entry: "/app/probe".into(),
        files: vec![spec("/app/probe", b"original", true)],
    }
}
#[test]
fn frozen_image_survives_origin_mutation_without_retaining_source_offset() {
    let mut source = source(b"original");
    let position = source.stream_position().unwrap();
    let image = Image::capture(manifest(), &[source.as_fd()]).unwrap();
    assert_eq!(source.stream_position().unwrap(), position);
    source.write_all_at(b"modified", 0).unwrap();
    drop(source);
    let (_, fd) = image.files().next().unwrap();
    let mut copy = File::from(fd.try_clone_to_owned().unwrap());
    let mut bytes = Vec::new();
    copy.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"original");
    assert_eq!(
        copy.write_at(b"x", 0).unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );
    assert!(copy.set_len(0).is_err());
    assert_eq!(
        unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GET_SEALS) },
        SEALS
    );
}
#[test]
fn manifest_cannot_supply_authority_paths_links_modes_or_unknown_fields() {
    for path in [
        "/proc/self/fd/4",
        "/run/x",
        "/dev/null",
        "/home/user",
        "/tmp/x",
        "/app/../proc/x",
        "/app/./x",
        "/app//x",
        "/app/",
        "app/x",
        "/app/a\0b",
        "/app/a\nb",
        "/app/á",
        "/app/a b",
    ] {
        let mut m = manifest();
        m.files[0].destination = path.into();
        m.entry = path.into();
        assert!(m.validate().is_err(), "{path:?}");
    }
    for added in [
        r#", "host_path":"/etc/shadow" "#,
        r#", "symlink":"/proc" "#,
        r#", "mode":4095 "#,
    ] {
        let bytes = format!(r#"{{"format":1,"entry":"/app/probe","files":[] {added}}}"#);
        assert!(Manifest::parse(bytes.as_bytes()).is_err());
    }
    assert!(Manifest::parse(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
    assert!(Manifest::parse(br#"{"format":1,"format":1,"entry":"x","files":[]}"#).is_err());
}
#[test]
fn digest_size_type_entry_and_parent_collisions_refuse_before_execution() {
    let source = source(b"original");
    let mut m = manifest();
    m.files[0].sha256 = "0".repeat(64);
    assert!(Image::capture(m, &[source.as_fd()]).is_err());
    for size in [0, 7, 9, MAX_FILE_BYTES + 1] {
        let mut m = manifest();
        m.files[0].bytes = size;
        assert!(Image::capture(m, &[source.as_fd()]).is_err());
    }
    let directory = File::open("/tmp").unwrap();
    assert!(Image::capture(manifest(), &[directory.as_fd()]).is_err());
    let null = File::open("/dev/null").unwrap();
    let mut m = manifest();
    m.files.push(spec("/app/data", b"", false));
    assert!(Image::capture(m, &[source.as_fd(), null.as_fd()]).is_err());
    for path in ["/app/probe", "/app/probe/child", "/app"] {
        let mut m = manifest();
        m.files.push(spec(path, b"a", false));
        assert!(m.validate().is_err());
        m.files.reverse();
        assert!(m.validate().is_err());
    }
    let mut m = manifest();
    m.files[0].executable = false;
    assert!(m.validate().is_err());
    let mut m = manifest();
    m.entry = "/app/absent".into();
    assert!(m.validate().is_err());
    assert!(Image::capture(manifest(), &[]).is_err());
}
#[test]
fn canonical_identity_binds_all_paths_bytes_permissions_and_entry() {
    let mut m = manifest();
    m.files.push(spec("/usr/share/test", b"data", true));
    let identity = m.identity().unwrap();
    m.files.reverse();
    assert_eq!(m.identity().unwrap(), identity);
    for change in 0..5 {
        let mut next = m.clone();
        match change {
            0 => next.entry = "/usr/share/test".into(),
            1 => next.files[0].destination = "/usr/share/other".into(),
            2 => next.files[0].bytes += 1,
            3 => next.files[0].executable = false,
            _ => next.files[0].sha256 = "0".repeat(64),
        }
        assert_ne!(next.identity().unwrap(), identity);
    }
    let encoded = serde_json::to_vec(&m).unwrap();
    assert_eq!(
        Manifest::parse(&encoded).unwrap().identity().unwrap(),
        identity
    );
}
#[test]
fn image_budget_and_file_count_are_aggregate_requirements() {
    let mut m = manifest();
    m.files = (0..5)
        .map(|n| {
            let mut f = spec(&format!("/app/f{n}"), b"x", true);
            f.bytes = MAX_FILE_BYTES;
            f
        })
        .collect();
    m.entry = "/app/f0".into();
    assert!(m.validate().unwrap_err().to_string().contains("aggregate"));
    m.files.truncate(4);
    assert!(m.validate().is_ok());
    m.files = (0..MAX_FILES + 1)
        .map(|n| spec(&format!("/app/f{n}"), b"x", true))
        .collect();
    assert!(m.validate().is_err());
}
#[test]
fn failed_multi_file_capture_releases_partial_snapshot_descriptors() {
    if std::env::var_os("HYE_IMAGE_FD_FAILURE_CHILD").is_none() {
        use std::os::unix::process::CommandExt;
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "image::tests::failed_multi_file_capture_releases_partial_snapshot_descriptors",
                "--nocapture",
            ])
            .env("HYE_IMAGE_FD_FAILURE_CHILD", "1");
        unsafe {
            command.pre_exec(|| {
                let limit = libc::rlimit {
                    rlim_cur: 64,
                    rlim_max: 64,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        assert!(command.status().unwrap().success());
        return;
    }
    let count = || std::fs::read_dir("/proc/self/fd").unwrap().count();
    let first = source(b"original");
    let original_seals = unsafe { libc::fcntl(first.as_raw_fd(), libc::F_GET_SEALS) };
    let second = source(b"wrong");
    let mut m = manifest();
    m.files.push(spec("/usr/share/data", b"right", false));

    let before = count();
    for _ in 0..256 {
        let error = Image::capture(m.clone(), &[first.as_fd(), second.as_fd()]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Runtime input changed or content digest differs"
        );
    }
    assert_eq!(count(), before);
    assert_eq!(
        unsafe { libc::fcntl(first.as_raw_fd(), libc::F_GET_SEALS) },
        original_seals
    );
    let mut copy = File::from(first.as_fd().try_clone_to_owned().unwrap());
    copy.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = String::new();
    copy.read_to_string(&mut bytes).unwrap();
    assert_eq!(bytes, "original");
}
