//! Executable path comparison and command-line path detection.

use path_rs::{
    CommandLinePathMatchOptions, ExecutableMatchOptions, command_line_contains_path,
    executable_paths_match,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    // --- executable_paths_match ---
    let match_opts = ExecutableMatchOptions::new();
    let same = executable_paths_match(
        Path::new("C:/Tools/app.exe"),
        Path::new(r"C:\Tools\app.exe"),
        match_opts,
    )?;
    println!("executable_paths_match (slash variants) = {same}");

    let different = executable_paths_match(
        Path::new("/usr/bin/true"),
        Path::new("/usr/bin/false"),
        match_opts,
    )?;
    println!("executable_paths_match (different)      = {different}");

    // --- command_line_contains_path ---
    let cli_opts = CommandLinePathMatchOptions::new();
    let cmdline = r#"tool --input "C:\repo\src\main.rs" --flag"#;
    let found = command_line_contains_path(cmdline, Path::new(r"C:\repo\src\main.rs"), cli_opts)?;
    println!("command_line_contains_path              = {found}");

    let missing =
        command_line_contains_path(cmdline, Path::new(r"C:\other\file.txt"), cli_opts)?;
    println!("command_line_contains_path (missing)    = {missing}");

    Ok(())
}
