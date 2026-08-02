//! Application root edge cases (generic overrides, never product-specific names).

use path_rs::{
    AppPathsOptions, AppRootPolicy, PathError, app_paths, app_paths_with_options,
    app_paths_with_policy,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn set_var(key: &str, val: impl AsRef<std::ffi::OsStr>) {
    unsafe { env::set_var(key, val) };
}
fn remove_var(key: &str) {
    unsafe { env::remove_var(key) };
}

#[test]
fn unset_override_uses_platform() {
    let var = "PATH_RS_EDGE_HOME_UNSET";
    remove_var(var);
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    assert!(p.root_dir.ends_with("edge-app"));
}

#[test]
fn relative_override_is_absolutized() {
    let dir = tempdir().unwrap();
    let prev = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    let var = "PATH_RS_EDGE_HOME_REL";
    set_var(var, "rel-root");
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: true,
        root_policy: None,
    })
    .unwrap();
    assert!(p.root_dir.is_absolute());
    assert!(p.root_dir.ends_with("rel-root"));
    assert!(p.root_dir.is_dir());
    remove_var(var);
    env::set_current_dir(prev).unwrap();
}

#[test]
fn override_with_dollar_home() {
    let var = "PATH_RS_EDGE_HOME_DOLLAR";
    // Ensure $HOME expands on all platforms (Windows may lack HOME by default).
    if env::var_os("HOME").is_none() {
        if let Ok(profile) = env::var("USERPROFILE") {
            set_var("HOME", &profile);
        } else {
            set_var("HOME", env::temp_dir().as_os_str());
        }
    }
    set_var(var, "$HOME/path-rs-edge-dollar");
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    assert!(p.root_dir.is_absolute());
    assert!(p.root_dir.to_string_lossy().contains("path-rs-edge-dollar"));
    remove_var(var);
}

#[test]
fn override_with_parent_components_normalized() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    let raw = nested.join("..").join("..").join("final");
    // absolute path with ..
    let var = "PATH_RS_EDGE_HOME_DOTDOT";
    set_var(var, raw.as_os_str());
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: true,
        root_policy: None,
    })
    .unwrap();
    assert!(p.root_dir.ends_with("final"));
    assert!(!p.root_dir.to_string_lossy().contains(".."));
    remove_var(var);
}

#[test]
fn separators_in_override_value() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("win-style");
    fs::create_dir_all(&root).unwrap();
    let var = "PATH_RS_EDGE_HOME_SEP";
    // Use native path; forward slashes also accepted by PathBuf on Windows.
    set_var(var, root.as_os_str());
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: false,
        root_policy: None,
    })
    .unwrap();
    assert_eq!(p.root_dir, path_rs::normalize(&root).unwrap());
    remove_var(var);
}

#[test]
fn root_policy_explicit_overrides_env_field() {
    let dir = tempdir().unwrap();
    let explicit = dir.path().join("explicit");
    let var = "PATH_RS_EDGE_HOME_IGNORED";
    set_var(var, dir.path().join("from-env").as_os_str());
    let p = app_paths_with_options(AppPathsOptions {
        application_name: "edge-app".into(),
        environment_override: Some(var.into()),
        create_directories: true,
        root_policy: Some(AppRootPolicy::Explicit {
            path: explicit.clone(),
        }),
    })
    .unwrap();
    assert_eq!(p.root_dir, path_rs::normalize(&explicit).unwrap());
    remove_var(var);
}

#[test]
fn app_paths_subdirs_under_override() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    let p = app_paths_with_policy(
        "edge-app",
        AppRootPolicy::Explicit { path: root.clone() },
        true,
    )
    .unwrap();
    assert_eq!(p.config_dir, path_rs::normalize(&root).unwrap());
    assert_eq!(p.data_dir, path_rs::normalize(&root).unwrap());
    assert!(p.cache_dir.ends_with("cache"));
    assert!(p.state_dir.as_ref().unwrap().ends_with("state"));
    // Product-specific subdirs remain the caller's concern:
    let custom = p.root_dir.join("custom-subdir");
    assert!(!custom.exists());
}

#[test]
fn platform_paths_distinct_from_cwd_and_temp() {
    let p = app_paths("edge-app-platform").unwrap();
    let cwd = env::current_dir().unwrap();
    let temp = env::temp_dir();
    assert_ne!(p.root_dir, cwd);
    // temp_dir field is system temp, not app root
    assert_eq!(p.temp_dir, temp);
    assert_ne!(p.root_dir, p.temp_dir);
}

#[test]
fn invalid_names_exhaustive() {
    for name in ["", ".", "..", "a/b", "a\\b", "CON", "nul", "COM1", "LPT9"] {
        assert!(app_paths(name).is_err(), "expected rejection for {name:?}");
    }
    assert!(app_paths("valid-app_1").is_ok());
}

#[test]
fn create_false_leaves_missing() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("ghost");
    let p = app_paths_with_policy(
        "edge-app",
        AppRootPolicy::Explicit { path: root.clone() },
        false,
    )
    .unwrap();
    assert!(!p.root_dir.exists());
    assert!(!p.cache_dir.exists());
}

#[test]
fn file_root_errors_on_create_and_on_validate() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("file-root");
    fs::write(&file, b"x").unwrap();
    let err = app_paths_with_policy("edge-app", AppRootPolicy::Explicit { path: file }, false)
        .unwrap_err();
    assert!(matches!(err, PathError::InvalidPath { .. }));
}

#[test]
fn paths_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<path_rs::AppPaths>();
    assert_send_sync::<PathBuf>();
}
