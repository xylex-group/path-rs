//! Exhaustive expansion edge cases from the product matrix.

use path_rs::{
    ExpandOptions, PathError, expand_dollar_variables, expand_input, expand_percent_variables,
    expand_tilde,
};
use std::env;
use std::path::PathBuf;

fn set_var(key: &str, val: &str) {
    // SAFETY: tests mutate process env with unique keys; not concurrent.
    unsafe { env::set_var(key, val) };
}

fn remove_var(key: &str) {
    unsafe { env::remove_var(key) };
}

#[test]
fn basic_inputs_trim_and_empty() {
    let opts = ExpandOptions::default();
    assert!(matches!(
        expand_input("", &opts),
        Err(PathError::EmptyInput)
    ));
    assert!(matches!(
        expand_input("   \t  ", &opts),
        Err(PathError::EmptyInput)
    ));

    let no_trim = ExpandOptions {
        trim_cli_input: false,
        expand_tilde: false,
        expand_percent_variables: false,
        expand_dollar_variables: false,
        ..ExpandOptions::default()
    };
    // Whitespace-only without trim is not empty string of length 0 after "is_empty"
    // but "   " is not empty — should succeed as a path of spaces.
    let p = expand_input("   ", &no_trim).unwrap();
    assert_eq!(p, PathBuf::from("   "));
}

#[test]
fn tilde_matrix() {
    assert_eq!(expand_tilde("foo/~/bar").unwrap(), "foo/~/bar");
    assert_eq!(expand_tilde("~other/foo").unwrap(), "~other/foo");
    assert_eq!(expand_tilde("~~").unwrap(), "~~");
    assert!(!expand_tilde("~").unwrap().is_empty());

    let home = expand_tilde("~").unwrap();
    let slash = expand_tilde("~/x").unwrap();
    assert!(slash.starts_with(&home));
    assert!(slash.ends_with("x") || slash.ends_with("/x") || slash.ends_with("\\x"));

    let back = expand_tilde("~\\x").unwrap();
    assert!(back.starts_with(&home));
}

#[test]
fn percent_malformed_and_escape() {
    assert!(expand_percent_variables("%", true).is_err());
    assert!(expand_percent_variables("%APPDATA", true).is_err());
    assert!(expand_percent_variables("APPDATA%", true).is_err());
    assert_eq!(expand_percent_variables("%%", true).unwrap(), "%");
    assert_eq!(
        expand_percent_variables("100%% done", true).unwrap(),
        "100% done"
    );

    set_var("PATH_RS_E1", "A");
    set_var("PATH_RS_E2", "B");
    assert_eq!(
        expand_percent_variables("%PATH_RS_E1%%PATH_RS_E2%", true).unwrap(),
        "AB"
    );
    remove_var("PATH_RS_E1");
    remove_var("PATH_RS_E2");

    assert!(expand_percent_variables("%PATH_RS_NO_SUCH_VAR_ZZ%", true).is_err());
    assert_eq!(
        expand_percent_variables("%PATH_RS_NO_SUCH_VAR_ZZ%", false).unwrap(),
        "%PATH_RS_NO_SUCH_VAR_ZZ%"
    );
}

#[test]
fn dollar_malformed_and_command_sub() {
    assert!(expand_dollar_variables("${HOME", true).is_err());
    assert!(expand_dollar_variables("$", true).is_err());
    assert_eq!(expand_dollar_variables("$123", true).unwrap(), "$123");
    assert_eq!(
        expand_dollar_variables("$(whoami)/x", true).unwrap(),
        "$(whoami)/x"
    );
    assert_eq!(
        expand_dollar_variables("`whoami`", true).unwrap(),
        "`whoami`"
    );
    assert_eq!(expand_dollar_variables("$$", true).unwrap(), "$");

    set_var("PATH_RS_DVAR", "val");
    assert_eq!(
        expand_dollar_variables("foo/$PATH_RS_DVAR/bar", true).unwrap(),
        "foo/val/bar"
    );
    assert_eq!(
        expand_dollar_variables("foo/${PATH_RS_DVAR}/bar", true).unwrap(),
        "foo/val/bar"
    );
    set_var("_PATH_RS_US", "u");
    assert_eq!(expand_dollar_variables("$_PATH_RS_US", true).unwrap(), "u");
    remove_var("PATH_RS_DVAR");
    remove_var("_PATH_RS_US");

    assert!(expand_dollar_variables("${}", true).is_err());
    assert_eq!(
        expand_dollar_variables("${PATH_RS_NO_SUCH}", false).unwrap(),
        "${PATH_RS_NO_SUCH}"
    );
}

#[test]
fn embedded_nul_rejected() {
    let opts = ExpandOptions::default();
    assert!(matches!(
        expand_input("foo\0bar", &opts),
        Err(PathError::EmbeddedNul)
    ));
    assert!(matches!(
        expand_percent_variables("a\0b", true),
        Err(PathError::EmbeddedNul)
    ));
}

#[test]
fn expansion_depth_limit() {
    // Self-referential expansion could grow if values re-introduced tokens;
    // depth is capped. With a fixed value there is no infinite growth.
    set_var("PATH_RS_DEPTH", "ok");
    let opts = ExpandOptions {
        max_expansion_depth: 1,
        ..ExpandOptions::default()
    };
    let _ = expand_input("%PATH_RS_DEPTH%", &opts).unwrap();
    remove_var("PATH_RS_DEPTH");
}

#[test]
fn options_none_is_passthrough() {
    let p = expand_input("%NOT_EXPANDED%/$HOME/~", &ExpandOptions::none()).unwrap();
    assert_eq!(p, PathBuf::from("%NOT_EXPANDED%/$HOME/~"));
}

#[test]
fn mid_path_env_still_expands_when_enabled() {
    set_var("PATH_RS_MID", "mid");
    let opts = ExpandOptions {
        expand_tilde: false,
        ..ExpandOptions::default()
    };
    assert_eq!(
        expand_input("foo/%PATH_RS_MID%/bar", &opts).unwrap(),
        PathBuf::from("foo/mid/bar")
    );
    assert_eq!(
        expand_input("foo/$PATH_RS_MID/bar", &opts).unwrap(),
        PathBuf::from("foo/mid/bar")
    );
    remove_var("PATH_RS_MID");
}

#[test]
fn wsl_option_matrix() {
    let base = ExpandOptions {
        expand_tilde: false,
        expand_percent_variables: false,
        expand_dollar_variables: false,
        translate_wsl_paths: true,
        ..ExpandOptions::default()
    };
    assert_eq!(
        expand_input("/mnt/c/Users/x", &base).unwrap(),
        PathBuf::from(r"C:\Users\x")
    );
    assert_eq!(
        expand_input("/mnt/D/repo", &base).unwrap(),
        PathBuf::from(r"D:\repo")
    );
    // Not single-letter drive mounts → no translation
    assert_eq!(
        expand_input("/mnt/data/x", &base).unwrap(),
        PathBuf::from("/mnt/data/x")
    );
    assert!(expand_input("/mnt/", &base).is_err());

    let off = ExpandOptions {
        translate_wsl_paths: false,
        expand_tilde: false,
        expand_percent_variables: false,
        expand_dollar_variables: false,
        ..ExpandOptions::default()
    };
    assert_eq!(
        expand_input("/mnt/c/Users/x", &off).unwrap(),
        PathBuf::from("/mnt/c/Users/x")
    );
}
