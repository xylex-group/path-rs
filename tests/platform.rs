use path_rs::{
    PathError, is_device_namespace, is_drive_relative, is_reserved_windows_name, is_unc,
    is_verbatim, path_contains_reserved_name, platform_dirs, translate_wsl_path,
};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[test]
fn platform_dirs_resolve() {
    let d = platform_dirs().unwrap();
    assert!(!d.home.as_os_str().is_empty());
    assert!(!d.config.as_os_str().is_empty());
    assert!(!d.temp.as_os_str().is_empty());
}

#[test]
fn reserved_names() {
    assert!(is_reserved_windows_name(OsStr::new("CON")));
    assert!(is_reserved_windows_name(OsStr::new("NUL.txt")));
    assert!(is_reserved_windows_name(OsStr::new("COM1.log")));
    assert!(is_reserved_windows_name(OsStr::new("PRN")));
    assert!(!is_reserved_windows_name(OsStr::new("console")));
    assert!(path_contains_reserved_name(Path::new("foo/NUL/bar")));
}

#[test]
fn wsl_paths() {
    assert_eq!(
        translate_wsl_path("/mnt/c/project").unwrap().unwrap(),
        PathBuf::from(r"C:\project")
    );
    assert_eq!(
        translate_wsl_path("/mnt/d/repo").unwrap().unwrap(),
        PathBuf::from(r"D:\repo")
    );
    assert_eq!(
        translate_wsl_path("/mnt/C/Users/floris").unwrap().unwrap(),
        PathBuf::from(r"C:\Users\floris")
    );

    assert!(translate_wsl_path("/mnt/data/foo").unwrap().is_none());
    assert!(translate_wsl_path("/mnt/cc/foo").unwrap().is_none());
    assert!(translate_wsl_path("/mnt/1/foo").unwrap().is_none());
    assert!(translate_wsl_path("/mnt/~/foo").unwrap().is_none());
    assert!(translate_wsl_path("/home/user").unwrap().is_none());
    assert!(matches!(
        translate_wsl_path("/mnt/"),
        Err(PathError::InvalidPath { .. })
    ));
}

#[cfg(windows)]
#[test]
fn windows_syntax() {
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
}

#[cfg(not(windows))]
#[test]
fn non_windows_prefix_helpers_are_false_for_unix_paths() {
    assert!(!is_drive_relative(Path::new("/home/user")));
    assert!(!is_unc(Path::new("/home/user")));
    assert!(!is_verbatim(Path::new("/home/user")));
    assert!(!is_device_namespace(Path::new("/home/user")));
}
