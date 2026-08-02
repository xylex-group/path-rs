//! Directory existence checks and inspection summaries.

use path_rs::{
    directory_exists, inspect_directory, is_existing_directory, require_directory,
};

fn main() -> Result<(), path_rs::PathError> {
    // --- directory_exists / is_existing_directory ---
    println!("directory_exists(\".\")       = {}", directory_exists("."));
    println!(
        "is_existing_directory(\".\")  = {}",
        is_existing_directory(".")
    );
    println!(
        "directory_exists(\"nope\")    = {}",
        directory_exists("nope-missing-path-rs")
    );

    // --- inspect_directory ---
    let info = inspect_directory(".")?;
    println!("inspect_directory(\".\"):");
    println!("  exists       = {}", info.exists);
    println!("  is_directory = {}", info.is_directory);
    println!("  is_symlink   = {}", info.is_symlink);
    println!("  is_readable  = {:?}", info.is_readable);
    if let Some(meta) = &info.metadata {
        println!("  len          = {}", meta.len);
        println!("  modified     = {:?}", meta.modified);
        println!("  readonly     = {}", meta.readonly);
    }

    // --- require_directory (errors if missing or not a directory) ---
    let dir = require_directory(".")?;
    println!("require_directory(\".\")      = {}", dir.display());

    Ok(())
}
