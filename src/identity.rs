//! Path identity keys, deduplication, and display records.
//!
//! An identity key is a **comparison key**, not proof that two paths refer to
//! the same filesystem object. Lexical identity and filesystem identity differ.

use crate::error::PathError;
use crate::internal::validation::reject_nul_path;
use crate::normalize::normalize;
use crate::platform::{is_verbatim, simplify_for_display, translate_wsl_path};
use crate::text::CaseNormalization;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Options controlling how a path identity key is produced.
///
/// # Defaults
///
/// - Lexical normalize
/// - Normalize separators to `/`
/// - Platform default case (lowercase on Windows, preserve on Unix)
/// - Do **not** resolve symlinks
/// - Do **not** strip Windows verbatim prefixes
/// - Do **not** translate WSL paths
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathIdentityOptions {
    /// Apply lexical [`crate::normalize`] before keying.
    pub normalize_lexically: bool,
    /// Normalize path separators to `/` in the key string.
    pub normalize_separators: bool,
    /// Case policy for the key.
    pub case: CaseNormalization,
    /// When true, attempt `canonicalize` if the path exists (resolves symlinks).
    ///
    /// Failed canonicalize falls back to the lexical form.
    pub resolve_existing_symlinks: bool,
    /// When true, simplify Windows verbatim prefixes via `dunce` for the key.
    pub strip_windows_verbatim_prefix: bool,
    /// When true and the path looks like `/mnt/<drive>/...`, translate to Windows form first.
    pub translate_wsl_paths: bool,
}

impl Default for PathIdentityOptions {
    fn default() -> Self {
        Self {
            normalize_lexically: true,
            normalize_separators: true,
            case: CaseNormalization::PlatformDefault,
            resolve_existing_symlinks: false,
            strip_windows_verbatim_prefix: false,
            translate_wsl_paths: false,
        }
    }
}

impl PathIdentityOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }
}

/// A path together with optional display text and identity key.
///
/// The identity key must never be used as a filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRecord {
    /// Path suitable for filesystem access (when valid on the host).
    pub path: PathBuf,
    /// User-facing display string (not an identity key).
    pub display: String,
    /// Optional comparison key; never use for filesystem I/O.
    pub identity_key: Option<String>,
}

impl PathRecord {
    /// Build a record from a path, optionally computing an identity key.
    pub fn from_path(
        path: impl AsRef<Path>,
        identity: Option<PathIdentityOptions>,
    ) -> Result<Self, PathError> {
        let path = path.as_ref().to_path_buf();
        let display = path_display_string(&path);
        let identity_key = match identity {
            Some(opts) => Some(path_identity_key(&path, opts)?),
            None => None,
        };
        Ok(Self {
            path,
            display,
            identity_key,
        })
    }
}

/// Produce a deterministic path identity / comparison key.
///
/// # Filesystem access
///
/// Only when `resolve_existing_symlinks` is true (may call `canonicalize`).
/// Does not expand environment variables. Does not use a cache.
///
/// # Returns
///
/// A `String` **unsuitable** for direct filesystem access. It is a comparison
/// key only.
///
/// # Platform behavior
///
/// - Windows default case folding is ASCII-lowercase.
/// - Linux default preserves case.
/// - Symlink resolution changes identity semantics when enabled.
/// - Unicode normalization is not applied unless the `unicode` feature helpers
///   are used separately.
///
/// # Examples
///
/// ```
/// use path_rs::{path_identity_key, CaseNormalization, PathIdentityOptions};
///
/// let opts = PathIdentityOptions {
///     case: CaseNormalization::AsciiLowercase,
///     ..PathIdentityOptions::default()
/// };
/// let a = path_identity_key(r"C:\Users\Floris\Repo", opts).unwrap();
/// let b = path_identity_key(r"c:/users/floris/repo/", opts).unwrap();
/// assert_eq!(a, b);
/// ```
pub fn path_identity_key(
    path: impl AsRef<Path>,
    options: PathIdentityOptions,
) -> Result<String, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;

    if path.as_os_str().is_empty() {
        return Err(PathError::EmptyInput);
    }

    let mut working = path.to_path_buf();

    if options.translate_wsl_paths {
        if let Some(s) = path.to_str() {
            if let Some(translated) = translate_wsl_path(s)? {
                working = translated;
            }
        }
    }

    if options.resolve_existing_symlinks {
        if let Ok(canon) = std::fs::canonicalize(&working) {
            working = canon;
        }
    }

    if options.strip_windows_verbatim_prefix && is_verbatim(&working) {
        working = simplify_for_display(&working);
    }

    if options.normalize_lexically {
        working = normalize(&working)?;
    }

    // Prefer lossy only for the comparison key when the path is non-UTF-8.
    let mut key = working.to_string_lossy().into_owned();

    if options.normalize_separators {
        key = key.replace('\\', "/");
        key = collapse_interior_slashes(&key);
        // Trim trailing slash except for root forms like `C:/` or `/`.
        key = trim_trailing_slash_keep_root(&key);
    }

    Ok(options.case.apply(&key))
}

