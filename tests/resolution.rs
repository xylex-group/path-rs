use path_rs::{PathError, absolute, join_relative, resolve_against, resolve_inside};
use std::path::PathBuf;

#[test]
fn resolve_against_relative() {
    let p = resolve_against("/repo", "src/main.rs").unwrap();
    assert_eq!(p, PathBuf::from("/repo/src/main.rs"));

    let p = resolve_against("/repo", "./src/../Cargo.toml").unwrap();
    assert_eq!(p, PathBuf::from("/repo/Cargo.toml"));
}

#[test]
fn resolve_against_absolute_input() {
    #[cfg(unix)]
    {
        let p = resolve_against("/repo", "/etc/passwd").unwrap();
        assert_eq!(p, PathBuf::from("/etc/passwd"));
    }
}

#[test]
fn join_relative_rejects_absolute() {
    #[cfg(unix)]
    {
        assert!(matches!(
            join_relative("/repo", "/etc/passwd"),
            Err(PathError::AbsoluteChildPath { .. })
        ));
    }
}

#[test]
fn resolve_inside_ok_and_escape() {
    assert_eq!(
        resolve_inside("/repo", "src/main.rs").unwrap(),
        PathBuf::from("/repo/src/main.rs")
    );
    assert_eq!(
        resolve_inside("/repo", "./src/../Cargo.toml").unwrap(),
        PathBuf::from("/repo/Cargo.toml")
    );
    assert!(matches!(
        resolve_inside("/repo", "../../etc/passwd"),
        Err(PathError::RootEscape { .. })
    ));
    #[cfg(unix)]
    {
        assert!(matches!(
            resolve_inside("/repo", "/etc/passwd"),
            Err(PathError::AbsoluteChildPath { .. })
        ));
    }
}

#[test]
fn absolute_cwd_relative() {
    let p = absolute(".").unwrap();
    assert!(p.is_absolute());
}

#[cfg(windows)]
#[test]
fn windows_join_and_escape() {
    assert!(matches!(
        join_relative(r"C:\repo", r"D:\other"),
        Err(PathError::AbsoluteChildPath { .. })
    ));
    assert!(matches!(
        resolve_inside(r"C:\repo", r"..\outside"),
        Err(PathError::RootEscape { .. })
    ));
    let p = resolve_inside(r"\\server\share", r"folder\file").unwrap();
    assert!(p.ends_with(r"folder\file") || p.ends_with("folder/file"));
}
