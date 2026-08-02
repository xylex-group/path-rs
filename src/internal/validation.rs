//! Input validation helpers.

use crate::error::PathError;
use crate::platform::is_reserved_windows_name;
use std::path::{Component, Path};

/// Maximum environment expansion depth (defensive DoS limit).
pub(crate) const MAX_EXPANSION_DEPTH: u32 = 8;

/// Maximum accepted glob pattern length.
#[cfg(feature = "search")]
pub(crate) const MAX_PATTERN_LENGTH: usize = 4096;

/// Reject paths containing an embedded NUL.
pub(crate) fn reject_nul(input: &str) -> Result<(), PathError> {
    if input.contains('\0') {
        return Err(PathError::EmbeddedNul);
    }
    Ok(())
}

/// Reject OsStr paths containing an embedded NUL (best-effort via lossy scan of bytes where possible).
pub(crate) fn reject_nul_path(path: &Path) -> Result<(), PathError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if path.as_os_str().as_bytes().contains(&0) {
            return Err(PathError::EmbeddedNul);
        }
    }
    #[cfg(not(unix))]
    {
        if path.to_string_lossy().contains('\0') {
            return Err(PathError::EmbeddedNul);
        }
    }
    Ok(())
}

/// Validate an application name used when building platform subdirectories.
///
/// Rules:
/// - non-empty
/// - no path separators
/// - no `..`
/// - no NUL
/// - no Windows reserved device names
/// - no path traversal components
pub(crate) fn validate_app_name(app_name: &str) -> Result<(), PathError> {
    if app_name.is_empty() {
        return Err(PathError::InvalidAppName {
            message: "application name must not be empty".into(),
        });
    }
    reject_nul(app_name).map_err(|_| PathError::InvalidAppName {
        message: "application name must not contain NUL".into(),
    })?;

    if app_name.contains('/') || app_name.contains('\\') {
        return Err(PathError::InvalidAppName {
            message: "application name must not contain path separators".into(),
        });
    }

    if app_name == "." || app_name == ".." {
        return Err(PathError::InvalidAppName {
            message: "application name must not be '.' or '..'".into(),
        });
    }

    if app_name.contains("..") {
        return Err(PathError::InvalidAppName {
            message: "application name must not contain '..'".into(),
        });
    }

    // Reject multi-component paths disguised without separators (defensive).
    let as_path = Path::new(app_name);
    let mut components = as_path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) => {
            if is_reserved_windows_name(name) {
                return Err(PathError::InvalidAppName {
                    message: format!(
                        "application name collides with a Windows reserved name: {}",
                        name.to_string_lossy()
                    ),
                });
            }
        }
        _ => {
            return Err(PathError::InvalidAppName {
                message: "application name must be a single path component".into(),
            });
        }
    }

    Ok(())
}

/// True if the path component name should be treated as hidden.
#[cfg(feature = "listing")]
pub(crate) fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}
