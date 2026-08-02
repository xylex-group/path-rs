//! End-to-end workflows combining expansion, normalization, resolution, and listing.

use path_rs::{
    ExpandOptions, ListOptions, expand_input, join_relative, list, normalize, resolve_inside,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn expand_normalize_resolve_list() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("readme.md"), b"hi").unwrap();

    // Expand a relative path input (no env needed).
    let mut opts = ExpandOptions::none();
    opts.trim_cli_input = true;
    let expanded = expand_input("  ./readme.md  ", &opts).unwrap();
    let normalized = normalize(&expanded).unwrap();
    assert_eq!(normalized, std::path::PathBuf::from("readme.md"));

    let joined = join_relative(dir.path(), &normalized).unwrap();
    assert!(joined.ends_with("readme.md"));

    let inside = resolve_inside(dir.path(), "readme.md").unwrap();
    assert!(inside.ends_with("readme.md"));

    let entries = list(dir.path(), &ListOptions::default()).unwrap();
    assert!(entries.iter().any(|e| e.path.ends_with("readme.md")));
}

#[test]
fn security_escape_blocked() {
    let dir = tempdir().unwrap();
    assert!(resolve_inside(dir.path(), "../outside").is_err());
}
