//! Path identity keys, display strings, records, and deduplication.

use path_rs::{
    CaseNormalization, PathIdentityOptions, PathRecord, deduplicate_paths, path_display_string,
    path_identity_key,
};
use std::path::{Path, PathBuf};

fn main() -> Result<(), path_rs::PathError> {
    let opts = PathIdentityOptions {
        case: CaseNormalization::PlatformDefault,
        ..PathIdentityOptions::new()
    };

    // --- path_identity_key (comparison only — not for I/O) ---
    let key_a = path_identity_key(Path::new("Foo/./Bar"), opts)?;
    let key_b = path_identity_key(Path::new("Foo/Bar"), opts)?;
    println!("path_identity_key(\"Foo/./Bar\") = {key_a}");
    println!("path_identity_key(\"Foo/Bar\")   = {key_b}");
    println!("keys equal = {}", key_a == key_b);

    // --- path_display_string ---
    let display = path_display_string(Path::new("src/lib.rs"));
    println!("path_display_string = {display}");

    // --- PathRecord ---
    let record = PathRecord::from_path("src/lib.rs", Some(opts))?;
    println!(
        "PathRecord path={}, display={}, key={:?}",
        record.path.display(),
        record.display,
        record.identity_key
    );

    // --- deduplicate_paths ---
    let paths = [
        PathBuf::from("Foo/Bar"),
        PathBuf::from("Foo/./Bar"),
        PathBuf::from("other"),
    ];
    let unique = deduplicate_paths(paths, opts)?;
    println!(
        "deduplicate_paths => {:?}",
        unique
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    Ok(())
}
