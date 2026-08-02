//! Platform classification and WSL matrix.

use path_rs::{
    PathError, is_device_namespace, is_drive_relative, is_reserved_windows_name, is_unc,
    is_verbatim, path_contains_reserved_name, platform_dirs, simplify_for_display,
    translate_wsl_path,
};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[test]
fn platform_dirs_fields() {
    let d = platform_dirs().unwrap();
    assert!(!d.home.as_os_str().is_empty());
    assert!(!d.config.as_os_str().is_empty());
    assert!(!d.data.as_os_str().is_empty());
    assert!(!d.cache.as_os_str().is_empty());
    assert!(!d.temp.as_os_str().is_empty());
}

#[test]
fn reserved_name_matrix() {
    for name in [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM9", "LPT1", "LPT9", "nul.txt", "COM1.log", "con",
    ] {
        assert!(
            is_reserved_windows_name(OsStr::new(name)),
            "expected reserved: {name}"
        );
    }
    for name in ["console", "file.txt", "nullish", "COM10", "LPT10"] {
        assert!(
            !is_reserved_windows_name(OsStr::new(name)),
            "expected not reserved: {name}"
        );
    }
    assert!(path_contains_reserved_name(Path::new("a/NUL/b")));
    assert!(!path_contains_reserved_name(Path::new("a/b/c")));
}

#[test]
fn wsl_matrix() {
    assert_eq!(
        translate_wsl_path("/mnt/c/project").unwrap().unwrap(),
        PathBuf::from(r"C:\project")
    );
    assert_eq!(
        translate_wsl_path("/mnt/d/repo").unwrap().unwrap(),
        PathBuf::from(r"D:\repo")
    );
    assert_eq!(
        translate_wsl_path("/mnt/C/Users/x").unwrap().unwrap(),
        PathBuf::from(r"C:\Users\x")
    );

    for bad_none in [
        "/mnt/data/foo",
        "/mnt/cc/foo",
        "/mnt/1/foo",
        "/mnt/~/foo",
        "/home/user",
        "/mnt/c", // actually allowed as drive root in our impl
    ] {
        // /mnt/c alone is allowed as C:\
        if bad_none == "/mnt/c" {
            assert!(translate_wsl_path(bad_none).unwrap().is_some());
            continue;
        }
        assert!(
            translate_wsl_path(bad_none).unwrap().is_none()
                || translate_wsl_path(bad_none).is_err(),
            "unexpected success for {bad_none}"
        );
    }

    assert!(matches!(
        translate_wsl_path("/mnt/"),
        Err(PathError::InvalidPath { .. })
    ));
}

#[cfg(windows)]
#[test]
fn windows_prefix_matrix() {
    assert!(!is_drive_relative(Path::new(r"C:\Users\floris")));
    assert!(!is_drive_relative(Path::new(r"C:/Users/floris")));
    assert!(is_drive_relative(Path::new(r"C:Users/floris")));
    assert!(is_drive_relative(Path::new(r"C:")));
    assert!(!is_drive_relative(Path::new(r"D:\repo")));

    assert!(is_unc(Path::new(r"\\server\share")));
    assert!(is_unc(Path::new(r"\\server\share\folder")));
    assert!(is_verbatim(Path::new(r"\\?\C:\repo")));
    assert!(is_verbatim(Path::new(r"\\?\UNC\server\share")));
    assert!(is_device_namespace(Path::new(r"\\.\PIPE\name")));

    let simplified = simplify_for_display(Path::new(r"\\?\C:\Windows"));
    assert!(!simplified.as_os_str().is_empty());
}

#[cfg(not(windows))]
#[test]
fn non_windows_unix_paths() {
    assert!(!is_drive_relative(Path::new("/home/user")));
    assert!(!is_unc(Path::new("/home/user")));
    assert!(!is_verbatim(Path::new("/home/user")));
    assert!(!is_device_namespace(Path::new("/home/user")));
    let p = simplify_for_display(Path::new("/tmp/x"));
    assert_eq!(p, PathBuf::from("/tmp/x"));
}
