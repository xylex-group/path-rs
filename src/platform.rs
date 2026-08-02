//! Platform-specific path classification and Windows helpers.
//!
//! These helpers inspect path *syntax*. They do not access the filesystem.

use crate::error::PathError;
use crate::internal::components::{is_drive_relative_path, reserved_base_name};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf, Prefix};

/// Returns `true` if `path` is a Windows drive-relative path such as `C:foo` or `C:`.
///
/// Drive-relative paths are not portable and are rejected by resolution helpers by default.
///
/// # Filesystem access
///
/// No.
pub fn is_drive_relative(path: &Path) -> bool {
    is_drive_relative_path(path)
}

/// Returns `true` if `path` uses a UNC prefix (`\\server\share` or verbatim UNC).
///
/// # Filesystem access
///
/// No.
pub fn is_unc(path: &Path) -> bool {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => {
            matches!(prefix.kind(), Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _))
        }
        _ => false,
    }
}

/// Returns `true` if `path` uses a Windows verbatim prefix (`\\?\...`).
///
/// # Filesystem access
///
/// No.
pub fn is_verbatim(path: &Path) -> bool {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => matches!(
            prefix.kind(),
            Prefix::Verbatim(_) | Prefix::VerbatimUNC(_, _) | Prefix::VerbatimDisk(_)
        ),
        _ => false,
    }
}

/// Returns `true` if `path` uses a device namespace prefix (`\\.\...`).
///
/// # Filesystem access
///
/// No.
pub fn is_device_namespace(path: &Path) -> bool {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => matches!(prefix.kind(), Prefix::DeviceNS(_)),
        _ => false,
    }
}

/// Returns `true` if `name` is a Windows reserved device name (optionally with extension).
///
/// Examples: `CON`, `NUL.txt`, `COM1.log`.
///
/// # Filesystem access
///
/// No.
pub fn is_reserved_windows_name(name: &OsStr) -> bool {
    reserved_base_name(name).is_some()
}

/// Returns `true` if any normal component of `path` is a Windows reserved name.
///
/// # Filesystem access
///
/// No.
pub fn path_contains_reserved_name(path: &Path) -> bool {
    path.components().any(|c| match c {
        Component::Normal(name) => is_reserved_windows_name(name),
        _ => false,
    })
}

/// Translate a WSL-style `/mnt/<drive>/...` path to a Windows path.
///
/// Only single-letter drive mounts are recognized (`/mnt/c/...`).
/// Returns `Ok(None)` when the input is not a WSL mount path.
///
/// Translation is only performed when `cfg!(windows)` is true; on other platforms
/// the function still parses and, if requested via callers, may return a synthetic
/// Windows-style path string for tooling that needs the mapping.
///
/// # Filesystem access
///
/// No.
pub fn translate_wsl_path(input: &str) -> Result<Option<PathBuf>, PathError> {
    let trimmed = input.trim();
    if !trimmed.starts_with("/mnt/") {
        return Ok(None);
    }

    let rest = &trimmed[5..]; // after "/mnt/"
    if rest.is_empty() {
        return Err(PathError::invalid(
            "WSL path '/mnt/' is incomplete; expected /mnt/<drive>/...",
        ));
    }

    let mut parts = rest.splitn(2, '/');
    let drive = parts.next().unwrap_or("");
    let tail = parts.next().unwrap_or("");

    if drive.is_empty() {
        return Err(PathError::invalid("WSL path is missing a drive letter"));
    }

    // Exactly one ASCII alphabetic character.
    let mut chars = drive.chars();
    let Some(letter) = chars.next() else {
        return Err(PathError::invalid("WSL path is missing a drive letter"));
    };
    if chars.next().is_some() || !letter.is_ascii_alphabetic() {
        // Not a single-letter drive mount (e.g. /mnt/data, /mnt/cc, /mnt/1).
        return Ok(None);
    }

    // Bare `/mnt/c` without trailing path: treat as incomplete for safety when no more path.
    // Callers often accept `/mnt/c` as the drive root; we allow it as `<letter>:\`.
    let mut out = PathBuf::new();
    let mut drive_root = String::with_capacity(3);
    drive_root.push(letter.to_ascii_uppercase());
    drive_root.push(':');
    drive_root.push('\\');
    out.push(drive_root);

    if !tail.is_empty() {
        for segment in tail.split('/') {
            if segment.is_empty() || segment == "." {
                continue;
            }
            if segment == ".." {
                if !out.pop() {
                    return Err(PathError::invalid(
                        "WSL path escapes the drive root via '..'",
                    ));
                }
                // Ensure we still have a drive root.
                if out.as_os_str().is_empty() {
                    return Err(PathError::invalid(
                        "WSL path escapes the drive root via '..'",
                    ));
                }
                continue;
            }
            out.push(segment);
        }
    }

    Ok(Some(out))
}

/// Soften a Windows path for display using `dunce` (may strip `\\?\` when safe).
///
/// Does not mutate paths used for security decisions; intended for user-facing output.
///
/// # Filesystem access
///
/// No.
pub fn simplify_for_display(path: &Path) -> PathBuf {
    dunce::simplified(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reserved_names() {
        assert!(is_reserved_windows_name(OsStr::new("CON")));
        assert!(is_reserved_windows_name(OsStr::new("nul.txt")));
        assert!(is_reserved_windows_name(OsStr::new("COM1.log")));
        assert!(!is_reserved_windows_name(OsStr::new("console")));
        assert!(!is_reserved_windows_name(OsStr::new("file.txt")));
    }

    #[test]
    fn wsl_translation() {
        let p = translate_wsl_path("/mnt/c/Users/floris").unwrap().unwrap();
        assert_eq!(p, PathBuf::from(r"C:\Users\floris"));

        let p = translate_wsl_path("/mnt/d/repo").unwrap().unwrap();
        assert_eq!(p, PathBuf::from(r"D:\repo"));

        assert!(translate_wsl_path("/mnt/data/foo").unwrap().is_none());
        assert!(translate_wsl_path("/mnt/cc/foo").unwrap().is_none());
        assert!(translate_wsl_path("/mnt/1/foo").unwrap().is_none());
        assert!(translate_wsl_path("/mnt/~/foo").unwrap().is_none());
        assert!(translate_wsl_path("/home/user").unwrap().is_none());
        assert!(translate_wsl_path("/mnt/").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_prefix_detection() {
        assert!(is_drive_relative(Path::new(r"C:foo")));
        assert!(is_drive_relative(Path::new(r"C:")));
        assert!(!is_drive_relative(Path::new(r"C:\foo")));
        assert!(!is_drive_relative(Path::new(r"C:/foo")));
        assert!(is_unc(Path::new(r"\\server\share")));
        assert!(is_verbatim(Path::new(r"\\?\C:\repo")));
        assert!(is_device_namespace(Path::new(r"\\.\PIPE\name")));
    }
}
