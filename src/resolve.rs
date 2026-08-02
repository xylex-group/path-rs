//! Path resolution and safe joining.
//!
//! These helpers operate on `Path` / `PathBuf` components. They do not expand
//! environment variables or glob patterns.

use crate::containment::is_lexically_inside;
use crate::error::PathError;
use crate::internal::components::is_drive_relative_path;
use crate::internal::validation::reject_nul_path;
use crate::normalize::normalize;
use std::path::{Path, PathBuf};

/// Make `path` absolute.
///
/// - Absolute inputs are lexically normalized.
/// - Relative inputs are joined with the process current directory, then normalized.
///
/// # Filesystem access
///
/// Reads the current working directory for relative inputs. Does **not** require
/// the path to exist and does **not** resolve symlinks.
pub fn absolute(path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;

    if is_drive_relative_path(path) {
        return Err(PathError::drive_relative(path));
    }

    if path.is_absolute() {
        return normalize(path);
    }

    let cwd = std::env::current_dir()
        .map_err(|e| PathError::CurrentDirectoryUnavailable { source: e })?;
    normalize(cwd.join(path))
}

/// Resolve `input` against `base`.
///
/// - If `input` is absolute, it is normalized and returned (base is ignored).
/// - If `input` is relative, it is joined to `base` and normalized.
///
/// # Filesystem access
///
/// **No** (unless `base` itself requires later I/O by the caller).
/// Does not follow symlinks. Does not require existence.
pub fn resolve_against(
    base: impl AsRef<Path>,
    input: impl AsRef<Path>,
) -> Result<PathBuf, PathError> {
    let base = base.as_ref();
    let input = input.as_ref();
    reject_nul_path(base)?;
    reject_nul_path(input)?;

    if is_drive_relative_path(input) {
        return Err(PathError::drive_relative(input));
    }
    if is_drive_relative_path(base) {
        return Err(PathError::drive_relative(base));
    }

    if input.is_absolute() {
        return normalize(input);
    }

    normalize(base.join(input))
}

/// Join a **relative** child under `base`, rejecting absolute children.
///
/// # Filesystem access
///
/// **No.** Does not follow symlinks.
///
/// # Errors
///
/// - Absolute `child` (including Windows drive-absolute / UNC)
/// - Drive-relative Windows children
pub fn join_relative(
    base: impl AsRef<Path>,
    child: impl AsRef<Path>,
) -> Result<PathBuf, PathError> {
    let base = base.as_ref();
    let child = child.as_ref();
    reject_nul_path(base)?;
    reject_nul_path(child)?;

    if is_drive_relative_path(child) {
        return Err(PathError::drive_relative(child));
    }
    if child.is_absolute() {
        return Err(PathError::absolute_child(child));
    }

    // Reject Windows prefixes that appear absolute-ish without is_absolute on some forms.
    if child
        .components()
        .next()
        .is_some_and(|c| matches!(c, std::path::Component::Prefix(_)))
    {
        return Err(PathError::absolute_child(child));
    }

    normalize(base.join(child))
}

/// Resolve `child` under `root`, ensuring the result stays inside `root` **lexically**.
///
/// # Security
///
/// Lexical containment is **not** symlink-safe. For security-sensitive uses,
/// combine with filesystem canonicalization of both root and result and re-check
/// containment. See `SECURITY.md`.
///
/// # Filesystem access
///
/// **No.**
pub fn resolve_inside(
    root: impl AsRef<Path>,
    child: impl AsRef<Path>,
) -> Result<PathBuf, PathError> {
    let root = root.as_ref();
    let child = child.as_ref();
    reject_nul_path(root)?;
    reject_nul_path(child)?;

    if is_drive_relative_path(root) {
        return Err(PathError::drive_relative(root));
    }
    if is_drive_relative_path(child) {
        return Err(PathError::drive_relative(child));
    }

    let joined = join_relative(root, child)?;
    let root_norm = normalize(root)?;

    if !is_lexically_inside(&joined, &root_norm) {
        return Err(PathError::root_escape(&joined));
    }

    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_rejects_absolute() {
        #[cfg(unix)]
        {
            assert!(matches!(
                join_relative("/repo", "/etc/passwd"),
                Err(PathError::AbsoluteChildPath { .. })
            ));
        }
    }

    #[test]
    fn resolve_inside_blocks_escape() {
        let err = resolve_inside("/repo", "../../etc/passwd").unwrap_err();
        assert!(matches!(err, PathError::RootEscape { .. }));
    }

    #[test]
    fn resolve_inside_allows_nested() {
        let p = resolve_inside("/repo", "src/main.rs").unwrap();
        assert_eq!(p, PathBuf::from("/repo/src/main.rs"));
    }
}
