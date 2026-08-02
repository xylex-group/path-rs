//! Lexical path normalization and filesystem canonicalization.
//!
//! # Important distinction
//!
//! - [`normalize`] is **lexical**: no filesystem access, no symlink resolution.
//! - [`canonicalize_existing`] accesses the filesystem, requires existence, and resolves symlinks.

use crate::error::PathError;
use crate::internal::components::is_drive_relative_path;
use crate::internal::validation::reject_nul_path;
use std::path::{Component, Path, PathBuf};

/// Lexically normalize a path without accessing the filesystem.
///
/// Collapses `.`, resolves `..` where possible, and removes empty segments
/// produced by repeated separators. Leading `..` components on relative paths
/// are preserved. Absolute paths that would escape the root via `..` stop at
/// the root (Unix `/` or Windows drive/UNC root).
///
/// # Filesystem access
///
/// **No.** This never calls `std::fs::canonicalize` and does not follow symlinks.
///
/// # Drive-relative paths
///
/// Windows drive-relative paths (`C:foo`, `C:`) are rejected.
///
/// # Examples
///
/// ```
/// use path_rs::normalize;
/// use std::path::PathBuf;
///
/// let p = normalize("foo/./bar/../baz").unwrap();
/// assert_eq!(p, PathBuf::from("foo/baz"));
/// ```
pub fn normalize(path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;

    if path.as_os_str().is_empty() {
        return Err(PathError::EmptyInput);
    }

    if is_drive_relative_path(path) {
        return Err(PathError::drive_relative(path));
    }

    let mut out = PathBuf::new();
    let mut absolute = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                out.push(prefix.as_os_str());
            }
            Component::RootDir => {
                out.push(component.as_os_str());
                absolute = true;
            }
            Component::CurDir => {
                // Skip `.`
            }
            Component::ParentDir => {
                match out.components().next_back() {
                    None => {
                        if !absolute {
                            out.push("..");
                        }
                    }
                    Some(Component::ParentDir) => {
                        // Relative path already stacked with `..`
                        out.push("..");
                    }
                    Some(Component::RootDir) | Some(Component::Prefix(_)) => {
                        // Cannot escape absolute root / prefix.
                    }
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    Some(Component::CurDir) => {
                        out.pop();
                        out.push("..");
                    }
                }
            }
            Component::Normal(c) => {
                out.push(c);
            }
        }
    }

    // Preserve a bare root / prefix-only path.
    if out.as_os_str().is_empty() {
        if absolute {
            // Should not happen if RootDir was pushed.
            out.push(Component::RootDir.as_os_str());
        } else {
            out.push(".");
        }
    }

    Ok(out)
}

/// Canonicalize an existing path using the filesystem.
///
/// This resolves symlinks and requires every component to exist (platform rules apply).
/// On Windows, the result may be a verbatim `\\?\` path; [`crate::platform::simplify_for_display`]
/// can be used for user-facing output.
///
/// # Filesystem access
///
/// **Yes.** Requires the path to exist. Resolves symlinks.
///
/// # Examples
///
/// ```no_run
/// use path_rs::canonicalize_existing;
///
/// let p = canonicalize_existing(".").unwrap();
/// assert!(p.is_absolute());
/// ```
pub fn canonicalize_existing(path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;

    if is_drive_relative_path(path) {
        return Err(PathError::drive_relative(path));
    }

    std::fs::canonicalize(path).map_err(|e| PathError::filesystem(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_dot_and_parent() {
        assert_eq!(
            normalize("foo/./bar/../baz").unwrap(),
            PathBuf::from("foo/baz")
        );
        assert_eq!(normalize("./foo").unwrap(), PathBuf::from("foo"));
        assert_eq!(normalize("foo//bar").unwrap(), PathBuf::from("foo/bar"));
    }

    #[test]
    fn preserves_leading_parent_on_relative() {
        assert_eq!(normalize("../foo").unwrap(), PathBuf::from("../foo"));
        assert_eq!(normalize("foo/../../bar").unwrap(), PathBuf::from("../bar"));
    }

    #[test]
    fn empty_rejected() {
        assert!(matches!(normalize(""), Err(PathError::EmptyInput)));
    }
}
