//! Directory listing and recursive traversal.
//!
//! Enabled by the `listing` feature (default).
//!
//! # Depth model
//!
//! Depth is measured the same way as [`walkdir`](https://docs.rs/walkdir) and
//! [`crate::discovery::DiscoveryOptions`]:
//!
//! - **Depth 0** — the walk root itself
//! - **Depth 1** — immediate children of the root
//! - **Depth *n*** — *n* path components below the root
//!
//! | `recursive` | `max_depth` | Effect |
//! | --- | --- | --- |
//! | `false` | ignored | Only immediate children (depth 1). Optionally include root via `include_root`. |
//! | `true` | `None` | Unlimited depth |
//! | `true` | `Some(n)` | Walkdir `max_depth = n` (root is depth 0) |
//!
//! Prefer [`list`] when you need sorted results. Prefer [`walk`] for streaming
//! (unsorted walk order).

use crate::error::PathError;
use crate::internal::validation::{is_hidden_name, reject_nul_path};
use crate::metadata::{EntryKind, FileEntry, SortMode, TraversalErrorPolicy, sort_entries};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Options controlling directory listing and traversal.
///
/// # Defaults
///
/// - Non-recursive (immediate children only)
/// - Do not follow symlinks
/// - Include files and directories
/// - Include symlinks as entries (not followed)
/// - Exclude hidden entries
/// - Fail-fast on errors
/// - Sort by path
///
/// See module docs for the unified depth model shared with discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOptions {
    /// Recurse into subdirectories.
    pub recursive: bool,
    /// Follow symbolic links (cycle detection is best-effort via walkdir).
    pub follow_symlinks: bool,
    /// Include regular files.
    pub include_files: bool,
    /// Include directories.
    pub include_directories: bool,
    /// Include symlink entries (when not followed into).
    pub include_symlinks: bool,
    /// Include hidden entries (dotfiles; Windows hidden attribute when available).
    pub include_hidden: bool,
    /// Include other entry types (sockets, devices, etc.).
    pub include_other: bool,
    /// Maximum walk depth when `recursive` is true (`None` = unlimited).
    ///
    /// Depth 0 is the root; depth 1 is immediate children. See module docs.
    /// Ignored when `recursive` is false (only depth-1 children are listed).
    pub max_depth: Option<usize>,
    /// Maximum number of returned entries.
    pub max_entries: Option<usize>,
    /// Result sort mode (`list` only; [`walk`] always yields in walk order).
    pub sort: SortMode,
    /// Error handling policy.
    pub error_policy: TraversalErrorPolicy,
    /// When true, include the root directory itself as an entry.
    pub include_root: bool,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            follow_symlinks: false,
            include_files: true,
            include_directories: true,
            include_symlinks: true,
            include_hidden: false,
            include_other: false,
            max_depth: None,
            max_entries: None,
            sort: SortMode::Path,
            error_policy: TraversalErrorPolicy::FailFast,
            include_root: false,
        }
    }
}

impl ListOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable recursion.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Follow symlinks when walking.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Include regular files.
    pub fn include_files(mut self, include: bool) -> Self {
        self.include_files = include;
        self
    }

    /// Include directories.
    pub fn include_directories(mut self, include: bool) -> Self {
        self.include_directories = include;
        self
    }

    /// Include symlink entries.
    pub fn include_symlinks(mut self, include: bool) -> Self {
        self.include_symlinks = include;
        self
    }

    /// Include other entry types.
    pub fn include_other(mut self, include: bool) -> Self {
        self.include_other = include;
        self
    }

    /// Include hidden files and directories.
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Set maximum depth (see module depth model).
    pub fn max_depth(mut self, depth: Option<usize>) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set maximum number of entries.
    pub fn max_entries(mut self, max: Option<usize>) -> Self {
        self.max_entries = max;
        self
    }

    /// Set sort mode (applies to [`list`], not [`walk`]).
    pub fn sort(mut self, sort: SortMode) -> Self {
        self.sort = sort;
        self
    }

    /// Set error policy.
    pub fn error_policy(mut self, policy: TraversalErrorPolicy) -> Self {
        self.error_policy = policy;
        self
    }

    /// Include the root directory itself.
    pub fn include_root(mut self, include: bool) -> Self {
        self.include_root = include;
        self
    }
}

