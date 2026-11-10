//! Corpus walk: ignore-aware discovery of Markdown files beneath one root.
//!
//! The corpus depends only on the root directory: `.gitignore` (without
//! requiring a git repository) and `.ontodocignore` files inside the root
//! apply; parent-directory, global and `.git/info/exclude` rules do not.
//! Hidden entries are skipped. Symlinks are never followed. Ineligible files
//! become typed [`Skip`] diagnostics; unsafe resolution and exhausted traversal
//! budgets refuse the operation.

use std::ffi::OsString;
use std::fs::{File, Metadata};
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rustix::fs::{FileType, Mode, OFlags, RawDir, ResolveFlags, openat2};
use sha2::{Digest, Sha256};

/// Index-time limits for the walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkConfig {
    /// Files larger than this are skipped with [`SkipReason::TooLarge`].
    pub max_file_bytes: u64,
    /// More eligible files than this fails the walk with [`CorpusError::TooManyFiles`].
    pub max_files: usize,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: 2 * 1024 * 1024,
            max_files: 100_000,
        }
    }
}

/// One eligible Markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Path relative to the root, `/`-separated.
    pub rel_path: String,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch.
    pub mtime_ns: i64,
    /// Lowercase hex SHA-256 of the file bytes.
    pub sha256: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    Symlink,
    TooLarge {
        size: u64,
        cap: u64,
    },
    NotUtf8,
    NonUtf8Path,
    /// The file changed between the directory scan and the read.
    Changed,
    Io {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    /// Relative path, lossily converted when not UTF-8.
    pub path: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Corpus {
    /// Sorted by `rel_path` bytes.
    pub files: Vec<SourceFile>,
    /// Sorted by `path` bytes.
    pub skipped: Vec<Skip>,
}

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("invalid corpus-relative path")]
    InvalidPath,
    #[error("file not found")]
    NotFound,
    #[error("symlink refused")]
    Symlink,
    #[error("not a regular file")]
    NotRegular,
    #[error("rooted filesystem operations unavailable")]
    UnsupportedFilesystem,
    #[error("filesystem changed during operation")]
    FilesystemChanged,
    #[error("file size {size} exceeds cap {cap}")]
    TooLarge { size: u64, cap: u64 },
    #[error("filesystem I/O failed: {source}")]
    Io { source: std::io::Error },
    #[error("corpus root {path:?} is not a readable directory: {source}")]
    Root {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("corpus root {0:?} is not a directory")]
    NotDirectory(PathBuf),
    #[error("corpus has more than {cap} eligible files")]
    TooManyFiles { cap: usize },
    #[error("corpus capture {0} budget exhausted")]
    CaptureBudget(&'static str),
    #[error("unsafe ignore control {0:?}")]
    UnsafeControl(PathBuf),
    #[error("cannot read ignore control {path:?}: {source}")]
    Control {
        path: PathBuf,
        source: std::io::Error,
    },
}

const MAX_DEPTH: usize = 64;
const MAX_IGNORE_BYTES: usize = 64 * 1024;
const MAX_IGNORE_LINE: usize = 4 * 1024;

/// Retained corpus identity. The logical path is used only for matching/output.
#[derive(Debug)]
pub(crate) struct RootedCorpus {
    fd: OwnedFd,
    root: PathBuf,
    #[cfg(test)]
    next_resolution_error: std::sync::Mutex<Option<rustix::io::Errno>>,
}

fn resolution_error(error: rustix::io::Errno) -> CorpusError {
    use rustix::io::Errno;
    match error {
        Errno::NOSYS | Errno::PERM | Errno::INVAL => CorpusError::UnsupportedFilesystem,
        Errno::AGAIN | Errno::XDEV => CorpusError::FilesystemChanged,
        Errno::LOOP => CorpusError::Symlink,
        Errno::NOENT => CorpusError::NotFound,
        Errno::NOTDIR => CorpusError::NotRegular,
        _ => CorpusError::Io {
            source: error.into(),
        },
    }
}

fn enumeration_error(error: rustix::io::Errno) -> CorpusError {
    if error == rustix::io::Errno::NOENT {
        CorpusError::FilesystemChanged
    } else {
        // EINVAL here is a directory-buffer/record refusal, not an openat2
        // capability probe. Preserve it as I/O without growing or retrying.
        CorpusError::Io {
            source: error.into(),
        }
    }
}

fn normal_relative(path: &Path) -> Result<(), CorpusError> {
    if path.as_os_str().as_bytes().contains(&0) {
        return Err(CorpusError::InvalidPath);
    }
    let mut rebuilt = PathBuf::new();
    for part in path.components() {
        let Component::Normal(part) = part else {
            return Err(CorpusError::InvalidPath);
        };
        rebuilt.push(part);
    }
    if rebuilt.as_os_str().is_empty() || rebuilt.as_os_str() != path.as_os_str() {
        return Err(CorpusError::InvalidPath);
    }
    Ok(())
}

impl RootedCorpus {
    pub(crate) fn open(path: &Path) -> Result<Self, CorpusError> {
        if path.as_os_str().is_empty() {
            return Err(CorpusError::InvalidPath);
        }
        // Path components remove trailing separators and dots without following
        // the final component. Only the trusted parent prefix is canonicalized.
        let normalized: PathBuf = path.components().collect();
        let absolute = if normalized.is_absolute() {
            normalized
        } else {
            std::env::current_dir()
                .map_err(|source| CorpusError::Io { source })?
                .join(normalized)
        };
        let root = match (absolute.parent(), absolute.file_name()) {
            (Some(parent), Some(name)) => parent
                .canonicalize()
                .map_err(|source| CorpusError::Root {
                    path: path.to_path_buf(),
                    source,
                })?
                .join(name),
            _ => absolute,
        };
        let fd = rustix::fs::open(
            &root,
            OFlags::PATH | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|error| match error {
            rustix::io::Errno::LOOP => CorpusError::Symlink,
            rustix::io::Errno::NOTDIR => CorpusError::NotDirectory(root.clone()),
            _ => CorpusError::Root {
                path: root.clone(),
                source: error.into(),
            },
        })?;
        let kind =
            FileType::from_raw_mode(rustix::fs::fstat(&fd).map_err(resolution_error)?.st_mode);
        if kind == FileType::Symlink {
            return Err(CorpusError::Symlink);
        }
        if kind != FileType::Directory {
            return Err(CorpusError::NotDirectory(root));
        }
        let reader = Self {
            fd,
            root,
            #[cfg(test)]
            next_resolution_error: std::sync::Mutex::new(None),
        };
        // Probe the actual operation, including seccomp denial; never fall back.
        reader.open_dir(Path::new(""))?;
        Ok(reader)
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }

    fn open_relative(&self, path: &Path, flags: OFlags) -> Result<OwnedFd, CorpusError> {
        normal_relative(path)?;
        self.resolve(path, flags)
    }

    fn resolve(&self, path: &Path, flags: OFlags) -> Result<OwnedFd, CorpusError> {
        #[cfg(test)]
        if let Some(error) = self
            .next_resolution_error
            .lock()
            .map_err(|_| CorpusError::FilesystemChanged)?
            .take()
        {
            return Err(resolution_error(error));
        }
        openat2(
            &self.fd,
            path,
            flags | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
        )
        .map_err(resolution_error)
    }

