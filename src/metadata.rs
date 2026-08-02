//! Entry metadata types shared by listing and search.

use std::path::PathBuf;
use std::time::SystemTime;

/// Classification of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link (when not followed).
    Symlink,
    /// Other (socket, fifo, device, unknown).
    Other,
}

/// A discovered filesystem entry.
///
/// Metadata fields are best-effort: permission errors or exotic filesystems
/// may leave some fields as `None` without failing the whole traversal when
/// the error policy allows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Absolute or walk-produced path of the entry.
    pub path: PathBuf,
    /// Path relative to the walk root, when known.
    pub relative_path: Option<PathBuf>,
    /// Entry kind classification.
    pub kind: EntryKind,
    /// File size in bytes, when available.
    pub size: Option<u64>,
    /// Last modification time, when available.
    pub modified: Option<SystemTime>,
    /// Read-only flag, when available.
    pub readonly: Option<bool>,
}

impl FileEntry {
    /// Create a minimal entry with the given path and kind.
    pub fn new(path: PathBuf, kind: EntryKind) -> Self {
        Self {
            path,
            relative_path: None,
            kind,
            size: None,
            modified: None,
            readonly: None,
        }
    }
}

/// Deterministic sort mode for listing and search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SortMode {
    /// Do not sort (walk order).
    None,
    /// Sort by full path (lexicographic on the platform's OsStr ordering via lossy UTF-8 for stability).
    #[default]
    Path,
    /// Sort by file name only.
    Name,
    /// Directories first, then files, then by path.
    DirsFirst,
}

/// How to handle I/O errors during traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TraversalErrorPolicy {
    /// Abort on the first error.
    #[default]
    FailFast,
    /// Skip entries that produce errors and continue.
    SkipErrors,
}

/// Apply sorting to a list of entries according to `mode`.
pub fn sort_entries(entries: &mut [FileEntry], mode: SortMode) {
    match mode {
        SortMode::None => {}
        SortMode::Path => {
            entries.sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
        }
        SortMode::Name => {
            entries.sort_by(|a, b| {
                let an = a
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                let bn = b
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                an.cmp(&bn)
                    .then_with(|| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()))
            });
        }
        SortMode::DirsFirst => {
            entries.sort_by(|a, b| {
                let ak = kind_rank(a.kind);
                let bk = kind_rank(b.kind);
                ak.cmp(&bk)
                    .then_with(|| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()))
            });
        }
    }
}

fn kind_rank(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::Directory => 0,
        EntryKind::File => 1,
        EntryKind::Symlink => 2,
        EntryKind::Other => 3,
    }
}