/// Walkdir depth bounds for listing.
///
/// - Non-recursive: only depth 1 (children). Root is handled separately via `include_root`.
/// - Recursive: depth 0..=max (or unlimited). `min_depth` is 0 when including root in the
///   walker, else 1 when root is emitted separately or omitted.
pub(crate) fn walk_depth_bounds(options: &ListOptions) -> (usize, usize) {
    if options.recursive {
        let max = options.max_depth.unwrap_or(usize::MAX);
        let min = if options.include_root { 0 } else { 1 };
        // If max is 0 and we only want root, min must be 0.
        let min = min.min(max);
        (min, max)
    } else {
        // Immediate children only; root is optional via include_root (emitted separately
        // or at depth 0 when include_root is true).
        if options.include_root { (0, 1) } else { (1, 1) }
    }
}

/// List filesystem entries under `root` according to `options`.
///
/// Results are sorted when `options.sort` is not [`SortMode::None`].
///
/// # Filesystem access
///
/// **Yes.** Requires `root` to exist. Does not follow symlinks by default.
/// Does not read file contents.
///
/// # Security
///
/// Always set `max_depth` / `max_entries` for untrusted roots.
pub fn list(root: impl AsRef<Path>, options: &ListOptions) -> Result<Vec<FileEntry>, PathError> {
    let mut entries = Vec::new();
    for item in walk(root, options)? {
        entries.push(item?);
        if let Some(max) = options.max_entries {
            if entries.len() >= max {
                break;
            }
        }
    }
    sort_entries(&mut entries, options.sort);
    // Re-apply max after sort so sorted prefix is stable when capped mid-stream.
    if let Some(max) = options.max_entries {
        entries.truncate(max);
    }
    Ok(entries)
}

/// Stream filesystem entries under `root` in walk order.
///
/// Unlike [`list`], this is a **lazy** iterator over the directory walk:
/// entries are produced as the filesystem is traversed.
///
/// # Sorting
///
/// `options.sort` is **ignored**. Streaming cannot sort without buffering.
/// Call [`list`] when deterministic ordering is required.
///
/// # Max entries
///
/// `options.max_entries` is enforced by stopping the iterator after that many
/// yielded entries.
///
/// # Filesystem access
///
/// **Yes.** Requires `root` to exist.
pub fn walk(root: impl AsRef<Path>, options: &ListOptions) -> Result<WalkIter, PathError> {
    let root = root.as_ref();
    reject_nul_path(root)?;
    ensure_listable_root(root)?;

    let (min_depth, max_depth) = walk_depth_bounds(options);
    let walker = WalkDir::new(root)
        .min_depth(min_depth)
        .max_depth(max_depth)
        .follow_links(options.follow_symlinks)
        .same_file_system(false);

    Ok(WalkIter {
        root: root.to_path_buf(),
        options: options.clone(),
        inner: walker.into_iter(),
        yielded: 0,
    })
}

/// Streaming iterator produced by [`walk`].
///
/// Yields `Ok(FileEntry)` for matching entries, or `Err` according to the
/// configured error policy (fail-fast stops; skip continues).
pub struct WalkIter {
    root: PathBuf,
    options: ListOptions,
    inner: walkdir::IntoIter,
    yielded: usize,
}

impl Iterator for WalkIter {
    type Item = Result<FileEntry, PathError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(max) = self.options.max_entries {
            if self.yielded >= max {
                return None;
            }
        }

