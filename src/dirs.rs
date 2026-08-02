//! Platform directory locations and application-level roots.
//!
//! Uses the `dirs` crate for platform-native resolution. Environment variable
//! expansion (`%APPDATA%`, `$XDG_CONFIG_HOME`, etc.) is separate — see [`crate::expand`].
//!
//! # Global application root vs repository root
//!
//! - **Application root** ([`AppPaths`]): user/global state for a tool (config, data, cache).
//! - **Repository root**: a project working tree discovered by the application.
//! - **Current working directory**: process-local; never used as the global app root unless
//!   an explicit policy says so.
//!
//! This module never places global application state under a repository automatically,
//! and never hardcodes product-specific subdirectory names.

use crate::error::PathError;
use crate::expand::{ExpandOptions, expand_input};
use crate::inspect::is_existing_directory;
use crate::internal::validation::validate_app_name;
use crate::normalize::normalize;
use crate::resolve::absolute;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Platform-standard directories for the current user / process.
///
/// # Platform notes
///
/// **Windows**
/// - Home: `%USERPROFILE%`
/// - Config / roaming data: `%APPDATA%`
/// - Local data / cache: `%LOCALAPPDATA%`
/// - Temp: `%TEMP%` or `%TMP%`
///
/// **macOS**
/// - Home: `$HOME`
/// - Config / data: `~/Library/Application Support`
/// - Cache: `~/Library/Caches`
/// - Temp: `$TMPDIR`
///
/// **Linux**
/// - Home: `$HOME`
/// - Config: `$XDG_CONFIG_HOME` or `~/.config`
/// - Data: `$XDG_DATA_HOME` or `~/.local/share`
/// - Cache: `$XDG_CACHE_HOME` or `~/.cache`
/// - State: `$XDG_STATE_HOME` or `~/.local/state` when available
/// - Runtime: `$XDG_RUNTIME_DIR` when available
/// - Temp: `$TMPDIR` or system temp
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDirs {
    /// User home directory.
    pub home: PathBuf,
    /// User configuration directory.
    pub config: PathBuf,
    /// User data directory.
    pub data: PathBuf,
    /// User cache directory.
    pub cache: PathBuf,
    /// User state directory (XDG state), when available.
    pub state: Option<PathBuf>,
    /// Runtime directory (XDG runtime), when available.
    pub runtime: Option<PathBuf>,
    /// Temporary directory.
    pub temp: PathBuf,
}

/// Resolve standard platform directories for the current user.
///
/// # Filesystem access
///
/// May consult environment variables and platform APIs. Does not create directories.
pub fn platform_dirs() -> Result<PlatformDirs, PathError> {
    let home = dirs::home_dir().ok_or(PathError::HomeDirectoryUnavailable)?;
    let config = dirs::config_dir()
        .ok_or_else(|| PathError::invalid("config directory is unavailable on this platform"))?;
    let data = dirs::data_dir()
        .ok_or_else(|| PathError::invalid("data directory is unavailable on this platform"))?;
    let cache = dirs::cache_dir()
        .ok_or_else(|| PathError::invalid("cache directory is unavailable on this platform"))?;
    let state = dirs::state_dir();
    let runtime = dirs::runtime_dir();
    let temp = std::env::temp_dir();

    Ok(PlatformDirs {
        home,
        config,
        data,
        cache,
        state,
        runtime,
        temp,
    })
}

/// Application-specific configuration directory: `{config}/{app_name}`.
///
/// Validates `app_name` (no separators, no reserved names, non-empty).
/// Does not create the directory.
pub fn config_dir(app_name: &str) -> Result<PathBuf, PathError> {
    validate_app_name(app_name)?;
    let base = dirs::config_dir()
        .ok_or_else(|| PathError::invalid("config directory is unavailable on this platform"))?;
    Ok(base.join(app_name))
}

/// Application-specific data directory: `{data}/{app_name}`.
///
/// Does not create the directory.
pub fn data_dir(app_name: &str) -> Result<PathBuf, PathError> {
    validate_app_name(app_name)?;
    let base = dirs::data_dir()
        .ok_or_else(|| PathError::invalid("data directory is unavailable on this platform"))?;
    Ok(base.join(app_name))
}

