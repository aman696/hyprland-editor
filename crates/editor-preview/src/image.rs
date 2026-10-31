//! Content-pinned runtime files. A manifest is data, never execution authority.
//! Snapshots own sealed copies and retain no host directory or source descriptor.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Write},
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd},
        unix::fs::{FileExt, MetadataExt},
    },
};

pub const MAX_MANIFEST_BYTES: usize = 512 * 1024;
pub const MAX_FILES: usize = 1024;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: u64 = 256 * 1024 * 1024;
const SEALS: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpec {
    /// Absolute destination inside the image; never a host source path.
    pub destination: String,
    pub bytes: u64,
    /// Lowercase, full SHA-256 of exact bytes, not an origin/trust assertion.
    pub sha256: String,
    pub executable: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub entry: String,
    pub files: Vec<FileSpec>,
}
impl Manifest {
    /// Bounds input before JSON allocation. Parsing does not open any path.
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("Runtime manifest exceeds its byte bound"));
        }
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|_| invalid("Invalid runtime manifest schema"))?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.format != 1 || self.files.is_empty() || self.files.len() > MAX_FILES {
            return Err(invalid("Unsupported runtime image format or file count"));
        }
        let mut paths = BTreeSet::new();
        let mut total = 0u64;
        for file in &self.files {
            if !destination(&file.destination)
                || !paths.insert(file.destination.as_str())
                || file.bytes > MAX_FILE_BYTES
                || file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid("Invalid or duplicate runtime file specification"));
            }
            total = total
                .checked_add(file.bytes)
                .ok_or_else(|| invalid("Runtime image size overflow"))?;
        }
        if total > MAX_IMAGE_BYTES {
            return Err(invalid("Runtime image exceeds its aggregate byte bound"));
        }
        // A file cannot also be a parent directory, including across input order.
        for path in &paths {
            for (index, _) in path.match_indices('/').skip(1) {
                if paths.contains(&path[..index]) {
                    return Err(invalid("Runtime file collides with a parent directory"));
                }
            }
        }
        if !self
            .files
            .iter()
            .any(|f| f.destination == self.entry && f.executable && f.bytes > 0)
        {
            return Err(invalid(
                "Runtime entry must name a nonempty executable image file",
            ));
        }
        Ok(())
    }
    /// Canonical, order-independent identity binding paths, modes, lengths and data.
    pub fn identity(&self) -> io::Result<String> {
        self.validate()?;
        let mut hash = Sha256::new();
        hash.update(b"HYE-RUNTIME-IMAGE-1\0");
        hash.update((self.entry.len() as u32).to_le_bytes());
        hash.update(self.entry.as_bytes());
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_key(|f| &f.destination);
        hash.update((files.len() as u32).to_le_bytes());
        for file in files {
            hash.update((file.destination.len() as u32).to_le_bytes());
            hash.update(file.destination.as_bytes());
            hash.update(file.bytes.to_le_bytes());
            hash.update([u8::from(file.executable)]);
            hash.update(file.sha256.as_bytes());
        }
        Ok(format!("{:x}", hash.finalize()))
    }
}
fn destination(path: &str) -> bool {
    path.len() <= 512
        && ["/app/", "/usr/", "/lib/", "/lib64/", "/etc/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
        && path[1..].split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._+-@".contains(&b))
        })
}

#[derive(Debug)]
pub struct Image {
    manifest: Manifest,
    identity: String,
    files: Vec<File>,
}
impl Image {
    /// Explicit caller-supplied live files, in manifest order. No path lookup,
    /// recursive dependencies, ELF interpretation or origin executable invocation.
    /// Call only from a bounded staging worker; this copies/hashes up to 256 MiB.
    pub fn capture(manifest: Manifest, inputs: &[BorrowedFd<'_>]) -> io::Result<Self> {
        let identity = manifest.identity()?;
        if inputs.len() != manifest.files.len() {
            return Err(invalid("Runtime input count differs from manifest"));
        }
        let files = manifest
            .files
            .iter()
            .zip(inputs)
            .map(|(spec, fd)| capture(spec, *fd))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            manifest,
            identity,
            files,
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn files(&self) -> impl ExactSizeIterator<Item = (&FileSpec, BorrowedFd<'_>)> {
        use std::os::fd::AsFd;
        self.manifest
            .files
            .iter()
            .zip(self.files.iter().map(|f| f.as_fd()))
    }
}
fn capture(spec: &FileSpec, input: BorrowedFd<'_>) -> io::Result<File> {
    // Duplicate only to own the staging read lifetime; pread preserves source offset.
    let source = File::from(input.try_clone_to_owned()?);
    let flags = unsafe { libc::fcntl(source.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    let before = source.metadata()?;
    if !before.is_file() || before.len() != spec.bytes || flags & libc::O_PATH != 0 {
        return Err(invalid(
            "Runtime input must be an exact-size readable regular file",
        ));
    }
    let raw = unsafe {
        libc::memfd_create(
            c"hye-runtime-file".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: newly returned unique descriptor, owned exactly once.
    let mut copy = unsafe { File::from_raw_fd(raw) };
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 16384];
    let mut offset = 0;
    while offset < spec.bytes {
        let count = (spec.bytes - offset).min(buffer.len() as u64) as usize;
        source.read_exact_at(&mut buffer[..count], offset)?;
        hash.update(&buffer[..count]);
        copy.write_all(&buffer[..count])?;
        offset += count as u64;
    }
    let after = source.metadata()?;
    if (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) || source.read_at(&mut buffer[..1], offset)? != 0
        || format!("{:x}", hash.finalize()) != spec.sha256
    {
        return Err(invalid("Runtime input changed or content digest differs"));
    }
    // Snapshot descriptors themselves are data, not executable bootstrap authority.
    if unsafe { libc::fchmod(raw, 0o400) } != 0
        || unsafe { libc::fcntl(raw, libc::F_ADD_SEALS, SEALS) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // ro-bind-data consumes from the current offset of its inherited duplicate.
    if unsafe { libc::lseek(raw, 0, libc::SEEK_SET) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(copy)
}

#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