    fn open_dir(&self, relative: &Path) -> Result<OwnedFd, CorpusError> {
        if relative.as_os_str().is_empty() {
            self.resolve(Path::new("."), OFlags::RDONLY | OFlags::DIRECTORY)
        } else {
            self.open_relative(relative, OFlags::RDONLY | OFlags::DIRECTORY)
        }
    }

    pub(crate) fn read_regular(
        &self,
        relative: &Path,
        cap: u64,
    ) -> Result<(Metadata, Vec<u8>), CorpusError> {
        let fd = match self.open_relative(relative, OFlags::RDONLY | OFlags::NONBLOCK) {
            Ok(fd) => fd,
            Err(CorpusError::Io { source })
                if source.raw_os_error().is_some_and(|code| {
                    code == rustix::io::Errno::NXIO.raw_os_error()
                        || code == rustix::io::Errno::NODEV.raw_os_error()
                }) =>
            {
                // Linux refuses socket/device opens before descriptor inspection.
                // Classify the rooted object, not the errno alone; a regular-file
                // I/O refusal must never turn into a citation's `missing` state.
                if entry_kind(self, relative, FileType::Unknown)? != FileType::RegularFile {
                    return Err(CorpusError::NotRegular);
                }
                return Err(CorpusError::Io { source });
            }
            Err(error) => return Err(error),
        };
        let file = File::from(fd);
        read_descriptor(file, cap, || {})
    }

    pub(crate) fn walk(
        &self,
        config: &WalkConfig,
        budget: &mut WalkBudget,
    ) -> Result<Corpus, CorpusError> {
        let mut files = Vec::new();
        let skipped = self.visit(config, budget, |file| files.push(file))?;
        Ok(sort_corpus(Corpus { files, skipped }))
    }

    /// Streams accepted sources in traversal order (not global path order).
    /// Each source is owned by the callback; only sorted skips are retained here.
    pub(crate) fn visit(
        &self,
        config: &WalkConfig,
        budget: &mut WalkBudget,
        on_file: impl FnMut(SourceFile),
    ) -> Result<Vec<Skip>, CorpusError> {
        let config = WalkConfig {
            max_file_bytes: config.max_file_bytes.min(2 * 1024 * 1024),
            max_files: config.max_files.min(100_000),
        };
        let mut state = VisitState {
            accepted: 0,
            skipped: Vec::new(),
            on_file,
        };
        budget.with_local_limits(128 * 1024 * 1024, 100_000, |budget| {
            bounded_dir(
                self,
                &self.root,
                &config,
                budget,
                &mut Vec::new(),
                &mut state,
            )
        })?;
        state
            .skipped
            .sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(state.skipped)
    }
}

fn read_descriptor(
    file: File,
    cap: u64,
    after_metadata: impl FnOnce(),
) -> Result<(Metadata, Vec<u8>), CorpusError> {
    let limit = cap
        .checked_add(1)
        .ok_or(CorpusError::CaptureBudget("byte"))?;
    usize::try_from(limit).map_err(|_| CorpusError::CaptureBudget("byte"))?;
    let meta = file
        .metadata()
        .map_err(|source| CorpusError::Io { source })?;
    if !meta.is_file() {
        return Err(CorpusError::NotRegular);
    }
    if meta.len() > cap {
        return Err(CorpusError::TooLarge {
            size: meta.len(),
            cap,
        });
    }
    after_metadata();
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|source| CorpusError::Io { source })?;
    let size = bytes.len() as u64;
    if size > cap {
        return Err(CorpusError::TooLarge { size, cap });
    }
    if size != meta.len() {
        return Err(CorpusError::FilesystemChanged);
    }
    Ok((meta, bytes))
}

/// Aggregate capture budgets, shared across corpus walks. Matching units
/// bound matcher visits weighted by rule count, not elapsed CPU or filesystem time.
pub(crate) struct WalkBudget {
    pub bytes: usize,
    pub entries: usize,
    paths: usize,
    lines: usize,
    matching: usize,
}

impl WalkBudget {
    pub(crate) fn new(bytes: usize, entries: usize) -> Self {
        Self {
            bytes,
            entries,
            paths: 16 * 1024 * 1024,
            lines: 4096,
            matching: 1_000_000,
        }
    }

    fn with_local_limits<T>(
        &mut self,
        bytes: usize,
        entries: usize,
        operation: impl FnOnce(&mut Self) -> Result<T, CorpusError>,
    ) -> Result<T, CorpusError> {
        let excluded_bytes = self.bytes.saturating_sub(bytes);
        let excluded_entries = self.entries.saturating_sub(entries);
        self.bytes -= excluded_bytes;
        self.entries -= excluded_entries;
        let result = operation(self);
        self.bytes += excluded_bytes;
        self.entries += excluded_entries;
        result
    }
}

struct VisitState<F> {
    accepted: usize,
    skipped: Vec<Skip>,
    on_file: F,
}

fn charge(left: &mut usize, amount: usize, name: &'static str) -> Result<(), CorpusError> {
    *left = left
        .checked_sub(amount)
        .ok_or(CorpusError::CaptureBudget(name))?;
    Ok(())
}

/// Walks `root` and returns eligible Markdown files plus skip diagnostics.
pub fn walk(root: &Path, config: &WalkConfig) -> Result<Corpus, CorpusError> {
    walk_bounded(
        root,
        config,
        &mut WalkBudget::new(128 * 1024 * 1024, 100_000),
    )
}

pub(crate) fn walk_bounded(
    root: &Path,
    config: &WalkConfig,
    budget: &mut WalkBudget,
) -> Result<Corpus, CorpusError> {
    RootedCorpus::open(root)?.walk(config, budget)
}

#[cfg(test)]
fn canonical_root(root: &Path) -> Result<PathBuf, CorpusError> {
    let root = root.canonicalize().map_err(|source| CorpusError::Root {
        path: root.to_path_buf(),
        source,
    })?;
    if !root.is_dir() {
        return Err(CorpusError::NotDirectory(root));
    }
    Ok(root)
}

#[cfg(test)]
fn walk_inner(root: &Path, config: &WalkConfig) -> Result<Corpus, CorpusError> {
    let root = canonical_root(root)?;
    let mut walker = ignore::WalkBuilder::new(&root);
    walker
        .follow_links(false)
        .hidden(true)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .add_custom_ignore_filename(".ontodocignore");

    let mut corpus = Corpus::default();
    for entry in walker.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let path = error_path(&err)
                    .map(|p| lossy_rel(&root, p))
                    .unwrap_or_default();
                corpus.skipped.push(Skip {
                    path,
                    reason: SkipReason::Io {
                        message: err.to_string(),
                    },
                });
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        collect_file(&root, path, file_type, config, &mut corpus, None)?;
    }
    Ok(sort_corpus(corpus))
}

