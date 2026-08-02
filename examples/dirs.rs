//! Platform directories and application path roots.

use path_rs::{
    AppPathsOptions, AppRootPolicy, app_paths, app_paths_with_options, app_paths_with_policy,
    cache_dir, config_dir, data_dir, platform_dirs, temp_dir,
};

fn main() -> Result<(), path_rs::PathError> {
    // --- platform_dirs ---
    let dirs = platform_dirs()?;
    println!("home   = {}", dirs.home.display());
    println!("config = {}", dirs.config.display());
    println!("data   = {}", dirs.data.display());
    println!("cache  = {}", dirs.cache.display());
    if let Some(state) = &dirs.state {
        println!("state  = {}", state.display());
    }
    if let Some(runtime) = &dirs.runtime {
        println!("runtime= {}", runtime.display());
    }
    println!("temp   = {}", dirs.temp.display());

    // --- per-app directory helpers (do not create) ---
    println!("config_dir(demo) = {}", config_dir("path-rs-demo")?.display());
    println!("data_dir(demo)   = {}", data_dir("path-rs-demo")?.display());
    println!("cache_dir(demo)  = {}", cache_dir("path-rs-demo")?.display());
    println!("temp_dir()       = {}", temp_dir().display());

    // --- app_paths (platform default) ---
    let paths = app_paths("path-rs-demo")?;
    println!("app root   = {}", paths.root_dir.display());
    println!("app config = {}", paths.config_dir.display());
    println!("app data   = {}", paths.data_dir.display());
    println!("app cache  = {}", paths.cache_dir.display());

    // --- app_paths_with_options ---
    let opts = AppPathsOptions::new("path-rs-demo");
    let with_opts = app_paths_with_options(opts)?;
    println!("with_options root = {}", with_opts.root_dir.display());

    // --- app_paths_with_policy ---
    let with_policy = app_paths_with_policy(
        "path-rs-demo",
        AppRootPolicy::PlatformDefault,
        false, // do not create directories
    )?;
    println!("with_policy root  = {}", with_policy.root_dir.display());

    Ok(())
}
