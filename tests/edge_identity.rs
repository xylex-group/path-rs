//! Path identity, token normalization, and directory inspection edge cases.

use path_rs::{
    CaseNormalization, PathIdentityOptions, PathRecord, TextNormalizationOptions,
    deduplicate_paths, directory_exists, inspect_directory, is_existing_directory,
    normalize_path_token, path_display_string, path_identity_key, require_directory,
    translate_wsl_path,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn ascii_opts() -> PathIdentityOptions {
    PathIdentityOptions {
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    }
}

#[test]
fn repeated_separators_and_trailing() {
    let o = ascii_opts();
    let a = path_identity_key(r"C:\Users\Floris\\Repo", o).unwrap();
    let b = path_identity_key(r"C:\Users\Floris\Repo\", o).unwrap();
    let c = path_identity_key(r"C:\Users\Floris\Repo\.", o).unwrap();
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn platform_default_case() {
    let o = PathIdentityOptions::default();
    let lower = path_identity_key(r"C:\Users\Floris", o).unwrap();
    let upper = path_identity_key(r"C:\USERS\FLORIS", o).unwrap();
    if cfg!(windows) {
        assert_eq!(lower, upper);
    } else {
        // Non-Windows: PlatformDefault preserves case; these paths are still strings.
        // On Unix, backslash is a normal character — keys may differ.
        let _ = (lower, upper);
    }
}

#[test]
fn preserve_case_differs() {
    let o = PathIdentityOptions {
        case: CaseNormalization::Preserve,
        normalize_separators: true,
        ..PathIdentityOptions::default()
    };
    let a = path_identity_key("Foo/Bar", o).unwrap();
    let b = path_identity_key("foo/bar", o).unwrap();
    assert_ne!(a, b);
}

#[test]
fn wsl_translation_in_identity() {
    let o = PathIdentityOptions {
        translate_wsl_paths: true,
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    let wsl = path_identity_key("/mnt/c/Users/Floris", o).unwrap();
    let win = path_identity_key(r"C:\Users\Floris", o).unwrap();
    assert_eq!(wsl, win);
}

#[test]
fn identity_key_not_for_fs_access_documented() {
    let key = path_identity_key(r"C:\Users\Floris\Repo", ascii_opts()).unwrap();
    // Comparison key only: normalized separators + case — not a native FS path form.
    assert!(!key.is_empty());
    assert!(key.contains("users/floris/repo") || key.contains("Users"));
    assert!(!key.contains('\\'));
}

#[test]
fn empty_path_errors() {
    assert!(path_identity_key("", ascii_opts()).is_err());
}

#[test]
fn path_record_display_differs_from_key() {
    let path = PathBuf::from(r"C:\Users\Floris\Repo");
    let rec = PathRecord::from_path(&path, Some(ascii_opts())).unwrap();
    assert_eq!(rec.path, path);
    assert_eq!(rec.display, path_display_string(&path));
    let key = rec.identity_key.unwrap();
    // Display preserves original formatting intent; key is normalized.
    assert_ne!(key, rec.display);
}

#[test]
fn dedup_order_preserved() {
    let paths = vec![
        PathBuf::from(r"D:\first"),
        PathBuf::from(r"d:/first"),
        PathBuf::from(r"D:\second"),
        PathBuf::from(r"d:\second\"),
        PathBuf::from(r"E:\third"),
    ];
    let out = deduplicate_paths(paths, ascii_opts()).unwrap();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], PathBuf::from(r"D:\first"));
    assert_eq!(out[1], PathBuf::from(r"D:\second"));
    assert_eq!(out[2], PathBuf::from(r"E:\third"));
}

#[test]
fn token_normalization_matrix() {
    let opts = TextNormalizationOptions {
        trim_whitespace: true,
        trim_trailing_separators: true,
        normalize_separators: true,
        case: CaseNormalization::AsciiLowercase,
    };
    assert_eq!(normalize_path_token("  REPO  ", &opts), "repo");
    assert_eq!(normalize_path_token(r"repo\", &opts), "repo");
    assert_eq!(normalize_path_token("repo/", &opts), "repo");
    assert_eq!(normalize_path_token(r"owner\repo", &opts), "owner/repo");
    // `.git` is NOT stripped by path-rs
    assert_eq!(normalize_path_token("repo.git", &opts), "repo.git");
    // URLs are opaque tokens here: separators normalize, `.git` is NOT stripped.
    let url = normalize_path_token("https://github.com/owner/repo.git", &opts);
    assert!(url.contains("github.com/owner/repo.git"));
    assert!(url.ends_with("repo.git"));

    let preserve = TextNormalizationOptions {
        case: CaseNormalization::Preserve,
        ..opts
    };
    assert_eq!(normalize_path_token("REPO", &preserve), "REPO");
}

#[test]
fn directory_inspection_file_missing_symlink() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("f.txt");
    fs::write(&file, b"hi").unwrap();

    assert!(is_existing_directory(dir.path()));
    assert!(directory_exists(dir.path()));
    assert!(!is_existing_directory(&file));
    assert!(!directory_exists(dir.path().join("nope")));

    let insp_dir = inspect_directory(dir.path()).unwrap();
    assert!(insp_dir.exists && insp_dir.is_directory && !insp_dir.is_symlink);

    let insp_file = inspect_directory(&file).unwrap();
    assert!(insp_file.exists && !insp_file.is_directory);

    let insp_miss = inspect_directory(dir.path().join("nope")).unwrap();
    assert!(!insp_miss.exists && !insp_miss.is_directory);

    assert!(require_directory(dir.path()).is_ok());
    assert!(require_directory(&file).is_err());
    assert!(require_directory(dir.path().join("nope")).is_err());
}

#[cfg(unix)]
#[test]
fn symlink_directory_inspection() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let insp = inspect_directory(&link).unwrap();
    assert!(insp.exists);
    assert!(insp.is_symlink);
    assert!(insp.is_directory); // follows for is_directory

    let broken = dir.path().join("broken");
    std::os::unix::fs::symlink(dir.path().join("missing"), &broken).unwrap();
    let b = inspect_directory(&broken).unwrap();
    assert!(b.exists); // symlink exists
    assert!(b.is_symlink);
    assert!(!b.is_directory);
}

#[cfg(windows)]
#[test]
fn windows_verbatim_and_unc_identity() {
    let o = PathIdentityOptions {
        strip_windows_verbatim_prefix: true,
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    // Verbatim form of a real path (cwd) if available.
    let cwd = std::env::current_dir().unwrap();
    let key1 = path_identity_key(&cwd, o).unwrap();
    assert!(!key1.is_empty());

    // UNC-like lexical forms still produce keys without panicking.
    let unc = path_identity_key(r"\\server\share\folder", ascii_opts());
    assert!(unc.is_ok());
}

#[test]
fn wsl_helper_still_generic() {
    assert!(translate_wsl_path("/mnt/c/x").unwrap().is_some());
    assert!(translate_wsl_path("/home/user").unwrap().is_none());
}

#[test]
fn resolve_existing_symlinks_option_does_not_require_existence() {
    let o = PathIdentityOptions {
        resolve_existing_symlinks: true,
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    // Missing path: falls back to lexical
    let key = path_identity_key("/this/path/should/not/exist/path-rs-test", o).unwrap();
    assert!(!key.is_empty());
}