/// Deduplicate paths using identity keys; preserve first occurrence order.
///
/// Paths that fail key generation produce an error (fail-fast).
///
/// # Filesystem access
///
/// Same as [`path_identity_key`] for each path (only if options request it).
pub fn deduplicate_paths(
    paths: impl IntoIterator<Item = PathBuf>,
    options: PathIdentityOptions,
) -> Result<Vec<PathBuf>, PathError> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in paths {
        let key = path_identity_key(&path, options)?;
        if seen.insert(key) {
            out.push(path);
        }
    }
    Ok(out)
}

/// User-facing display string for a path (not an identity key).
pub fn path_display_string(path: &Path) -> String {
    path.display().to_string()
}

fn collapse_interior_slashes(s: &str) -> String {
    // Keep a leading "//" for UNC-ish keys; collapse only the remainder so we
    // never loop forever on a preserved prefix.
    if let Some(rest) = s.strip_prefix("//") {
        let mut rem = rest.to_owned();
        while rem.contains("//") {
            rem = rem.replace("//", "/");
        }
        format!("//{rem}")
    } else {
        let mut out = s.to_owned();
        while out.contains("//") {
            out = out.replace("//", "/");
        }
        out
    }
}

fn trim_trailing_slash_keep_root(s: &str) -> String {
    if s == "/" || s == "//" {
        return s.to_owned();
    }
    // Drive root `C:/` or `c:/`
    if s.len() == 3 && s.as_bytes()[1] == b':' && (s.ends_with('/') || s.ends_with('\\')) {
        return s.to_owned();
    }
    let mut out = s.to_owned();
    while out.len() > 1 && (out.ends_with('/') || out.ends_with('\\')) {
        // Don't strip to empty; don't strip drive root.
        if out.len() == 3 && out.as_bytes()[1] == b':' {
            break;
        }
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_and_separators() {
        let opts = PathIdentityOptions {
            case: CaseNormalization::AsciiLowercase,
            ..PathIdentityOptions::default()
        };
        let a = path_identity_key(r"C:\Users\Floris\Repo", opts).unwrap();
        let b = path_identity_key(r"c:/users/floris/repo/", opts).unwrap();
        let c = path_identity_key(r"C:/Users/Floris/Repo/.", opts).unwrap();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn dedup_preserves_first() {
        let opts = PathIdentityOptions {
            case: CaseNormalization::AsciiLowercase,
            ..PathIdentityOptions::default()
        };
        let paths = vec![
            PathBuf::from(r"C:\Users\Floris\Repo"),
            PathBuf::from(r"c:/users/floris/repo"),
            PathBuf::from(r"D:\other"),
        ];
        let d = deduplicate_paths(paths, opts).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0], PathBuf::from(r"C:\Users\Floris\Repo"));
    }
}