        loop {
            if let Some(max) = self.options.max_entries {
                if self.yielded >= max {
                    return None;
                }
            }

            let item = self.inner.next()?;
            let dir_entry = match item {
                Ok(e) => e,
                Err(err) => match self.options.error_policy {
                    TraversalErrorPolicy::FailFast => {
                        return Some(Err(PathError::traversal(err.to_string())));
                    }
                    TraversalErrorPolicy::SkipErrors => continue,
                },
            };

            match filter_and_build(&self.root, &dir_entry, &self.options) {
                Ok(Some(entry)) => {
                    self.yielded += 1;
                    return Some(Ok(entry));
                }
                Ok(None) => continue,
                Err(e) => match self.options.error_policy {
                    TraversalErrorPolicy::FailFast => return Some(Err(e)),
                    TraversalErrorPolicy::SkipErrors => continue,
                },
            }
        }
    }
}

fn ensure_listable_root(root: &Path) -> Result<(), PathError> {
    let meta = fs::symlink_metadata(root).map_err(|e| PathError::filesystem(root, e))?;
    if meta.is_dir() {
        return Ok(());
    }
    // Symlink to directory is listable when metadata follows, but symlink_metadata
    // alone is not a dir. Try followed metadata.
    if meta.file_type().is_symlink() {
        match fs::metadata(root) {
            Ok(m) if m.is_dir() => return Ok(()),
            Ok(_) => {}
            Err(e) => return Err(PathError::filesystem(root, e)),
        }
    }
    Err(PathError::invalid(format!(
        "list root is not a directory: {}",
        root.to_string_lossy()
    )))
}

fn filter_and_build(
    root: &Path,
    dir_entry: &DirEntry,
    options: &ListOptions,
) -> Result<Option<FileEntry>, PathError> {
    let path = dir_entry.path();
    let name = dir_entry.file_name();

    if name == "." || name == ".." {
        return Ok(None);
    }

    // Never treat the walk root itself as "hidden" solely because its basename
    // starts with `.` (common for tempfile directories like `.tmpXXXX`).
    let is_root = path == root;
    if !is_root && !options.include_hidden && (is_hidden_name(name) || is_windows_hidden(path)) {
        return Ok(None);
    }

    let relative = path.strip_prefix(root).ok().map(Path::to_path_buf);
    entry_from_dir_entry(path, relative, dir_entry, options)
}

fn entry_from_dir_entry(
    path: &Path,
    relative: Option<PathBuf>,
    dir_entry: &DirEntry,
    options: &ListOptions,
) -> Result<Option<FileEntry>, PathError> {
    let file_type = dir_entry.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };

    if !kind_allowed(kind, options) {
        return Ok(None);
    }

    let (size, modified, readonly) = match fs::symlink_metadata(path) {
        Ok(meta) => {
            let size = if meta.is_file() {
                Some(meta.len())
            } else {
                None
            };
            let modified = meta.modified().ok();
            let readonly = Some(meta.permissions().readonly());
            (size, modified, readonly)
        }
        Err(e) => match options.error_policy {
            TraversalErrorPolicy::FailFast => {
                return Err(PathError::filesystem(path, e));
            }
            TraversalErrorPolicy::SkipErrors => (None, None, None),
        },
    };

    Ok(Some(FileEntry {
        path: path.to_path_buf(),
        relative_path: relative,
        kind,
        size,
        modified,
        readonly,
    }))
}

fn kind_allowed(kind: EntryKind, options: &ListOptions) -> bool {
    match kind {
        EntryKind::File => options.include_files,
        EntryKind::Directory => options.include_directories,
        EntryKind::Symlink => options.include_symlinks,
        EntryKind::Other => options.include_other,
    }
}

#[cfg(windows)]
fn is_windows_hidden(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    fs::symlink_metadata(path)
        .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_windows_hidden(_path: &Path) -> bool {
    false
}

#[cfg(feature = "async")]
/// Async wrapper around [`list`] using `spawn_blocking`.
pub async fn list_async(
    root: impl AsRef<Path> + Send + 'static,
    options: ListOptions,
) -> Result<Vec<FileEntry>, PathError> {
    let root = root.as_ref().to_path_buf();
    tokio::task::spawn_blocking(move || list(root, &options))
        .await
        .map_err(|e| PathError::traversal(format!("async listing join failed: {e}")))?
}