#[cfg(test)]
fn collect_file(
    root: &Path,
    path: &Path,
    file_type: std::fs::FileType,
    config: &WalkConfig,
    corpus: &mut Corpus,
    budget: Option<&mut WalkBudget>,
) -> Result<(), CorpusError> {
    if file_type.is_symlink() {
        corpus.skipped.push(Skip {
            path: lossy_rel(root, path),
            reason: SkipReason::Symlink,
        });
        return Ok(());
    }
    if !file_type.is_file() || !is_markdown(path) {
        return Ok(());
    }
    let Some(rel_path) = rel_slash_path(root, path) else {
        corpus.skipped.push(Skip {
            path: lossy_rel(root, path),
            reason: SkipReason::NonUtf8Path,
        });
        return Ok(());
    };
    // Reserve before opening, including overflow detection. Failed reads keep
    // their reservation, so retries cannot escape accounting.
    let reservation = if let Some(budget) = budget.as_ref() {
        let reserve = usize::try_from(config.max_file_bytes)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(CorpusError::CaptureBudget("byte"))?;
        if budget.bytes < reserve {
            return Err(CorpusError::CaptureBudget("byte"));
        }
        reserve
    } else {
        0
    };
    let mut budget = budget;
    if let Some(budget) = budget.as_deref_mut() {
        budget.bytes -= reservation;
    }
    let result = read_file(path, config.max_file_bytes);
    if let (Some(budget), Ok((_, bytes))) = (budget, &result) {
        budget.bytes += reservation - bytes.len();
    }
    match result {
        Ok((meta, bytes)) => match String::from_utf8(bytes) {
            Ok(text) => {
                if corpus.files.len() == config.max_files {
                    return Err(CorpusError::TooManyFiles {
                        cap: config.max_files,
                    });
                }
                corpus.files.push(SourceFile {
                    rel_path,
                    size: meta.len(),
                    mtime_ns: mtime_ns(&meta),
                    sha256: sha256_hex(text.as_bytes()),
                    text,
                });
            }
            Err(_) => corpus.skipped.push(Skip {
                path: rel_path,
                reason: SkipReason::NotUtf8,
            }),
        },
        Err(reason) => corpus.skipped.push(Skip {
            path: rel_path,
            reason,
        }),
    }
    Ok(())
}

fn sort_corpus(mut corpus: Corpus) -> Corpus {
    corpus
        .files
        .sort_by(|a, b| a.rel_path.as_bytes().cmp(b.rel_path.as_bytes()));
    corpus
        .skipped
        .sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    corpus
}

// No automatic ignore loaders: enumerate and charge before filtering, then
// feed captured control bytes to the dependency's public pattern policy.
fn bounded_dir(
    reader: &RootedCorpus,
    dir: &Path,
    config: &WalkConfig,
    budget: &mut WalkBudget,
    ancestors: &mut Vec<[Gitignore; 2]>,
    state: &mut VisitState<impl FnMut(SourceFile)>,
) -> Result<(), CorpusError> {
    if ancestors.len() == MAX_DEPTH {
        return Err(CorpusError::CaptureBudget("depth"));
    }
    let root = reader.root_path();
    let children = directory_children(reader, dir, budget)?;
    ancestors.push([
        capture_ignore(reader, dir, ".ontodocignore", budget)?,
        capture_ignore(reader, dir, ".gitignore", budget)?,
    ]);
    for (name, path, kind) in children {
        let mut decision = ignore::Match::None;
        // Custom controls outrank git controls even across directory depths.
        'families: for family in 0..2 {
            for frame in ancestors.iter().rev() {
                let matcher = &frame[family];
                charge(&mut budget.matching, matcher.len().max(1), "matching")?;
                decision = matcher.matched(&path, kind == FileType::Directory);
                if !decision.is_none() {
                    break 'families;
                }
            }
        }
        if decision.is_ignore() || (decision.is_none() && name.as_bytes().starts_with(b".")) {
            continue;
        }
        if kind == FileType::Directory {
            bounded_dir(reader, &path, config, budget, ancestors, state)?;
        } else {
            collect_rooted_file(reader, root, &path, kind, config, state, budget)?;
        }
    }
    ancestors.pop();
    Ok(())
}

// Keep the fixed scratch buffer out of recursive traversal frames.
#[inline(never)]
fn directory_children(
    reader: &RootedCorpus,
    dir: &Path,
    budget: &mut WalkBudget,
) -> Result<Vec<(OsString, PathBuf, FileType)>, CorpusError> {
    let relative = dir
        .strip_prefix(reader.root_path())
        .map_err(|_| CorpusError::InvalidPath)?;
    let fd = reader.open_dir(relative)?;
    directory_children_fd(reader, dir, budget, fd)
}

#[inline(never)]
fn directory_children_fd(
    reader: &RootedCorpus,
    dir: &Path,
    budget: &mut WalkBudget,
    fd: OwnedFd,
) -> Result<Vec<(OsString, PathBuf, FileType)>, CorpusError> {
    let mut buffer = [MaybeUninit::uninit(); 64 * 1024];
    let mut entries = RawDir::new(fd, &mut buffer);
    let mut children = Vec::new();
    loop {
        charge(&mut budget.entries, 1, "entry")?;
        let Some(entry) = entries.next() else { break };
        let entry = entry.map_err(enumeration_error)?;
        let name = entry.file_name().to_bytes();
        charge(
            &mut budget.paths,
            3 * (dir.as_os_str().len() + 1 + name.len()),
            "path",
        )?;
        if name == b"." || name == b".." {
            continue;
        }
        let name = std::ffi::OsStr::from_bytes(name).to_os_string();
        let path = dir.join(&name);
        let kind = entry_kind(
            reader,
            path.strip_prefix(reader.root_path())
                .map_err(|_| CorpusError::InvalidPath)?,
            entry.file_type(),
        )?;
        children.push((name, path, kind));
    }
    children.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(children)
}

fn entry_kind(
    reader: &RootedCorpus,
    relative: &Path,
    kind: FileType,
) -> Result<FileType, CorpusError> {
    if kind != FileType::Unknown {
        return Ok(kind);
    }
    let fd = reader.open_relative(relative, OFlags::PATH)?;
    let stat = rustix::fs::fstat(fd).map_err(resolution_error)?;
    Ok(FileType::from_raw_mode(stat.st_mode))
}

