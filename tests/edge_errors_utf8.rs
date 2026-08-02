//! Error surface and UTF-8 helpers.

use path_rs::{
    ExpandOptions, PathError, expand_input, normalize, path_to_string_lossy, path_to_utf8,
};
use std::path::Path;

#[test]
fn path_error_display_is_useful() {
    let err = PathError::UndefinedEnvironmentVariable { name: "FOO".into() };
    assert!(err.to_string().contains("FOO"));

    let err = PathError::RootEscape {
        path: "/etc/passwd".into(),
    };
    assert!(err.to_string().contains("/etc/passwd"));

    let err = PathError::filesystem(Path::new("/tmp/x"), std::io::Error::other("boom"));
    assert!(err.to_string().contains("boom"));
}

#[test]
fn error_helpers() {
    let e = PathError::invalid("bad");
    assert!(matches!(e, PathError::InvalidPath { .. }));
    let e = PathError::traversal("walk failed");
    assert!(matches!(e, PathError::Traversal { .. }));
    let e = PathError::cache("cache fail");
    assert!(matches!(e, PathError::Cache { .. }));
    let e = PathError::root_escape(Path::new("/x"));
    assert!(matches!(e, PathError::RootEscape { .. }));
    let e = PathError::absolute_child(Path::new("/abs"));
    assert!(matches!(e, PathError::AbsoluteChildPath { .. }));
    let e = PathError::drive_relative(Path::new("C:foo"));
    assert!(matches!(e, PathError::DriveRelativePath { .. }));
}

#[test]
fn utf8_helpers() {
    assert_eq!(path_to_utf8(Path::new("foo/bar")).unwrap(), "foo/bar");
    let lossy = path_to_string_lossy(Path::new("foo"));
    assert_eq!(&*lossy, "foo");
}

#[cfg(unix)]
#[test]
fn non_utf8_path_errors_on_strict_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let path = Path::new(OsStr::from_bytes(b"foo/\xff/bar"));
    assert!(matches!(path_to_utf8(path), Err(PathError::NotUtf8)));
    // lossy still works
    let _ = path_to_string_lossy(path);
}

#[test]
fn expand_and_normalize_error_paths() {
    assert!(matches!(
        expand_input("", &ExpandOptions::default()),
        Err(PathError::EmptyInput)
    ));
    assert!(matches!(normalize(""), Err(PathError::EmptyInput)));
}

#[cfg(feature = "unicode")]
#[test]
fn logical_path_key_nfc() {
    use path_rs::logical_path_key;
    // é as single codepoint vs combining
    let a = logical_path_key(Path::new("cafe\u{301}"));
    let b = logical_path_key(Path::new("caf\u{e9}"));
    assert_eq!(a, b);
}

#[test]
fn path_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PathError>();
}
