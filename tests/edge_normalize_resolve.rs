//! Normalization, resolution, containment, and platform edge cases.

use path_rs::{
    PathError, absolute, ensure_inside, is_drive_relative, is_lexically_inside, join_relative,
    normalize, path_contains_reserved_name, resolve_against, resolve_inside,
};
use std::path::{Path, PathBuf};

#[test]
fn normalize_matrix() {
    for (input, expected) in [
        (".", "."),
        ("./", "."),
        ("foo", "foo"),
        ("./foo", "foo"),
        ("foo/../bar", "bar"),
        ("foo//bar", "foo/bar"),
        ("foo///bar", "foo/bar"),
        ("foo/./bar", "foo/bar"),
        ("../foo", "../foo"),
        ("foo/../../bar", "../bar"),
        ("foo/./../bar/./baz", "bar/baz"),
    ] {
        assert_eq!(
            normalize(input).unwrap(),
            PathBuf::from(expected),
            "normalize({input:?})"
        );
    }
    assert!(matches!(normalize(""), Err(PathError::EmptyInput)));
}

#[cfg(unix)]
#[test]
fn unix_root_cannot_escape() {
    assert_eq!(normalize("/").unwrap(), PathBuf::from("/"));
    assert_eq!(normalize("/../foo").unwrap(), PathBuf::from("/foo"));
    assert_eq!(normalize("/foo/../bar").unwrap(), PathBuf::from("/bar"));
}

#[cfg(windows)]
#[test]
fn windows_drive_and_relative() {
    assert_eq!(
        normalize(r"C:\Users\x\..\y").unwrap(),
        PathBuf::from(r"C:\Users\y")
    );
    assert!(matches!(
        normalize(r"C:foo"),
        Err(PathError::DriveRelativePath { .. })
    ));
    assert!(matches!(
        normalize(r"C:"),
        Err(PathError::DriveRelativePath { .. })
    ));
    assert!(!is_drive_relative(Path::new(r"C:\foo")));
    assert!(is_drive_relative(Path::new(r"C:foo")));
}

#[test]
fn resolve_and_join_matrix() {
    assert_eq!(
        resolve_against("/repo", "src/main.rs").unwrap(),
        PathBuf::from("/repo/src/main.rs")
    );
    assert_eq!(
        resolve_against("/repo", "./src/../Cargo.toml").unwrap(),
        PathBuf::from("/repo/Cargo.toml")
    );
    assert_eq!(
        join_relative("/repo", "src/lib.rs").unwrap(),
        PathBuf::from("/repo/src/lib.rs")
    );
}

#[cfg(unix)]
#[test]
fn join_rejects_absolute_unix() {
    assert!(matches!(
        join_relative("/repo", "/etc/passwd"),
        Err(PathError::AbsoluteChildPath { .. })
    ));
    assert!(matches!(
        resolve_inside("/repo", "/etc/passwd"),
        Err(PathError::AbsoluteChildPath { .. })
    ));
}

#[test]
fn containment_matrix() {
    assert!(is_lexically_inside(Path::new("/repo"), Path::new("/repo")));
    assert!(is_lexically_inside(
        Path::new("/repo/src"),
        Path::new("/repo")
    ));
    assert!(!is_lexically_inside(
        Path::new("/repo2"),
        Path::new("/repo")
    ));
    assert!(!is_lexically_inside(
        Path::new("/etc/passwd"),
        Path::new("/repo")
    ));

    assert_eq!(
        ensure_inside("/repo", "/repo/src/../Cargo.toml").unwrap(),
        PathBuf::from("/repo/Cargo.toml")
    );
    assert!(matches!(
        ensure_inside("/repo", "/etc/passwd"),
        Err(PathError::RootEscape { .. })
    ));
    assert!(matches!(
        resolve_inside("/repo", "../../etc/passwd"),
        Err(PathError::RootEscape { .. })
    ));
    assert_eq!(
        resolve_inside("/repo", "src/main.rs").unwrap(),
        PathBuf::from("/repo/src/main.rs")
    );
}

#[test]
fn absolute_makes_relative_absolute() {
    let p = absolute(".").unwrap();
    assert!(p.is_absolute());
}

#[test]
fn reserved_names_in_components() {
    assert!(path_contains_reserved_name(Path::new("foo/NUL/bar")));
    assert!(path_contains_reserved_name(Path::new("COM1")));
    assert!(!path_contains_reserved_name(Path::new("console/file")));
}

#[test]
fn normalize_idempotent_samples() {
    for s in ["foo/bar", "./a/../b", "x//y/./z", "../up"] {
        let once = normalize(s).unwrap();
        let twice = normalize(&once).unwrap();
        assert_eq!(once, twice, "idempotent for {s}");
    }
}