fn collect_rooted_file(
    reader: &RootedCorpus,
    root: &Path,
    path: &Path,
    kind: FileType,
    config: &WalkConfig,
    state: &mut VisitState<impl FnMut(SourceFile)>,
    budget: &mut WalkBudget,
) -> Result<(), CorpusError> {
    if kind == FileType::Symlink {
        state.skipped.push(Skip {
            path: lossy_rel(root, path),
            reason: SkipReason::Symlink,
        });
        return Ok(());
    }
    if kind != FileType::RegularFile || !is_markdown(path) {
        return Ok(());
    }
    let Some(rel_path) = rel_slash_path(root, path) else {
        state.skipped.push(Skip {
            path: lossy_rel(root, path),
            reason: SkipReason::NonUtf8Path,
        });
        return Ok(());
    };
    let reserve = usize::try_from(config.max_file_bytes)
        .ok()
        .and_then(|cap| cap.checked_add(1))
        .ok_or(CorpusError::CaptureBudget("byte"))?;
    charge(&mut budget.bytes, reserve, "byte")?;
    match reader.read_regular(Path::new(&rel_path), config.max_file_bytes) {
        Ok((meta, bytes)) => {
            budget.bytes += reserve - bytes.len() - 1;
            match String::from_utf8(bytes) {
                Ok(text) => {
                    if state.accepted == config.max_files {
                        return Err(CorpusError::TooManyFiles {
                            cap: config.max_files,
                        });
                    }
                    state.accepted += 1;
                    (state.on_file)(SourceFile {
                        rel_path,
                        size: meta.len(),
                        mtime_ns: mtime_ns(&meta),
                        sha256: sha256_hex(text.as_bytes()),
                        text,
                    });
                }
                Err(_) => state.skipped.push(Skip {
                    path: rel_path,
                    reason: SkipReason::NotUtf8,
                }),
            }
        }
        Err(CorpusError::TooLarge { size, cap }) => state.skipped.push(Skip {
            path: rel_path,
            reason: SkipReason::TooLarge { size, cap },
        }),
        Err(CorpusError::Symlink) => state.skipped.push(Skip {
            path: rel_path,
            reason: SkipReason::Symlink,
        }),
        Err(CorpusError::NotRegular | CorpusError::NotFound) => state.skipped.push(Skip {
            path: rel_path,
            reason: SkipReason::Changed,
        }),
        Err(CorpusError::Io { source }) => state.skipped.push(Skip {
            path: rel_path,
            reason: SkipReason::Io {
                message: source.to_string(),
            },
        }),
        Err(error) => return Err(error),
    }
    Ok(())
}

fn capture_ignore(
    reader: &RootedCorpus,
    dir: &Path,
    name: &str,
    budget: &mut WalkBudget,
) -> Result<Gitignore, CorpusError> {
    capture_ignore_after_preflight(reader, dir, name, budget, || {})
}

fn capture_ignore_after_preflight(
    reader: &RootedCorpus,
    dir: &Path,
    name: &str,
    budget: &mut WalkBudget,
    after_preflight: impl FnOnce(),
) -> Result<Gitignore, CorpusError> {
    charge(
        &mut budget.paths,
        dir.as_os_str().len() + 1 + name.len(),
        "path",
    )?;
    let path = dir.join(name);
    let relative = path
        .strip_prefix(reader.root_path())
        .map_err(|_| CorpusError::InvalidPath)?;
    // A missing control consumes no byte reservation. This metadata is only a
    // preflight; the subsequent rooted open independently checks its descriptor.
    let fd = match reader.open_relative(relative, OFlags::PATH) {
        Ok(fd) => fd,
        Err(CorpusError::NotFound) => return Ok(Gitignore::empty()),
        Err(CorpusError::Symlink) => return Err(CorpusError::UnsafeControl(path)),
        Err(error) => return Err(error),
    };
    let stat = rustix::fs::fstat(fd).map_err(resolution_error)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(CorpusError::UnsafeControl(path));
    }
    if stat.st_size > MAX_IGNORE_BYTES as i64 {
        return Err(CorpusError::CaptureBudget("ignore file byte"));
    }
    charge(&mut budget.bytes, MAX_IGNORE_BYTES + 1, "byte")?;
    after_preflight();
    let bytes = match reader.read_regular(relative, MAX_IGNORE_BYTES as u64) {
        Ok((_, bytes)) => bytes,
        Err(CorpusError::NotFound) => return Err(CorpusError::FilesystemChanged),
        Err(CorpusError::Symlink | CorpusError::NotRegular) => {
            return Err(CorpusError::UnsafeControl(path));
        }
        Err(CorpusError::TooLarge { .. }) => {
            return Err(CorpusError::CaptureBudget("ignore file byte"));
        }
        Err(CorpusError::Io { source }) => return Err(CorpusError::Control { path, source }),
        Err(error) => return Err(error),
    };
    budget.bytes += MAX_IGNORE_BYTES - bytes.len();
    // build() retains a clone while the builder is still alive.
    charge(&mut budget.paths, 2 * dir.as_os_str().len(), "path")?;
    let mut builder = GitignoreBuilder::new(dir);
    for (i, raw) in bytes.split_inclusive(|b| *b == b'\n').enumerate() {
        charge(&mut budget.lines, 1, "ignore line count")?;
        let line = if let Some(line) = raw.strip_suffix(b"\n") {
            line.strip_suffix(b"\r").unwrap_or(line)
        } else {
            raw
        };
        if line.len() > MAX_IGNORE_LINE {
            return Err(CorpusError::CaptureBudget("ignore line byte"));
        }
        let Ok(line) = std::str::from_utf8(line) else {
            break;
        };
        let line = if i == 0 {
            line.trim_start_matches('\u{feff}')
        } else {
            line
        };
        // Preserve valid rules around malformed globs and before invalid UTF-8,
        // matching the ordinary loader's partial-error policy.
        charge(&mut budget.paths, 2 * path.as_os_str().len(), "path")?;
        let _ = builder.add_line(Some(path.clone()), line);
    }
    Ok(builder.build().unwrap_or_else(|_| Gitignore::empty()))
}

#[cfg(test)]
fn open_regular(path: &Path) -> std::io::Result<File> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    // Linux ABI: final-component no-follow and nonblocking before descriptor
    // inspection. Ancestor containment remains the OD-3.4 contract, not this one.
    const O_NOFOLLOW: i32 = 0o400000;
    const O_NONBLOCK: i32 = 0o4000;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

/// Reads a regular file without following a symlink swapped in after the scan.
#[cfg(test)]
fn read_file(path: &Path, cap: u64) -> Result<(Metadata, Vec<u8>), SkipReason> {
    let io = |e: std::io::Error| SkipReason::Io {
        message: e.to_string(),
    };
    let before = std::fs::symlink_metadata(path).map_err(io)?;
    if before.file_type().is_symlink() {
        return Err(SkipReason::Symlink);
    }
    if before.len() > cap {
        return Err(SkipReason::TooLarge {
            size: before.len(),
            cap,
        });
    }
    let file = open_regular(path).map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if !meta.is_file() {
        return Err(SkipReason::Changed);
    }
    if meta.dev() != before.dev() || meta.ino() != before.ino() {
        return Err(SkipReason::Changed);
    }
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    let read = bytes.len() as u64;
    if read > cap {
        return Err(SkipReason::TooLarge { size: read, cap });
    }
    if read != meta.len() {
        return Err(SkipReason::Changed);
    }
    Ok((meta, bytes))
}

fn mtime_ns(meta: &Metadata) -> i64 {
    meta.mtime()
        .saturating_mul(1_000_000_000)
        .saturating_add(meta.mtime_nsec())
}

fn rel_slash_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts = rel
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?;
    Some(parts.join("/"))
}

