//! Cross-module workflows: expand → normalize → resolve → discover → identity.

use path_rs::{
    AppRootPolicy, CaseNormalization, DiscoveryOptions, ExpandOptions, PathIdentityOptions,
    absolute, app_paths_with_policy, deduplicate_paths, discover_where, expand_input,
    is_existing_directory, join_relative, normalize, path_identity_key, resolve_inside,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn end_to_end_tooling_workflow() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("Cargo.toml"), b"[package]\nname=\"x\"\n").unwrap();
    fs::write(project.join("src/main.rs"), b"fn main(){}").unwrap();

    // 1. Expand CLI-like input (relative, no env)
    let expanded = expand_input(
        "  project/src/../Cargo.toml  ",
        &ExpandOptions {
            expand_tilde: false,
            expand_percent_variables: false,
            expand_dollar_variables: false,
            ..ExpandOptions::default()
        },
    )
    .unwrap();
    let clean = normalize(expanded).unwrap();
    assert_eq!(clean, PathBuf::from("project/Cargo.toml"));

    // 2. Join under workspace root
    let full = join_relative(dir.path(), &clean).unwrap();
    assert!(full.ends_with("Cargo.toml"));
    assert!(full.is_file() || fs::metadata(&full).is_ok());

    // 3. Containment
    let inside = resolve_inside(dir.path(), "project/src/main.rs").unwrap();
    assert!(inside.ends_with("main.rs"));
    assert!(resolve_inside(dir.path(), "../outside").is_err());

    // 4. Discover "project roots" by generic marker (Cargo.toml) — not Git
    let roots = discover_where(dir.path(), &DiscoveryOptions::default(), |path, _| {
        path.join("Cargo.toml").is_file()
    })
    .unwrap();
    assert_eq!(roots.len(), 1);
    assert!(roots[0].ends_with("project"));

    // 5. Identity dedup of duplicate spellings
    let opts = PathIdentityOptions {
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    };
    let keys_in = vec![
        roots[0].clone(),
        roots[0].clone(),
        absolute(&roots[0]).unwrap_or_else(|_| roots[0].clone()),
    ];
    let deduped = deduplicate_paths(keys_in, opts).unwrap();
    assert_eq!(deduped.len(), 1);

    let key = path_identity_key(&roots[0], opts).unwrap();
    assert!(!key.is_empty());

    // 6. App paths separate from repo root
    let app = app_paths_with_policy(
        "edge-integration-app",
        AppRootPolicy::Explicit {
            path: dir.path().join("global-app-root"),
        },
        true,
    )
    .unwrap();
    assert!(is_existing_directory(&app.root_dir));
    assert_ne!(app.root_dir, roots[0]);
    // Repo is not under global app root and vice versa by construction
    assert!(!roots[0].starts_with(&app.root_dir));
}

#[test]
fn parallel_safe_identity_on_many_paths() {
    use std::thread;

    let dir = tempdir().unwrap();
    let mut paths = Vec::new();
    for i in 0..20 {
        let p = dir.path().join(format!("p{i}"));
        fs::create_dir_all(&p).unwrap();
        paths.push(p);
    }

    let opts = PathIdentityOptions::default();
    let handles: Vec<_> = paths
        .into_iter()
        .map(|p| thread::spawn(move || path_identity_key(p, opts).unwrap()))
        .collect();

    let mut keys = Vec::new();
    for h in handles {
        keys.push(h.join().unwrap());
    }
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 20);
}