/// Application-specific cache directory: `{cache}/{app_name}`.
///
/// Does not create the directory.
pub fn cache_dir(app_name: &str) -> Result<PathBuf, PathError> {
    validate_app_name(app_name)?;
    let base = dirs::cache_dir()
        .ok_or_else(|| PathError::invalid("cache directory is unavailable on this platform"))?;
    Ok(base.join(app_name))
}

/// Temporary directory for the process.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Application-level platform roots for a named tool.
///
/// Callers may join product-specific subdirectories under [`Self::root_dir`].
/// Those subdirectory names are **not** defined by this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Validated application name.
    pub application_name: String,
    /// Primary application root directory.
    pub root_dir: PathBuf,
    /// Configuration directory for the application.
    pub config_dir: PathBuf,
    /// Data directory for the application.
    pub data_dir: PathBuf,
    /// Cache directory for the application.
    pub cache_dir: PathBuf,
    /// State directory when available.
    pub state_dir: Option<PathBuf>,
    /// Temporary directory (system temp unless rooted under an explicit override).
    pub temp_dir: PathBuf,
}

/// How to choose the application root.
///
/// Does not hardcode product environment variables; callers pass the variable name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AppRootPolicy {
    /// Use platform-native application directories (`dirs`).
    PlatformDefault,
    /// Prefer an environment variable when set and non-empty; else platform default.
    EnvironmentOverride {
        /// Name of the environment variable (not its value).
        variable: String,
    },
    /// Use an explicit root path (expanded/normalized by the caller or this API).
    Explicit {
        /// Explicit application root.
        path: PathBuf,
    },
}

/// Options for resolving [`AppPaths`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPathsOptions {
    /// Application name (validated).
    pub application_name: String,
    /// Optional environment variable name used as root override when set.
    ///
    /// Equivalent to [`AppRootPolicy::EnvironmentOverride`] when `Some`.
    pub environment_override: Option<String>,
    /// When true, create the resolved directories.
    pub create_directories: bool,
    /// Explicit root policy. When set, takes precedence over `environment_override`
    /// alone for the policy shape; if both are present, `root_policy` wins.
    pub root_policy: Option<AppRootPolicy>,
}

impl AppPathsOptions {
    /// Build options for `application_name` with platform defaults.
    pub fn new(application_name: impl Into<String>) -> Self {
        Self {
            application_name: application_name.into(),
            environment_override: None,
            create_directories: false,
            root_policy: None,
        }
    }
}

/// Resolve application paths using platform defaults (no directory creation).
///
/// # Filesystem access
///
/// Consults platform directory APIs. Does not create directories.
/// Does not use the process current directory as the global root.
pub fn app_paths(application_name: &str) -> Result<AppPaths, PathError> {
    app_paths_with_options(AppPathsOptions {
        application_name: application_name.to_owned(),
        environment_override: None,
        create_directories: false,
        root_policy: Some(AppRootPolicy::PlatformDefault),
    })
}

/// Resolve application paths with explicit options.
///
/// # Environment override
///
/// When an override variable is set and non-empty, its value is expanded with
/// [`expand_input`] (tilde, `%VAR%`, `$VAR`) and used as the application root.
/// Relative override values are made absolute against the current directory.
///
/// # Filesystem access
///
/// Platform directory lookup always. Optional directory creation when requested.
/// May read the current directory only to absolutize a **relative** override.
pub fn app_paths_with_options(options: AppPathsOptions) -> Result<AppPaths, PathError> {
    validate_app_name(&options.application_name)?;

    let policy = options.root_policy.clone().unwrap_or_else(|| {
        if let Some(var) = &options.environment_override {
            AppRootPolicy::EnvironmentOverride {
                variable: var.clone(),
            }
        } else {
            AppRootPolicy::PlatformDefault
        }
    });

    let mut paths = match policy {
        AppRootPolicy::PlatformDefault => platform_app_paths(&options.application_name)?,
        AppRootPolicy::Explicit { path } => rooted_app_paths(&options.application_name, path)?,
        AppRootPolicy::EnvironmentOverride { variable } => match env::var(&variable) {
            Ok(value) if !value.trim().is_empty() => {
                let expanded = expand_input(
                    value.trim(),
                    &ExpandOptions {
                        reject_undefined_variables: true,
                        ..ExpandOptions::default()
                    },
                )?;
                let abs = if expanded.is_absolute() {
                    normalize(expanded)?
                } else {
                    absolute(expanded)?
                };
                rooted_app_paths(&options.application_name, abs)?
            }
            Ok(_) | Err(env::VarError::NotPresent) => {
                platform_app_paths(&options.application_name)?
            }
            Err(env::VarError::NotUnicode(_)) => {
                return Err(PathError::invalid(format!(
                    "environment variable {variable} is not valid Unicode"
                )));
            }
        },
    };

    if options.create_directories {
        create_app_directories(&paths)?;
    } else {
        // Soft validation: if root exists and is a file, error early.
        if paths.root_dir.exists() && !is_existing_directory(&paths.root_dir) {
            return Err(PathError::invalid(format!(
                "application root exists and is not a directory: {}",
                paths.root_dir.to_string_lossy()
            )));
        }
    }

    // Re-check after create.
    if options.create_directories && !is_existing_directory(&paths.root_dir) {
        return Err(PathError::invalid(format!(
            "failed to materialize application root as a directory: {}",
            paths.root_dir.to_string_lossy()
        )));
    }

    let _ = &mut paths;
    Ok(paths)
}