fn lossy_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
fn error_path(err: &ignore::Error) -> Option<&Path> {
    match err {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    /// Fixture tree covering every eligibility rule.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(r, "README.md", b"hello\n");
        write(r, "b/z.markdown", b"# z\n");
        write(r, "b/A.MD", b"# A\n");
        write(r, "a/x.md", b"# x\n");
        write(r, "notes.txt", b"not markdown\n");
        write(r, ".gitignore", b"gen/\n");
        write(r, "gen/out.md", b"# generated\n");
        write(r, "drafts/.ontodocignore", b"wip.md\n");
        write(r, "drafts/wip.md", b"# wip\n");
        write(r, "drafts/ok.md", b"# ok\n");
        write(r, ".hidden/h.md", b"# hidden\n");
        write(r, "big.md", &[b'a'; 33]);
        write(r, "latin1.md", b"caf\xe9\n");
        write(r, "outside/target.md", b"# outside\n");
        symlink(r.join("README.md"), r.join("link.md")).unwrap();
        symlink(r.join("outside"), r.join("linkdir")).unwrap();
        dir
    }

    fn config() -> WalkConfig {
        WalkConfig {
            max_file_bytes: 32,
            max_files: 100,
        }
    }

    #[test]
    fn visit_matches_collecting_manifest_and_sorted_skips() {
        let dir = fixture();
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let expected = reader
            .walk(&config(), &mut WalkBudget::new(1024 * 1024, 100))
            .unwrap();
        let mut files = Vec::new();
        let skipped = reader
            .visit(&config(), &mut WalkBudget::new(1024 * 1024, 100), |file| {
                files.push(file);
            })
            .unwrap();
        assert_eq!(sort_corpus(Corpus { files, skipped }), expected);
    }

    #[test]
    fn visitor_consumes_and_drops_each_source_before_reading_next() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.md", b"first");
        write(dir.path(), "b.md", b"before");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let mut seen = Vec::new();
        reader
            .visit(&config(), &mut WalkBudget::new(1024, 100), |file| {
                seen.push((file.rel_path.clone(), file.sha256.clone()));
                let first = file.rel_path == "a.md";
                drop(file);
                if first {
                    write(dir.path(), "b.md", b"after");
                }
            })
            .unwrap();
        assert_eq!(
            seen,
            vec![
                ("a.md".to_owned(), sha256_hex(b"first")),
                ("b.md".to_owned(), sha256_hex(b"after")),
            ]
        );
    }

    #[test]
    fn source_reserves_before_read_and_retains_detection_or_failed_reservation() {
        use rustix::io::Errno;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "x.md", b"text");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let path = reader.root_path().join("x.md");
        let mut state = VisitState {
            accepted: 0,
            skipped: Vec::new(),
            on_file: |_: SourceFile| {},
        };
        *reader.next_resolution_error.lock().unwrap() = Some(Errno::NOSYS);
        let mut budget = WalkBudget::new(32, 100);
        assert!(matches!(
            collect_rooted_file(
                &reader,
                reader.root_path(),
                &path,
                FileType::RegularFile,
                &config(),
                &mut state,
                &mut budget,
            ),
            Err(CorpusError::CaptureBudget("byte"))
        ));
        assert_eq!(budget.bytes, 32);
        assert_eq!(
            *reader.next_resolution_error.lock().unwrap(),
            Some(Errno::NOSYS)
        );
        budget.bytes = 33;
        assert!(matches!(
            collect_rooted_file(
                &reader,
                reader.root_path(),
                &path,
                FileType::RegularFile,
                &config(),
                &mut state,
                &mut budget,
            ),
            Err(CorpusError::UnsupportedFilesystem)
        ));
        assert_eq!(budget.bytes, 0);
        budget.bytes = 33;
        collect_rooted_file(
            &reader,
            reader.root_path(),
            &path,
            FileType::RegularFile,
            &config(),
            &mut state,
            &mut budget,
        )
        .unwrap();
        assert_eq!(budget.bytes, 33 - b"text".len() - 1);
        assert_eq!(state.accepted, 1);
        write(dir.path(), "x.md", &[b'x'; 33]);
        budget.bytes = 33;
        collect_rooted_file(
            &reader,
            reader.root_path(),
            &path,
            FileType::RegularFile,
            &config(),
            &mut state,
            &mut budget,
        )
        .unwrap();
        assert_eq!(budget.bytes, 0);
        assert!(matches!(
            state.skipped[0].reason,
            SkipReason::TooLarge { .. }
        ));
    }

    #[test]
    fn successful_control_retains_detection_byte_and_requires_full_reservation() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", b"*.txt\n");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let mut budget = WalkBudget::new(MAX_IGNORE_BYTES, 100);
        assert!(matches!(
            capture_ignore_after_preflight(
                &reader,
                reader.root_path(),
                ".gitignore",
                &mut budget,
                || panic!("read must not begin without cap+1 reservation"),
            ),
            Err(CorpusError::CaptureBudget("byte"))
        ));
        budget.bytes = MAX_IGNORE_BYTES + 1;
        capture_ignore(&reader, reader.root_path(), ".gitignore", &mut budget).unwrap();
        assert_eq!(budget.bytes, MAX_IGNORE_BYTES + 1 - b"*.txt\n".len() - 1);
    }

    #[test]
    fn repeated_visits_preserve_shared_residual_counters() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "x.md", b"x");
        write(dir.path(), ".gitignore", b"*.txt\n");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let mut budget = WalkBudget::new(1024 * 1024, 100);
        budget.lines = 2;
        budget.matching = 20;
        let initial_paths = budget.paths;
        for remaining_lines in [1, 0] {
            let before = (budget.bytes, budget.entries, budget.paths, budget.matching);
            reader.visit(&config(), &mut budget, drop).unwrap();
            assert_eq!(
                budget.bytes,
                before.0 - (b"x".len() + 1 + b"*.txt\n".len() + 1)
            );
            assert_eq!(budget.entries, before.1 - 5);
            assert!(budget.paths < before.2);
            assert!(budget.matching < before.3);
            assert_eq!(budget.lines, remaining_lines);
        }
        assert!(budget.paths < initial_paths);
        assert!(matches!(
            reader.visit(&config(), &mut budget, drop),
            Err(CorpusError::CaptureBudget("ignore line count"))
        ));
        let mut no_entries = WalkBudget::new(1024, 4);
        assert!(matches!(
            reader.visit(&config(), &mut no_entries, drop),
            Err(CorpusError::CaptureBudget("entry"))
        ));
        assert_eq!(no_entries.entries, 0);
    }

    #[test]
    fn local_windows_restore_excluded_allowance_on_success_and_error() {
        for fail in [false, true] {
            let mut budget = WalkBudget::new(usize::MAX, usize::MAX);
            let result = budget.with_local_limits(10, 3, |budget| {
                assert_eq!((budget.bytes, budget.entries), (10, 3));
                charge(&mut budget.bytes, 4, "byte")?;
                charge(&mut budget.entries, 2, "entry")?;
                charge(&mut budget.paths, 5, "path")?;
                charge(&mut budget.lines, 1, "ignore line count")?;
                charge(&mut budget.matching, 3, "matching")?;
                if fail {
                    charge(&mut budget.entries, 2, "entry")?;
                }
                Ok(())
            });
            assert_eq!(result.is_err(), fail);
            assert_eq!(budget.bytes, usize::MAX - 4);
            assert_eq!(budget.entries, usize::MAX - 2);
            assert_eq!(budget.paths, 16 * 1024 * 1024 - 5);
            assert_eq!(budget.lines, 4095);
            assert_eq!(budget.matching, 999_997);
        }
        let mut budget = WalkBudget::new(2, 1);
        budget
            .with_local_limits(10, 3, |budget| {
                assert_eq!((budget.bytes, budget.entries), (2, 1));
                charge(&mut budget.bytes, 2, "byte")
            })
            .unwrap();
        assert_eq!((budget.bytes, budget.entries), (0, 1));
    }

    #[test]
    fn accepted_file_cap_still_skips_oversized_and_invalid_utf8_candidates() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.md", b"accepted");
        write(dir.path(), "b.md", &[b'x'; 33]);
        write(dir.path(), "c.md", b"\xff");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let config = WalkConfig {
            max_files: 1,
            ..config()
        };
        let mut accepted = 0;
        let skips = reader
            .visit(&config, &mut WalkBudget::new(1024, 100), |_| {
                accepted += 1;
            })
            .unwrap();
        assert_eq!(accepted, 1);
        assert!(matches!(skips[0].reason, SkipReason::TooLarge { .. }));
        assert_eq!(skips[1].reason, SkipReason::NotUtf8);
        write(dir.path(), "d.md", b"excess");
        assert!(matches!(
            reader.visit(&config, &mut WalkBudget::new(1024, 100), drop),
            Err(CorpusError::TooManyFiles { cap: 1 })
        ));
    }

    #[test]
    fn rooted_bootstrap_refuses_final_symlinks_and_normalizes_trailing_spelling() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("root");
        fs::create_dir(&root).unwrap();
        write(&root, "x.md", b"original");
        let link = parent.path().join("link");
        symlink(&root, &link).unwrap();
        for path in [
            link.clone(),
            link.join("."),
            PathBuf::from(format!("{}/", link.display())),
        ] {
            assert!(matches!(
                RootedCorpus::open(&path),
                Err(CorpusError::Symlink)
            ));
        }
        let reader = RootedCorpus::open(&root.join(".")).unwrap();
        assert_eq!(reader.root_path(), root.canonicalize().unwrap());
        assert_eq!(
            reader.read_regular(Path::new("x.md"), 32).unwrap().1,
            b"original"
        );
        let parent_link = parent.path().join("parent-link");
        symlink(parent.path(), &parent_link).unwrap();
        assert!(RootedCorpus::open(&parent_link.join("root")).is_ok());
    }

    #[test]
    fn retained_root_survives_rename_and_walks_start_with_fresh_offsets() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("root");
        write(&root, "x.md", b"original");
        let reader = RootedCorpus::open(&root).unwrap();
        fs::rename(&root, parent.path().join("moved")).unwrap();
        write(&root, "x.md", b"replacement");
        assert_eq!(
            reader.read_regular(Path::new("x.md"), 32).unwrap().1,
            b"original"
        );
        let first = reader
            .walk(&config(), &mut WalkBudget::new(1024 * 1024, 100))
            .unwrap();
        let second = reader
            .walk(&config(), &mut WalkBudget::new(1024 * 1024, 100))
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.files[0].text, "original");
    }

    #[test]
    fn rooted_reads_refuse_non_normal_paths_and_replaced_ancestors_or_leaves() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(dir.path(), "sub/x.md", b"inside");
        write(outside.path(), "x.md", b"outside");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        for path in [
            "",
            ".",
            "..",
            "../x.md",
            "/x.md",
            "sub//x.md",
            "sub/./x.md",
            "sub/x.md/",
            "x\0.md",
        ] {
            assert!(matches!(
                reader.read_regular(Path::new(path), 32),
                Err(CorpusError::InvalidPath)
            ));
        }
        fs::rename(dir.path().join("sub"), dir.path().join("old")).unwrap();
        symlink(outside.path(), dir.path().join("sub")).unwrap();
        assert!(matches!(
            reader.read_regular(Path::new("sub/x.md"), 32),
            Err(CorpusError::Symlink)
        ));
        symlink(outside.path().join("x.md"), dir.path().join("leaf.md")).unwrap();
        assert!(matches!(
            reader.read_regular(Path::new("leaf.md"), 32),
            Err(CorpusError::Symlink)
        ));
        assert!(matches!(
            entry_kind(&reader, Path::new("leaf.md"), FileType::Unknown).unwrap(),
            FileType::Symlink
        ));
        assert_eq!(
            entry_kind(&reader, Path::new("old/x.md"), FileType::Unknown).unwrap(),
            FileType::RegularFile
        );
        assert_eq!(
            entry_kind(&reader, Path::new("old"), FileType::Unknown).unwrap(),
            FileType::Directory
        );
        fs::remove_file(dir.path().join("sub")).unwrap();
        write(dir.path(), "sub", b"not a directory");
        assert!(matches!(
            reader.read_regular(Path::new("sub/x.md"), 32),
            Err(CorpusError::NotRegular)
        ));
    }

    #[test]
    fn descriptor_read_detects_growth_after_metadata_and_checked_caps() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "x.md", b"x");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let file = File::from(
            reader
                .open_relative(Path::new("x.md"), OFlags::RDONLY)
                .unwrap(),
        );
        let result = read_descriptor(file, 2, || write(dir.path(), "x.md", b"grew"));
        assert!(matches!(
            result,
            Err(CorpusError::TooLarge { size: 3, cap: 2 })
        ));
        assert!(matches!(
            reader.read_regular(Path::new("x.md"), u64::MAX),
            Err(CorpusError::CaptureBudget("byte"))
        ));
        write(dir.path(), "x.md", b"x");
        let file = File::from(
            reader
                .open_relative(Path::new("x.md"), OFlags::RDONLY)
                .unwrap(),
        );
        assert!(matches!(
            read_descriptor(file, 32, || write(dir.path(), "x.md", b"longer")),
            Err(CorpusError::FilesystemChanged)
        ));
        let file = File::from(
            reader
                .open_relative(Path::new("x.md"), OFlags::RDONLY)
                .unwrap(),
        );
        assert!(matches!(
            read_descriptor(file, 32, || write(dir.path(), "x.md", b"x")),
            Err(CorpusError::FilesystemChanged)
        ));
        assert_eq!(reader.read_regular(Path::new("x.md"), 1).unwrap().1, b"x");
        let file = File::from(
            reader
                .open_relative(Path::new("x.md"), OFlags::RDONLY)
                .unwrap(),
        );
        let result = read_descriptor(file, 1, || {
            fs::rename(dir.path().join("x.md"), dir.path().join("old.md")).unwrap();
            write(dir.path(), "x.md", b"replacement");
        })
        .unwrap();
        assert_eq!(result.1, b"x");
    }

    #[test]
    fn rooted_errors_fail_closed_and_deleted_directory_is_not_empty() {
        use rustix::io::Errno;
        for error in [Errno::NOSYS, Errno::PERM, Errno::INVAL] {
            assert!(matches!(
                resolution_error(error),
                CorpusError::UnsupportedFilesystem
            ));
        }
        for error in [Errno::AGAIN, Errno::XDEV] {
            assert!(matches!(
                resolution_error(error),
                CorpusError::FilesystemChanged
            ));
        }
        assert!(matches!(
            resolution_error(Errno::LOOP),
            CorpusError::Symlink
        ));
        assert!(matches!(
            resolution_error(Errno::NOENT),
            CorpusError::NotFound
        ));
        assert!(matches!(
            resolution_error(Errno::ACCESS),
            CorpusError::Io { .. }
        ));
        assert!(matches!(
            enumeration_error(Errno::NOENT),
            CorpusError::FilesystemChanged
        ));
        assert!(matches!(
            enumeration_error(Errno::INVAL),
            CorpusError::Io { .. }
        ));
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("gone")).unwrap();
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let fd = reader.open_dir(Path::new("gone")).unwrap();
        fs::remove_dir(dir.path().join("gone")).unwrap();
        let mut budget = WalkBudget::new(1024, 10);
        assert!(matches!(
            directory_children_fd(&reader, &dir.path().join("gone"), &mut budget, fd),
            Err(CorpusError::FilesystemChanged)
        ));
        assert_eq!(budget.entries, 9);
    }

    #[test]
    fn rooted_enumeration_charges_dot_and_terminal_probes() {
        let dir = tempfile::tempdir().unwrap();
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let mut budget = WalkBudget::new(1024, 3);
        assert!(
            directory_children(&reader, reader.root_path(), &mut budget)
                .unwrap()
                .is_empty()
        );
        assert_eq!(budget.entries, 0);
        let mut budget = WalkBudget::new(1024, 2);
        assert!(matches!(
            directory_children(&reader, reader.root_path(), &mut budget),
            Err(CorpusError::CaptureBudget("entry"))
        ));
    }

    #[test]
    fn disappeared_control_refuses_without_refunding_failed_read() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", b"hidden.md\n");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        let mut budget = WalkBudget::new(100_000, 100);
        let result = capture_ignore_after_preflight(
            &reader,
            reader.root_path(),
            ".gitignore",
            &mut budget,
            || {
                fs::remove_file(dir.path().join(".gitignore")).unwrap();
            },
        );
        assert!(matches!(result, Err(CorpusError::FilesystemChanged)));
        assert_eq!(budget.bytes, 100_000 - MAX_IGNORE_BYTES - 1);
    }

    #[test]
    fn resolver_refusals_propagate_without_retry_or_empty_walk() {
        use rustix::io::Errno;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "x.md", b"text");
        let reader = RootedCorpus::open(dir.path()).unwrap();
        for error in [Errno::NOSYS, Errno::PERM, Errno::AGAIN] {
            *reader.next_resolution_error.lock().unwrap() = Some(error);
            let mut budget = WalkBudget::new(100_000, 100);
            let result = reader.walk(&config(), &mut budget);
            assert!(if error == Errno::AGAIN {
                matches!(result, Err(CorpusError::FilesystemChanged))
            } else {
                matches!(result, Err(CorpusError::UnsupportedFilesystem))
            });
            assert_eq!(budget.entries, 100);
            // The fault is one-shot: a retry would have succeeded, not refused.
            assert_eq!(
                reader.read_regular(Path::new("x.md"), 4).unwrap().1,
                b"text"
            );
        }
    }

    #[test]
    fn enumerated_ancestor_moved_outside_root_is_never_used_for_child_opens() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("root");
        write(&root, "sub/x.md", b"original");
        let reader = RootedCorpus::open(&root).unwrap();
        let mut budget = WalkBudget::new(100_000, 100);
        let logical_sub = reader.root_path().join("sub");
        let enumerated = directory_children(&reader, &logical_sub, &mut budget).unwrap();
        fs::rename(root.join("sub"), parent.path().join("outside")).unwrap();
        symlink(parent.path().join("outside"), root.join("sub")).unwrap();
        assert_eq!(enumerated.len(), 1);
        let path = enumerated[0].1.strip_prefix(reader.root_path()).unwrap();
        assert!(matches!(
            reader.read_regular(path, 32),
            Err(CorpusError::Symlink)
        ));
        assert!(matches!(
            entry_kind(&reader, path, FileType::Unknown),
            Err(CorpusError::Symlink)
        ));
        assert!(matches!(
            capture_ignore(&reader, &logical_sub, ".gitignore", &mut budget),
            Err(CorpusError::UnsafeControl(_))
        ));
    }

    #[test]
    fn bounded_walk_preserves_results_and_refuses_bytes_or_entry_overflow() {
        let dir = fixture();
        let mut budget = WalkBudget::new(1024 * 1024, 100);
        assert_eq!(
            walk_bounded(dir.path(), &config(), &mut budget).unwrap(),
            walk_inner(dir.path(), &config()).unwrap()
        );
        assert!(budget.bytes < 1024 * 1024);
        let mut no_bytes = WalkBudget::new(0, 100);
        assert!(matches!(
            walk_bounded(dir.path(), &config(), &mut no_bytes),
            Err(CorpusError::CaptureBudget("byte"))
        ));
        let mut no_entries = WalkBudget::new(4096, 0);
        assert!(matches!(
            walk_bounded(dir.path(), &config(), &mut no_entries),
            Err(CorpusError::CaptureBudget("entry"))
        ));
    }

    fn bounded(root: &Path) -> Result<Corpus, CorpusError> {
        walk_bounded(root, &config(), &mut WalkBudget::new(1024 * 1024, 10_000))
    }

    #[test]
    fn bounded_ignore_policy_matches_ordinary_walker() {
        let dir = fixture();
        let r = dir.path();
        write(r, ".ontodocignore", b"!root-wins.md\ncustom-drop.md\n");
        write(
            r,
            ".gitignore",
            b"gen/\npruned/\n!.hidden.md\n!.visible/\n*.tmp.md\n",
        );
        write(
            r,
            "sub/.gitignore",
            b"root-wins.md\n!custom-drop.md\n!keep.tmp.md\n/anchored.md\nonlydir/\n",
        );
        write(r, "sub/root-wins.md", b"root custom whitelist\n");
        write(r, "sub/custom-drop.md", b"root custom exclude\n");
        write(r, "sub/keep.tmp.md", b"nested whitelist\n");
        write(r, "sub/drop.tmp.md", b"excluded\n");
        write(r, "sub/anchored.md", b"excluded\n");
        write(r, "sub/deeper/anchored.md", b"kept\n");
        write(r, "sub/onlydir/x.md", b"excluded\n");
        write(r, "sub/.git/HEAD", b"ref: refs/heads/main\n");
        write(r, "sub/.ignore", b"deeper/\n");
        write(r, ".visible/x.md", b"visible hidden directory\n");
        write(r, ".hidden.md", b"visible\n");
        write(r, "pruned/.ontodocignore", b"!x.md\n");
        write(r, "pruned/x.md", b"cannot rescue ignored parent\n");
        let ordinary = walk_inner(r, &config()).unwrap();
        assert_eq!(bounded(r).unwrap(), ordinary);
        let paths: Vec<_> = ordinary.files.iter().map(|f| f.rel_path.as_str()).collect();
        assert!(paths.contains(&"sub/root-wins.md"));
        assert!(paths.contains(&"sub/keep.tmp.md"));
        assert!(paths.contains(&".visible/x.md"));
        assert!(paths.contains(&".hidden.md"));
        assert!(!paths.contains(&"sub/custom-drop.md"));
        assert!(!paths.contains(&"pruned/x.md"));
    }

    #[test]
    fn bounded_control_decoding_and_partial_rules_match_ordinary_walker() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        assert!(GitignoreBuilder::new(r).add_line(None, "[z-a]").is_err());
        write(r, ".gitignore", b"\xef\xbb\xbfdrop.md\r\nlast.md\r\n!last.md\r\n\\#hash.md\n\\!bang.md\ntrailing\\ .md\n[z-a]\nkept-after-invalid.md\nprefix.md\n\xff\nnot-read.md\n");
        for name in [
            "drop.md",
            "last.md",
            "#hash.md",
            "!bang.md",
            "trailing .md",
            "kept-after-invalid.md",
            "prefix.md",
            "not-read.md",
        ] {
            write(r, name, b"x\n");
        }
        let ordinary = walk_inner(r, &config()).unwrap();
        assert_eq!(bounded(r).unwrap(), ordinary);
        let paths: Vec<_> = ordinary.files.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["last.md", "not-read.md"]);
    }

    #[test]
    fn bounded_counts_ignored_and_hidden_entries_before_filtering() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", b"*.md\n");
        for i in 0..20 {
            write(dir.path(), &format!("{i}.md"), b"ignored\n");
            write(dir.path(), &format!(".hidden-{i}"), b"hidden\n");
        }
        assert!(walk(dir.path(), &config()).unwrap().files.is_empty());
        let mut budget = WalkBudget::new(1024 * 1024, 10);
        assert!(matches!(
            walk_bounded(dir.path(), &config(), &mut budget),
            Err(CorpusError::CaptureBudget("entry"))
        ));
        assert_eq!(budget.entries, 0);
    }

    #[test]
    fn bounded_ignores_parent_and_dot_ignore_policies() {
        let parent = tempfile::tempdir().unwrap();
        write(parent.path(), ".gitignore", b"*.md\n");
        write(parent.path(), "root/.ignore", b"*.md\n");
        write(parent.path(), "root/kept.md", b"kept\n");
        let root = parent.path().join("root");
        let ordinary = walk_inner(&root, &config()).unwrap();
        assert_eq!(bounded(&root).unwrap(), ordinary);
        assert_eq!(ordinary.files[0].rel_path, "kept.md");
    }

    #[test]
    fn bounded_controls_refuse_byte_line_count_and_unsafe_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(r, ".gitignore", &vec![b'a'; MAX_IGNORE_BYTES + 1]);
        assert!(matches!(
            bounded(r),
            Err(CorpusError::CaptureBudget("ignore file byte"))
        ));
        write(r, ".gitignore", &vec![b'a'; MAX_IGNORE_LINE + 1]);
        assert!(matches!(
            bounded(r),
            Err(CorpusError::CaptureBudget("ignore line byte"))
        ));
        write(r, ".gitignore", &vec![b'\n'; 4097]);
        assert!(matches!(
            bounded(r),
            Err(CorpusError::CaptureBudget("ignore line count"))
        ));
        fs::remove_file(r.join(".gitignore")).unwrap();
        let external = tempfile::tempdir().unwrap();
        write(external.path(), "control", b"*.md\n");
        symlink(external.path().join("control"), r.join(".gitignore")).unwrap();
        assert!(matches!(bounded(r), Err(CorpusError::UnsafeControl(_))));
        fs::remove_file(r.join(".gitignore")).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(r.join(".gitignore"))
                .status()
                .unwrap()
                .success()
        );
        assert!(matches!(bounded(r), Err(CorpusError::UnsafeControl(_))));
    }

    #[test]
    fn bounded_prunes_before_loading_controls_and_caps_depth_paths_and_matching() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(r, ".gitignore", b"pruned/\n");
        write(r, "pruned/.gitignore", &vec![b'a'; MAX_IGNORE_BYTES + 1]);
        assert_eq!(bounded(r).unwrap(), walk_inner(r, &config()).unwrap());
        write(r, "x.md", b"x\n");
        let mut paths = WalkBudget::new(1024 * 1024, 100);
        paths.paths = 0;
        assert!(matches!(
            walk_bounded(r, &config(), &mut paths),
            Err(CorpusError::CaptureBudget("path"))
        ));
        let mut matching = WalkBudget::new(1024 * 1024, 100);
        matching.matching = 0;
        assert!(matches!(
            walk_bounded(r, &config(), &mut matching),
            Err(CorpusError::CaptureBudget("matching"))
        ));
        let mut deep = r.to_path_buf();
        for _ in 0..MAX_DEPTH {
            deep.push("d");
        }
        fs::create_dir_all(deep).unwrap();
        assert!(matches!(
            bounded(r),
            Err(CorpusError::CaptureBudget("depth"))
        ));
    }

    #[test]
    fn walk_returns_sorted_eligible_files_and_typed_skips() {
        let dir = fixture();
        let corpus = walk(dir.path(), &config()).unwrap();

        let paths: Vec<&str> = corpus.files.iter().map(|f| f.rel_path.as_str()).collect();
        // gen/ (.gitignore), drafts/wip.md (.ontodocignore), .hidden/, notes.txt,
        // symlinks, big.md and latin1.md are excluded.
        assert_eq!(
            paths,
            vec![
                "README.md",
                "a/x.md",
                "b/A.MD",
                "b/z.markdown",
                "drafts/ok.md",
                "outside/target.md",
            ]
        );
        assert_eq!(
            corpus.skipped,
            vec![
                Skip {
                    path: "big.md".into(),
                    reason: SkipReason::TooLarge { size: 33, cap: 32 },
                },
                Skip {
                    path: "latin1.md".into(),
                    reason: SkipReason::NotUtf8,
                },
                Skip {
                    path: "link.md".into(),
                    reason: SkipReason::Symlink,
                },
                Skip {
                    path: "linkdir".into(),
                    reason: SkipReason::Symlink,
                },
            ]
        );
    }

    #[test]
    fn walk_records_size_mtime_and_sha256() {
        let dir = fixture();
        let corpus = walk(dir.path(), &config()).unwrap();
        let readme = &corpus.files[0];
        let meta = fs::metadata(dir.path().join("README.md")).unwrap();
        assert_eq!(readme.rel_path, "README.md");
        assert_eq!(readme.size, 6);
        assert_eq!(readme.text, "hello\n");
        // sha256("hello\n"), from `printf 'hello\n' | sha256sum`.
        assert_eq!(
            readme.sha256,
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        );
        assert_eq!(
            readme.mtime_ns,
            meta.mtime() * 1_000_000_000 + meta.mtime_nsec()
        );
    }

    #[test]
    fn walk_is_byte_stable_across_runs() {
        let dir = fixture();
        let first = walk(dir.path(), &config()).unwrap();
        let second = walk(dir.path(), &config()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn walk_fails_when_file_count_exceeds_cap() {
        let dir = fixture();
        let cfg = WalkConfig {
            max_files: 2,
            ..config()
        };
        assert!(matches!(
            walk(dir.path(), &cfg),
            Err(CorpusError::TooManyFiles { cap: 2 })
        ));
    }

    #[test]
    fn walk_rejects_missing_root() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            walk(&dir.path().join("missing"), &config()),
            Err(CorpusError::Root { .. })
        ));
    }
}
