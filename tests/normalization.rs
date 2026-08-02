use path_rs::{PathError, normalize};
use proptest::prelude::*;
use std::path::PathBuf;

#[test]
fn basic_normalization() {
    assert_eq!(normalize(".").unwrap(), PathBuf::from("."));
    assert_eq!(normalize("./").unwrap(), PathBuf::from("."));
    assert_eq!(normalize("foo").unwrap(), PathBuf::from("foo"));
    assert_eq!(normalize("./foo").unwrap(), PathBuf::from("foo"));
    assert_eq!(normalize("foo/../bar").unwrap(), PathBuf::from("bar"));
    assert_eq!(normalize("foo//bar").unwrap(), PathBuf::from("foo/bar"));
    assert_eq!(normalize("foo///bar").unwrap(), PathBuf::from("foo/bar"));
    assert_eq!(normalize("foo/./bar").unwrap(), PathBuf::from("foo/bar"));
    assert_eq!(normalize("../foo").unwrap(), PathBuf::from("../foo"));
    assert_eq!(normalize("foo/../../bar").unwrap(), PathBuf::from("../bar"));
}

#[test]
fn empty_input() {
    assert!(matches!(normalize(""), Err(PathError::EmptyInput)));
}

#[cfg(unix)]
#[test]
fn unix_absolute_root() {
    assert_eq!(normalize("/").unwrap(), PathBuf::from("/"));
    assert_eq!(normalize("/foo/../bar").unwrap(), PathBuf::from("/bar"));
    // Cannot escape root
    assert_eq!(normalize("/../foo").unwrap(), PathBuf::from("/foo"));
}

#[cfg(windows)]
#[test]
fn windows_drive_absolute() {
    let p = normalize(r"C:\Users\floris\..\other").unwrap();
    assert_eq!(p, PathBuf::from(r"C:\Users\other"));
    assert!(matches!(
        normalize(r"C:foo"),
        Err(PathError::DriveRelativePath { .. })
    ));
    assert!(matches!(
        normalize(r"C:"),
        Err(PathError::DriveRelativePath { .. })
    ));
}

proptest! {
    #[test]
    fn normalize_is_idempotent(s in "[A-Za-z0-9_./]{1,64}") {
        // Avoid empty and pure-dot-oddities that may map differently.
        if s.trim().is_empty() {
            return Ok(());
        }
        // Skip Windows drive-relative-looking inputs on all platforms for simplicity.
        if s.len() >= 2 && s.as_bytes()[1] == b':' {
            return Ok(());
        }
        if let Ok(once) = normalize(&s) {
            let twice = normalize(&once).unwrap();
            prop_assert_eq!(once, twice);
        }
    }
}
