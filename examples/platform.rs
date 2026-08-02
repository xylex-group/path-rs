//! Windows path classification, reserved names, WSL translation, display simplify.

use path_rs::{
    is_device_namespace, is_drive_relative, is_reserved_windows_name, is_unc, is_verbatim,
    path_contains_reserved_name, simplify_for_display, translate_wsl_path,
};
use std::ffi::OsStr;
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    // These helpers inspect syntax only — no filesystem access.
    // On non-Windows, UNC / verbatim / drive-relative classification typically returns false.

    let samples: &[(&str, &str)] = &[
        (r"C:\repo", "drive-absolute"),
        (r"C:foo", "drive-relative"),
        (r"\\server\share\path", "UNC"),
        (r"\\?\C:\repo", "verbatim"),
        (r"\\.\pipe\name", "device namespace"),
        (r"folder\NUL.txt", "reserved component"),
    ];

    for (raw, label) in samples {
        let p = Path::new(raw);
        println!("{label}: {raw}");
        println!("  is_drive_relative          = {}", is_drive_relative(p));
        println!("  is_unc                     = {}", is_unc(p));
        println!("  is_verbatim                = {}", is_verbatim(p));
        println!("  is_device_namespace        = {}", is_device_namespace(p));
        println!(
            "  path_contains_reserved_name = {}",
            path_contains_reserved_name(p)
        );
    }

    println!(
        "is_reserved_windows_name(\"CON\") = {}",
        is_reserved_windows_name(OsStr::new("CON"))
    );
    println!(
        "is_reserved_windows_name(\"file\") = {}",
        is_reserved_windows_name(OsStr::new("file.txt"))
    );

    // --- translate_wsl_path ---
    match translate_wsl_path("/mnt/c/Users/demo")? {
        Some(win) => println!("translate_wsl_path => {}", win.display()),
        None => println!("translate_wsl_path => None (not a WSL mount)"),
    }
    println!(
        "translate_wsl_path(\"/home/x\") => {:?}",
        translate_wsl_path("/home/x")?
    );

    // --- simplify_for_display ---
    let simplified = simplify_for_display(Path::new(r"\\?\C:\repo"));
    println!("simplify_for_display(verbatim) => {}", simplified.display());

    Ok(())
}
