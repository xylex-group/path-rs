//! Directory existence and inspection helpers.
//!
//! These APIs intentionally document symlink and metadata semantics so callers
//! do not rely on ambiguous convenience methods alone.

use crate::error::PathError;
use crate::internal::validation::reject_nul_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Summary of filesystem metadata without exposing full `std::fs::Metadata`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSummary {
    /// File size in bytes (for files).
    pub len: u64,
    /// Whether the entry is read-only according to permissions.
    pub readonly: bool,
    /// Modification time when available.
    pub modified: Option<SystemTime>,
}

/// Result of inspecting a path as a potential directory.
///
/// # Symlink semantics
///
/// - [`Path::is_dir`] follows symlinks.
/// - `symlink_metadata` inspects the link itself.
/// - `metadata` follows symlinks.
/// - Broken symlinks are not directories.
/// - Permission errors may surface as `exists: false` only in convenience
///   booleans; prefer [`inspect_directory`] / [`require_directory`] for errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryInspection {
    /// Path that was inspected.
    pub path: PathBuf,
    /// Whether the path exists (symlink target may not, if inspecting a link).
    pub exists: bool,
    /// Whether the path is a directory (symlink-to-dir counts when following).
    pub is_directory: bool,
    /// Whether the path itself is a symbolic link.
    pub is_symlink: bool,
    /// Best-effort readability probe (`None` if not checked / not applicable).
    pub is_readable: Option<bool>,
    /// Optional metadata summary.
    pub metadata: Option<MetadataSummary>,
}

/// Returns `true` if `path` exists and is a directory.
///
/// Uses `std::fs::metadata` (follows symlinks). Permission errors and missing
/// paths yield `false` — this is a convenience wrapper, not a strict probe.
///
/// Equivalent in intent to:
///
/// ```ignore
/// path.is_dir()
///     || fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
/// ```
///
/// # Filesystem access
///
/// **Yes.** Follows symlinks. Does not populate a cache.
///
/// Prefer [`inspect_directory`] when you need error context.
pub fn directory_exists(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    if path.is_dir() {
        return true;
    }
    fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
}

/// Alias for [`directory_exists`].
#[inline]
pub fn is_existing_directory(path: impl AsRef<Path>) -> bool {
    directory_exists(path)
}

/// Inspect a path without panicking on I/O errors.
///
/// Uses `symlink_metadata` for the link itself and, when the path is a symlink,
/// also checks whether the target is a directory via `metadata`.
///
/// # Filesystem access
///
/// **Yes.** Does not follow symlinks for `is_symlink`; `is_directory` is true
/// for symlink-to-directory when the target is reachable.
pub fn inspect_directory(path: impl AsRef<Path>) -> Result<DirectoryInspection, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;
    let path_buf = path.to_path_buf();

    let symlink_meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DirectoryInspection {
                path: path_buf,
                exists: false,
                is_directory: false,
                is_symlink: false,
                is_readable: None,
                metadata: None,
            });
        }
        Err(e) => return Err(PathError::filesystem(path, e)),
    };

    let is_symlink = symlink_meta.file_type().is_symlink();
    let is_directory = if is_symlink {
        fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        symlink_meta.is_dir()
    };

    let is_readable = if is_directory {
        Some(fs::read_dir(path).is_ok())
    } else {
        None
    };

    let metadata = Some(MetadataSummary {
        len: symlink_meta.len(),
        readonly: symlink_meta.permissions().readonly(),
        modified: symlink_meta.modified().ok(),
    });

    Ok(DirectoryInspection {
        path: path_buf,
        exists: true,
        is_directory,
        is_symlink,
        is_readable,
        metadata,
    })
}

/// Require that `path` exists and is a directory; return its path on success.
///
/// # Filesystem access
///
/// **Yes.** Follows symlinks for the directory check.
///
/// # Errors
///
/// Returns a contextual filesystem or invalid-path error when the path is
/// missing, not a directory, or inaccessible.
pub fn require_directory(path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;
    let inspection = inspect_directory(path)?;
    if !inspection.exists {
        return Err(PathError::invalid(format!(
            "path does not exist: {}",
            path.to_string_lossy()
        )));
    }
    if !inspection.is_directory {
        return Err(PathError::invalid(format!(
            "path is not a directory: {}",
            path.to_string_lossy()
        )));
    }
    Ok(path.to_path_buf())
}
