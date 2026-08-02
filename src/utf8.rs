//! UTF-8 helpers for paths.
//!
//! Internal filesystem representation remains `Path` / `PathBuf` / `OsStr`.
//! Never round-trip a path through lossy UTF-8 for filesystem operations.

use crate::error::PathError;
use std::borrow::Cow;
use std::path::Path;

/// Convert a path to a UTF-8 string slice, or error if it is not valid UTF-8.
///
/// # Filesystem access
///
/// **No.**
pub fn path_to_utf8(path: &Path) -> Result<&str, PathError> {
    path.to_str().ok_or(PathError::NotUtf8)
}

/// Lossy UTF-8 conversion suitable for logs and UI only.
///
/// **Never** use the result as a filesystem path for further I/O.
///
/// # Filesystem access
///
/// **No.**
pub fn path_to_string_lossy(path: &Path) -> Cow<'_, str> {
    path.to_string_lossy()
}

/// Produce a Unicode-normalized logical key for comparison purposes.
///
/// Requires the `unicode` feature. The result must **not** be used as a filesystem path.
///
/// Normalization form: NFC.
#[cfg(feature = "unicode")]
pub fn logical_path_key(path: &Path) -> String {
    use unicode_normalization::UnicodeNormalization;
    path.to_string_lossy().nfc().collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn utf8_ok() {
        assert_eq!(path_to_utf8(Path::new("foo/bar")).unwrap(), "foo/bar");
    }
}
