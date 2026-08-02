use path_rs::{
    CaseNormalization, CommandLinePathMatchOptions, ExecutableMatchOptions, PathIdentityOptions,
    PathRecord, TextNormalizationOptions, command_line_contains_path, deduplicate_paths,
    directory_exists, executable_paths_match, inspect_directory, is_existing_directory,
    normalize_path_token, path_identity_key, require_directory,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn path_token_normalization() {
    let opts = TextNormalizationOptions {
        trim_whitespace: true,
        trim_trailing_separators: true,
        normalize_separators: true,
        case: CaseNormalization::AsciiLowercase,
    };
    assert_eq!(normalize_path_token(r"  Repo\Name\  ", &opts), "repo/name");
    // Callers strip `.git` themselves — library must not.
    assert_eq!(normalize_path_token("Repo.git", &opts), "repo.git");
}

#[test]
fn identity_key_equivalences() {
    let opts = PathIdentityOptions {
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    let keys = [
        path_identity_key(r"C:\Users\Floris\Repo", opts).unwrap(),
        path_identity_key(r"c:/users/floris/repo", opts).unwrap(),
        path_identity_key(r"C:/Users/Floris/Repo/", opts).unwrap(),
        path_identity_key(r"C:\Users\Floris\Repo\.", opts).unwrap(),
    ];
    assert!(keys.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn identity_preserves_case_on_unix_policy() {
    let opts = PathIdentityOptions {
        case: CaseNormalization::Preserve,
        ..PathIdentityOptions::default()
    };
    let a = path_identity_key("/tmp/Foo", opts).unwrap();
    let b = path_identity_key("/tmp/foo", opts).unwrap();
    // On all platforms Preserve must keep distinction when separators match.
    assert_ne!(a, b);
}

#[test]
fn deduplicate_and_record() {
    let opts = PathIdentityOptions {
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    let paths = vec![
        PathBuf::from(r"C:\Users\Floris\Repo"),
        PathBuf::from(r"c:/users/floris/repo"),
    ];
    let d = deduplicate_paths(paths, opts).unwrap();
    assert_eq!(d.len(), 1);

    let rec = PathRecord::from_path(r"C:\Users\Floris\Repo", Some(opts)).unwrap();
    assert!(rec.identity_key.is_some());
    assert!(!rec.display.is_empty());
}

#[test]
fn directory_inspection() {
    let dir = tempdir().unwrap();
    assert!(is_existing_directory(dir.path()));
    assert!(directory_exists(dir.path()));
    let insp = inspect_directory(dir.path()).unwrap();
    assert!(insp.exists);
    assert!(insp.is_directory);
    assert!(!insp.is_symlink);
    assert!(require_directory(dir.path()).is_ok());

    let file = dir.path().join("f.txt");
    fs::write(&file, b"x").unwrap();
    assert!(!is_existing_directory(&file));
    assert!(require_directory(&file).is_err());
    assert!(!is_existing_directory(dir.path().join("missing")));
}

#[test]
fn executable_and_command_line() {
    let exec = ExecutableMatchOptions::default();
    assert!(
        executable_paths_match(
            r"C:\Program Files\XBP\xbp.exe",
            r"c:/program files/xbp/xbp.exe",
            exec
        )
        .unwrap()
    );

    let opts = CommandLinePathMatchOptions {
        case_insensitive: true,
        require_component_boundary: true,
        support_quotes: true,
        match_basename: false,
        ..CommandLinePathMatchOptions::default()
    };
    assert!(
        command_line_contains_path(
            r#"run "C:\Users\Floris\repo" --watch"#,
            r"C:\Users\Floris\repo",
            opts
        )
        .unwrap()
    );
    assert!(!command_line_contains_path(r"run C:\repository\x", r"C:\repo", opts).unwrap());
    assert!(
        command_line_contains_path(
            r"run /mnt/c/Users/Floris/repo",
            r"/mnt/c/Users/Floris/repo",
            CommandLinePathMatchOptions {
                support_wsl_translation: true,
                ..opts
            }
        )
        .unwrap()
    );
    assert!(command_line_contains_path("", r"C:\repo", opts).is_ok());
    assert!(command_line_contains_path("x", "", opts).is_err());
}
