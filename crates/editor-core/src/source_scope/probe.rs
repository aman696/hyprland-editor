//! Typed, metadata-only point observations below a live scope. No read permission
//! or stable filesystem snapshot is inferred from a positive or negative result.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    NotFound,
    NotDirectory,
    PermissionDenied,
    NeedsGrant,
    AuthorityChanged,
    LinkChanged,
    UnsupportedIsolation,
    InvalidInput,
    Budget,
    Unstable,
    Io,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Input,
    Scope,
    Alias,
    Open,
    Traverse,
    Metadata,
    Link,
    Recheck,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub kind: FailureKind,
    pub stage: Stage,
    pub at: Vec<u8>,
    pub errno: Option<i32>,
    pub detail: String,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}
impl std::error::Error for Failure {}
impl Failure {
    fn new(kind: FailureKind, stage: Stage, at: &[u8], detail: impl Into<String>) -> Self {
        Self {
            kind,
            stage,
            at: if at.len() <= 4096 {
                at.to_vec()
            } else {
                vec![]
            },
            errno: None,
            detail: detail.into(),
        }
    }
    fn system(stage: Stage, at: &[u8], error: std::io::Error) -> Self {
        let kind = match error.raw_os_error() {
            Some(libc::ENOENT) if stage == Stage::Open => FailureKind::NotFound,
            Some(libc::ENOTDIR) if stage == Stage::Open => FailureKind::NotDirectory,
            Some(libc::EACCES | libc::EPERM) => FailureKind::PermissionDenied,
            Some(libc::EXDEV) => FailureKind::NeedsGrant,
            Some(libc::ENOSYS | libc::EINVAL | libc::E2BIG) => FailureKind::UnsupportedIsolation,
            Some(libc::ELOOP | libc::EAGAIN | libc::ESTALE) => FailureKind::Unstable,
            Some(libc::ENAMETOOLONG) => FailureKind::Budget,
            _ => FailureKind::Io,
        };
        let mut failure = Self::new(
            kind,
            stage,
            at,
            match kind {
                FailureKind::NeedsGrant => {
                    "Dependency crosses its approved folder or mount; select its target separately"
                        .into()
                }
                FailureKind::UnsupportedIsolation => {
                    "Dependency capture requires openat2 containment support".into()
                }
                _ => error.to_string(),
            },
        );
        failure.errno = error.raw_os_error();
        failure
    }
    fn absence(&self) -> bool {
        matches!(
            (self.kind, self.stage),
            (FailureKind::NotFound, Stage::Open)
                | (FailureKind::NotDirectory, Stage::Open | Stage::Traverse)
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEvidence {
    pub device: u64,
    pub inode: u64,
    pub mount_id: u64,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub links: u64,
    pub size: u64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: i64,
}
impl NodeEvidence {
    fn observe(node: &File) -> Result<Self, Failure> {
        let m = node
            .metadata()
            .map_err(|e| Failure::system(Stage::Metadata, b"", e))?;
        if m.nlink() == 0 {
            return Err(Failure::new(
                FailureKind::Unstable,
                Stage::Metadata,
                b"",
                "Dependency was unlinked during observation",
            ));
        }
        Ok(Self {
            device: m.dev(),
            inode: m.ino(),
            mount_id: mount_id(node).map_err(|e| {
                Failure::new(FailureKind::UnsupportedIsolation, Stage::Metadata, b"", e)
            })?,
            owner: m.uid(),
            group: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            size: m.len(),
            changed_seconds: m.ctime(),
            changed_nanoseconds: m.ctime_nsec(),
            modified_seconds: m.mtime(),
            modified_nanoseconds: m.mtime_nsec(),
        })
    }
    pub fn is_regular(&self) -> bool {
        self.mode & libc::S_IFMT == libc::S_IFREG
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.links == 0
            || self.mount_id == 0
            || !(0..1_000_000_000).contains(&self.changed_nanoseconds)
            || !(0..1_000_000_000).contains(&self.modified_nanoseconds)
        {
            return Err("Invalid dependency metadata evidence".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Status {
    Present { node: NodeEvidence },
    Absent { errno: i32, at: Vec<u8> },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub candidate: Vec<u8>,
    pub selected_path: Vec<u8>,
    pub links: Vec<LinkIdentity>,
    pub status: Status,
}
impl Observation {
    /// Data integrity only. This never restores authority or certifies a stable
    /// directory, read access, loader precedence, ownership or effective state.
    pub fn validate(&self) -> Result<(), String> {
        if self.candidate.is_empty()
            || self.candidate.len() > 4096
            || self.candidate.contains(&0)
            || !path_ok(&self.selected_path)
            || self.links.len() > 32
        {
            return Err("Invalid scoped dependency observation".into());
        }
        for link in &self.links {
            if !path_ok(&link.encountered_path)
                || link.target.is_empty()
                || link.target.len() > 4096
                || link.target.contains(&0)
                || link.mode & libc::S_IFMT != libc::S_IFLNK
                || !(0..1_000_000_000).contains(&link.changed_nanoseconds)
            {
                return Err("Invalid dependency link observation".into());
            }
        }
        match &self.status {
            Status::Present { node } => node.validate(),
            Status::Absent { errno, at }
                if [libc::ENOENT, libc::ENOTDIR].contains(errno)
                    && !at.is_empty()
                    && at.len() <= 4096
                    && !at.contains(&0) =>
            {
                Ok(())
            }
            _ => Err("Invalid dependency absence observation".into()),
        }
    }
}
#[derive(Default)]
pub(super) struct Chain {
    pub(super) links: Vec<LinkIdentity>,
    pub(super) held_links: Vec<File>,
}
impl Scope {
    /// Capture one metadata observation. Only ENOENT/ENOTDIR from a scoped
    /// traversal are absence. Denials, alias changes, unsupported isolation,
    /// budgets, transport failures and I/O uncertainty are never absence.
    pub fn probe(&self, candidate: &[u8]) -> Result<Observation, Failure> {
        let mut chain = Chain::default();
        let result = self.walk(candidate, &mut chain);
        self.finish_probe(candidate, chain, result)
    }
    pub(super) fn finish_probe(
        &self,
        candidate: &[u8],
        chain: Chain,
        result: Result<File, Failure>,
    ) -> Result<Observation, Failure> {
        let result = result.and_then(|node| NodeEvidence::observe(&node));
        source_path::validate_held_links(&chain.links, &chain.held_links)
            .map_err(|e| Failure::new(FailureKind::LinkChanged, Stage::Recheck, b"", e))?;
        self.validate()
            .map_err(|e| Failure::new(FailureKind::AuthorityChanged, Stage::Recheck, b"", e))?;
        let status = match result {
            Ok(node) => Status::Present { node },
            Err(error) if error.absence() => Status::Absent {
                errno: error.errno.expect("Absence has errno"),
                at: error.at,
            },
            Err(error) => return Err(error),
        };
        let selected_path = self
            .selected_path(candidate)
            .map_err(|e| Failure::new(FailureKind::InvalidInput, Stage::Input, b"", e))?;
        let observation = Observation {
            candidate: candidate.to_vec(),
            selected_path,
            links: chain.links,
            status,
        };
        observation
            .validate()
            .map_err(|e| Failure::new(FailureKind::Io, Stage::Metadata, b"", e))?;
        Ok(observation)
    }
    pub(super) fn walk(&self, candidate: &[u8], chain: &mut Chain) -> Result<File, Failure> {
        self.validate()
            .map_err(|e| Failure::new(FailureKind::AuthorityChanged, Stage::Scope, b"", e))?;
        if candidate.is_empty() || candidate.len() > 4096 || candidate.contains(&0) {
            return Err(Failure::new(
                FailureKind::InvalidInput,
                Stage::Input,
                b"",
                "Invalid dependency path",
            ));
        }
        let relative = self
            .relative(candidate)
            .map_err(|e| Failure::new(FailureKind::NeedsGrant, Stage::Input, candidate, e))?;
        self.verify_absolute_alias(candidate)
            .map_err(|e| Failure::new(FailureKind::AuthorityChanged, Stage::Alias, candidate, e))?;
        let mut pending = source_path::components(relative);
        let mut resolved: Vec<Vec<u8>> = vec![];
        let mut steps = 0;
        while let Some(part) = pending.pop_front() {
            steps += 1;
            if steps > 4096 {
                return Err(Failure::new(
                    FailureKind::Budget,
                    Stage::Traverse,
                    b"",
                    "Dependency path resolution exceeded its step budget",
                ));
            }
            if part == b"." {
                continue;
            }
            if part == b".." {
                if resolved.pop().is_none() {
                    return Err(Failure::new(
                        FailureKind::NeedsGrant,
                        Stage::Traverse,
                        relative,
                        "Dependency escapes its approved folder; select its target separately",
                    ));
                }
                continue;
            }
            let joined = join_relative(&resolved, &part);
            let node = open_node(&self.node, &joined)?;
            let meta = node
                .metadata()
                .map_err(|e| Failure::system(Stage::Metadata, &joined, e))?;
            if meta.file_type().is_symlink() {
                if chain.links.len() >= 32 {
                    return Err(Failure::new(
                        FailureKind::Budget,
                        Stage::Link,
                        &joined,
                        "Dependency symlink cycle or depth limit exceeded",
                    ));
                }
                if source_path::is_proc_link(&node)
                    .map_err(|e| Failure::new(FailureKind::Io, Stage::Link, &joined, e))?
                {
                    return Err(Failure::new(
                        FailureKind::NeedsGrant,
                        Stage::Link,
                        &joined,
                        "Dependency magic links are not authorized",
                    ));
                }
                let target = source_path::link_bytes(&node)
                    .map_err(|e| Failure::new(FailureKind::Io, Stage::Link, &joined, e))?;
                chain.links.push(LinkIdentity {
                    encountered_path: join_absolute(&self.identity.resolved_path, &joined)
                        .map_err(|e| Failure::new(FailureKind::Budget, Stage::Link, &joined, e))?,
                    target: target.clone(),
                    device: meta.dev(),
                    inode: meta.ino(),
                    owner: meta.uid(),
                    mode: meta.mode(),
                    changed_seconds: meta.ctime(),
                    changed_nanoseconds: meta.ctime_nsec(),
                });
                chain.held_links.push(node);
                let mut next = if target.starts_with(b"/") {
                    let relative_target = self.relative(&target).map_err(|e| {
                        Failure::new(FailureKind::NeedsGrant, Stage::Link, &target, e)
                    })?;
                    self.verify_absolute_alias(&target).map_err(|e| {
                        Failure::new(FailureKind::AuthorityChanged, Stage::Alias, &target, e)
                    })?;
                    resolved.clear();
                    source_path::components(relative_target)
                } else {
                    source_path::components(&target)
                };
                next.append(&mut pending);
                pending = next;
                continue;
            }
            if !pending.is_empty() {
                if !meta.is_dir() {
                    let mut error = Failure::new(
                        FailureKind::NotDirectory,
                        Stage::Traverse,
                        &joined,
                        "A dependency path component is not a directory",
                    );
                    error.errno = Some(libc::ENOTDIR);
                    return Err(error);
                }
                resolved.push(part);
                continue;
            }
            return Ok(node);
        }
        open_node(&self.node, &join_relative(&resolved, b"."))
    }
}
pub(super) fn open_node(root: &File, path: &[u8]) -> Result<File, Failure> {
    #[repr(C)]
    struct OpenHow {
        flags: u64,
        mode: u64,
        resolve: u64,
    }
    let name = CString::new(path).map_err(|_| {
        Failure::new(
            FailureKind::InvalidInput,
            Stage::Input,
            b"",
            "Invalid dependency path",
        )
    })?;
    let how = OpenHow {
        flags: (libc::O_PATH | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64,
        mode: 0,
        resolve: 0x08 | 0x02 | 0x01 | 0x04,
    };
    // BENEATH | NO_MAGICLINKS | NO_XDEV | NO_SYMLINKS. O_PATH|NOFOLLOW
    // still returns a final symlink itself for explicit manual inspection.
    // Unrecorded intermediate symlinks (including raced replacements) cannot
    // be silently followed by the repeated root-relative lookup.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            root.as_raw_fd(),
            name.as_ptr(),
            &how,
            std::mem::size_of::<OpenHow>(),
        )
    };
    if fd < 0 {
        return Err(Failure::system(
            Stage::Open,
            path,
            std::io::Error::last_os_error(),
        ));
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}
