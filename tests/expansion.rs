use path_rs::{
    ExpandOptions, PathError, expand_dollar_variables, expand_input, expand_percent_variables,
    expand_tilde,
};
use std::env;
use std::path::PathBuf;

#[test]
fn empty_and_whitespace() {
    let opts = ExpandOptions::default();
    assert!(matches!(
        expand_input("", &opts),
        Err(PathError::EmptyInput)
    ));
    assert!(matches!(
        expand_input("   ", &opts),
        Err(PathError::EmptyInput)
    ));
}

#[test]
fn tilde_expansion() {
    let opts = ExpandOptions::default();
    let home = expand_input("~", &opts).unwrap();
    assert!(home.is_absolute() || !home.as_os_str().is_empty());

    let project = expand_input("~/project", &opts).unwrap();
    assert!(project.ends_with("project"));

    // Mid-path tilde must not expand.
    let mid = expand_tilde("foo/~/bar").unwrap();
    assert_eq!(mid, "foo/~/bar");

    // ~user is not expanded.
    let other = expand_tilde("~other/foo").unwrap();
    assert_eq!(other, "~other/foo");
}

#[test]
fn percent_variables() {
    // SAFETY: test process is single-threaded for this env mutation.
    unsafe {
        env::set_var("PATH_RS_TEST_VAR", "hello");
    }
    let s = expand_percent_variables("%PATH_RS_TEST_VAR%/x", true).unwrap();
    assert!(s.starts_with("hello"));

    assert!(expand_percent_variables("%", true).is_err());
    assert!(expand_percent_variables("%APPDATA", true).is_err());
    // Trailing `%` starts an unclosed percent token in strict mode.
    assert!(expand_percent_variables("APPDATA%", true).is_err());

    let escaped = expand_percent_variables("100%%", true).unwrap();
    assert_eq!(escaped, "100%");

    unsafe {
        env::set_var("PATH_RS_A", "A");
        env::set_var("PATH_RS_B", "B");
    }
    let multi = expand_percent_variables("%PATH_RS_A%%PATH_RS_B%", true).unwrap();
    assert_eq!(multi, "AB");

    assert!(matches!(
        expand_percent_variables("%PATH_RS_UNDEFINED_XYZ%", true),
        Err(PathError::UndefinedEnvironmentVariable { .. })
    ));

    let permissive = expand_percent_variables("%PATH_RS_UNDEFINED_XYZ%", false).unwrap();
    assert_eq!(permissive, "%PATH_RS_UNDEFINED_XYZ%");

    unsafe {
        env::remove_var("PATH_RS_TEST_VAR");
        env::remove_var("PATH_RS_A");
        env::remove_var("PATH_RS_B");
    }
}

#[test]
fn dollar_variables() {
    unsafe {
        env::set_var("PATH_RS_HOME_LIKE", "/tmp/pathrs");
    }
    assert_eq!(
        expand_dollar_variables("$PATH_RS_HOME_LIKE/x", true).unwrap(),
        "/tmp/pathrs/x"
    );
    assert_eq!(
        expand_dollar_variables("${PATH_RS_HOME_LIKE}/x", true).unwrap(),
        "/tmp/pathrs/x"
    );

    assert!(expand_dollar_variables("${HOME", true).is_err());
    assert_eq!(expand_dollar_variables("$123", true).unwrap(), "$123");
    assert_eq!(
        expand_dollar_variables("$(whoami)", true).unwrap(),
        "$(whoami)"
    );

    unsafe {
        env::set_var("_PATH_RS", "ok");
    }
    assert_eq!(expand_dollar_variables("$_PATH_RS", true).unwrap(), "ok");
    unsafe {
        env::remove_var("_PATH_RS");
        env::remove_var("PATH_RS_HOME_LIKE");
    }
}

#[test]
fn expansion_options_disable() {
    let opts = ExpandOptions::none();
    let p = expand_input("~/nope", &opts).unwrap();
    assert_eq!(p, PathBuf::from("~/nope"));
}

#[test]
fn wsl_translation_opt_in() {
    let opts = ExpandOptions {
        translate_wsl_paths: true,
        expand_tilde: false,
        expand_percent_variables: false,
        expand_dollar_variables: false,
        ..ExpandOptions::default()
    };

    let p = expand_input("/mnt/c/Users/floris", &opts).unwrap();
    assert_eq!(p, PathBuf::from(r"C:\Users\floris"));

    let p2 = expand_input("/mnt/data/foo", &opts).unwrap();
    assert_eq!(p2, PathBuf::from("/mnt/data/foo"));
}
