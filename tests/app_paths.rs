use path_rs::{
    AppPathsOptions, AppRootPolicy, PathError, app_paths, app_paths_with_options,
    app_paths_with_policy,
};
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn rejects_invalid_app_names() {
    assert!(app_paths("").is_err());
    assert!(app_paths("a/b").is_err());
    assert!(app_paths("..").is_err());
    assert!(app_paths("CON").is_err());
}

#[test]
fn platform_default_does_not_use_cwd_as_root() {
    let cwd = env::current_dir().unwrap();
    let paths = app_paths("path-rs-app-paths-test").unwrap();
    assert_ne!(paths.root_dir, cwd);
    assert!(!paths.root_dir.starts_with(&cwd) || paths.root_dir != cwd);
}

#[test]
fn environment_override_absolute() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("override-root");
    fs::create_dir_all(&root).unwrap();
    let var = "PATH_RS_TEST_APP_HOME";
    unsafe {
        env::set_var(var, root.as_os_str());
    }
    let paths = app_paths_with_options(AppPathsOptions {
        application_name: "my-tool".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    assert_eq!(paths.root_dir, path_rs::normalize(&root).unwrap());
    unsafe {
        env::remove_var(var);
    }
}

#[test]
fn environment_override_empty_falls_back() {
    let var = "PATH_RS_TEST_APP_HOME_EMPTY";
    unsafe {
        env::set_var(var, "   ");
    }
    let paths = app_paths_with_options(AppPathsOptions {
        application_name: "my-tool".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    // Platform default data dir ends with app name.
    assert!(paths.root_dir.ends_with("my-tool"));
    unsafe {
        env::remove_var(var);
    }
}

#[test]
fn environment_override_with_tilde() {
    let var = "PATH_RS_TEST_APP_HOME_TILDE";
    unsafe {
        env::set_var(var, "~/path-rs-override-test-dir");
    }
    let paths = app_paths_with_options(AppPathsOptions {
        application_name: "my-tool".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    assert!(paths.root_dir.is_absolute());
    assert!(
        paths
            .root_dir
            .to_string_lossy()
            .contains("path-rs-override-test-dir")
    );
    unsafe {
        env::remove_var(var);
    }
}

#[test]
fn override_pointing_at_file_errors() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    fs::write(&file, b"x").unwrap();
    let var = "PATH_RS_TEST_APP_HOME_FILE";
    unsafe {
        env::set_var(var, file.as_os_str());
    }
    let err = app_paths_with_options(AppPathsOptions {
        application_name: "my-tool".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap_err();
    assert!(matches!(err, PathError::InvalidPath { .. }));
    unsafe {
        env::remove_var(var);
    }
}

#[test]
fn create_directories_true() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("created");
    let paths = app_paths_with_policy(
        "my-tool",
        AppRootPolicy::Explicit { path: root.clone() },
        true,
    )
    .unwrap();
    assert!(paths.root_dir.is_dir());
    assert!(paths.cache_dir.is_dir());
}

#[test]
fn create_directories_false_does_not_create() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("not-created-yet");
    let paths = app_paths_with_policy(
        "my-tool",
        AppRootPolicy::Explicit { path: root.clone() },
        false,
    )
    .unwrap();
    assert!(!paths.root_dir.exists());
}