/// Resolve paths using an explicit [`AppRootPolicy`].
pub fn app_paths_with_policy(
    application_name: &str,
    policy: AppRootPolicy,
    create_directories: bool,
) -> Result<AppPaths, PathError> {
    app_paths_with_options(AppPathsOptions {
        application_name: application_name.to_owned(),
        environment_override: None,
        create_directories,
        root_policy: Some(policy),
    })
}

fn platform_app_paths(application_name: &str) -> Result<AppPaths, PathError> {
    let data = data_dir(application_name)?;
    let config = config_dir(application_name)?;
    let cache = cache_dir(application_name)?;
    let state = dirs::state_dir().map(|s| s.join(application_name));
    Ok(AppPaths {
        application_name: application_name.to_owned(),
        // Primary root is the data directory for the application.
        root_dir: data.clone(),
        config_dir: config,
        data_dir: data,
        cache_dir: cache,
        state_dir: state,
        temp_dir: temp_dir(),
    })
}

fn rooted_app_paths(application_name: &str, root: PathBuf) -> Result<AppPaths, PathError> {
    let root = normalize(root)?;
    if root.as_os_str().is_empty() {
        return Err(PathError::EmptyInput);
    }
    Ok(AppPaths {
        application_name: application_name.to_owned(),
        config_dir: root.clone(),
        data_dir: root.clone(),
        cache_dir: root.join("cache"),
        state_dir: Some(root.join("state")),
        temp_dir: temp_dir(),
        root_dir: root,
    })
}

fn create_app_directories(paths: &AppPaths) -> Result<(), PathError> {
    for dir in [
        Some(paths.root_dir.as_path()),
        Some(paths.config_dir.as_path()),
        Some(paths.data_dir.as_path()),
        Some(paths.cache_dir.as_path()),
        paths.state_dir.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        create_dir_if_needed(dir)?;
    }
    Ok(())
}

fn create_dir_if_needed(path: &Path) -> Result<(), PathError> {
    if path.exists() {
        if is_existing_directory(path) {
            return Ok(());
        }
        return Err(PathError::invalid(format!(
            "path exists and is not a directory: {}",
            path.to_string_lossy()
        )));
    }
    fs::create_dir_all(path).map_err(|e| PathError::filesystem(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_dirs_available() {
        let d = platform_dirs().unwrap();
        assert!(!d.home.as_os_str().is_empty());
        assert!(!d.temp.as_os_str().is_empty());
    }

    #[test]
    fn app_name_validation() {
        assert!(config_dir("").is_err());
        assert!(config_dir("a/b").is_err());
        assert!(config_dir("..").is_err());
        assert!(config_dir("CON").is_err());
        assert!(config_dir("my-app").is_ok());
    }

    #[test]
    fn app_paths_platform() {
        let p = app_paths("path-rs-test-app").unwrap();
        assert_eq!(p.application_name, "path-rs-test-app");
        assert!(p.root_dir.ends_with("path-rs-test-app"));
    }

    #[test]
    fn app_paths_explicit_and_create() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("app-root");
        let p = app_paths_with_policy(
            "my-tool",
            AppRootPolicy::Explicit { path: root.clone() },
            true,
        )
        .unwrap();
        assert_eq!(p.root_dir, normalize(&root).unwrap());
        assert!(p.root_dir.is_dir());
        assert!(p.cache_dir.is_dir());
    }
}
