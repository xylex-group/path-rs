//! Lexical root containment checks.
//!
//! # Security warning
//!
//! Lexical containment does **not** account for symlinks. A path that is
//! lexically inside a root may still resolve outside it via symlink traversal.
//! Do not use these helpers as a sole security boundary for untrusted paths.

use crate::error::PathError;
use crate::internal::components::starts_with_components;
use crate::internal::validation::reject_nul_path;
use crate::normalize::normalize;
use std::path::{Path, PathBuf};

/// Returns `true` if `path` is lexically equal to or beneath `root`.
///
/// Both paths should already be normalized for reliable results. Prefer
/// [`ensure_inside`] which normalizes first.
///
/// # Filesystem access
///
/// **No.**
pub fn is_lexically_inside(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    starts_with_components(path, root)
    // Ensure we match on component boundaries, not as a string prefix of a longer name.
    // `starts_with_components` already compares whole components.
}

/// Normalize both paths and ensure `path` stays inside `root`.
///
/// Returns the normalized path on success.
///
/// # Filesystem access
///
/// **No.**
///
/// # Security
///
/// Not symlink-safe. See module documentation.
pub fn ensure_inside(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let root = normalize(root.as_ref())?;
    let path = normalize(path.as_ref())?;
    reject_nul_path(&root)?;
    reject_nul_path(&path)?;

    if is_lexically_inside(&path, &root) {
        Ok(path)
    } else {
        Err(PathError::root_escape(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_and_escape() {
        let root = Path::new("/repo");
        assert!(is_lexically_inside(Path::new("/repo/src"), root));
        assert!(is_lexically_inside(Path::new("/repo"), root));
        assert!(!is_lexically_inside(Path::new("/etc"), root));
        assert!(!is_lexically_inside(Path::new("/repo2"), root));
    }
}
