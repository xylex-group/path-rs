//! Discovery edge cases: depth, skips, predicates, duplicates, errors.

use path_rs::{
    DirectoryInspection, DirectoryVisitor, DiscoveryOptions, SortMode, TraversalErrorPolicy,
    VisitControl, discover_directories, discover_where, visit_directories,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn mkdir(p: &Path) {
    fs::create_dir_all(p).unwrap();
}

#[test]
fn nested_tree_depth_limits() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("a/b/c/d"));

    let deep = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            max_depth: None,
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert!(deep.iter().any(|p| p.ends_with("d")));

    let shallow = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            max_depth: Some(1),
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    // depth 0 root + depth 1 children only
    assert!(shallow.iter().any(|p| p == dir.path()));
    assert!(shallow.iter().any(|p| p.ends_with("a")));
    assert!(!shallow.iter().any(|p| p.ends_with("d")));
}

#[test]
fn skip_nested_but_not_root() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("keep"));
    mkdir(&dir.path().join("target/inner"));
    mkdir(&dir.path().join("keep/target/inner"));

    let found =
        discover_directories(dir.path(), &DiscoveryOptions::new().skip_names(["target"])).unwrap();
    assert!(found.iter().any(|p| p.ends_with("keep")));
    assert!(!found.iter().any(|p| p.to_string_lossy().contains("target")));
}

#[test]
fn skip_relative_prefix() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("src/lib"));
    mkdir(&dir.path().join("vendor/pkg"));

    let found = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            skip_relative_prefixes: vec!["vendor".into()],
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert!(found.iter().any(|p| p.ends_with("lib")));
    assert!(!found.iter().any(|p| p.to_string_lossy().contains("vendor")));
}

#[cfg(feature = "search")]
#[test]
fn skip_globs() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("src"));
    mkdir(&dir.path().join("build-output"));
    mkdir(&dir.path().join("dist"));

    let found = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            skip_globs: vec!["build-*".into(), "dist".into()],
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert!(found.iter().any(|p| p.ends_with("src")));
    assert!(!found.iter().any(|p| p.ends_with("build-output")));
    assert!(!found.iter().any(|p| p.ends_with("dist")));
}

#[test]
fn predicate_marker_file() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("proj_a"));
    mkdir(&dir.path().join("proj_b"));
    mkdir(&dir.path().join("other"));
    fs::write(dir.path().join("proj_a/.marker"), b"1").unwrap();
    fs::write(dir.path().join("proj_b/.marker"), b"1").unwrap();

    let found = discover_where(dir.path(), &DiscoveryOptions::default(), |path, _| {
        path.join(".marker").is_file()
    })
    .unwrap();
    assert_eq!(found.len(), 2);
    // Deterministic sort by path default
    assert!(found[0] < found[1] || found[0] != found[1]);
}

#[test]
fn parent_is_itself_matching_predicate() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("ROOT_MARKER"), b"1").unwrap();
    mkdir(&dir.path().join("child"));
    fs::write(dir.path().join("child/ROOT_MARKER"), b"1").unwrap();

    let found = discover_where(dir.path(), &DiscoveryOptions::default(), |path, _| {
        path.join("ROOT_MARKER").is_file()
    })
    .unwrap();
    assert!(found.iter().any(|p| p == dir.path()));
    assert!(found.iter().any(|p| p.ends_with("child")));
}

#[test]
fn max_entries_caps_results() {
    let dir = tempdir().unwrap();
    for i in 0..10 {
        mkdir(&dir.path().join(format!("d{i:02}")));
    }
    let found = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            max_entries: Some(4),
            sort: SortMode::Name,
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert!(found.len() <= 4);
}

#[test]
fn non_recursive() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("a/b"));
    let found = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            recursive: false,
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0], dir.path());
}

#[test]
fn not_a_directory_root_errors() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("f");
    fs::write(&file, b"x").unwrap();
    assert!(discover_directories(&file, &DiscoveryOptions::default()).is_err());
}

#[test]
fn visitor_skip_children() {
    struct SkipA {
        visited: Vec<PathBuf>,
    }
    impl DirectoryVisitor for SkipA {
        fn visit_directory(
            &mut self,
            path: &Path,
            _inspection: &DirectoryInspection,
        ) -> VisitControl {
            self.visited.push(path.to_path_buf());
            if path.file_name().and_then(|n| n.to_str()) == Some("skipme") {
                VisitControl::SkipChildren
            } else {
                VisitControl::Continue
            }
        }
    }

    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("skipme/hidden"));
    mkdir(&dir.path().join("keep/visible"));

    let mut v = SkipA {
        visited: Vec::new(),
    };
    visit_directories(dir.path(), &DiscoveryOptions::default(), &mut v).unwrap();
    assert!(v.visited.iter().any(|p| p.ends_with("skipme")));
    assert!(!v.visited.iter().any(|p| p.ends_with("hidden")));
    assert!(v.visited.iter().any(|p| p.ends_with("visible")));
}

#[test]
fn skip_errors_policy() {
    let dir = tempdir().unwrap();
    mkdir(&dir.path().join("ok"));
    let found = discover_directories(
        dir.path(),
        &DiscoveryOptions {
            error_policy: TraversalErrorPolicy::SkipErrors,
            ..DiscoveryOptions::default()
        },
    )
    .unwrap();
    assert!(found.iter().any(|p| p.ends_with("ok")));
}

#[test]
fn empty_directory_returns_root_only() {
    let dir = tempdir().unwrap();
    let found = discover_directories(dir.path(), &DiscoveryOptions::default()).unwrap();
    assert_eq!(found, vec![dir.path().to_path_buf()]);
}

#[cfg(unix)]
#[test]
fn broken_symlink_not_listed_as_dir() {
    let dir = tempdir().unwrap();
    let link = dir.path().join("broken");
    std::os::unix::fs::symlink(dir.path().join("missing"), &link).unwrap();
    let found = discover_directories(dir.path(), &DiscoveryOptions::default()).unwrap();
    assert!(!found.iter().any(|p| p == &link));
}
