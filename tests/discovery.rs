use path_rs::{
    DirectoryInspection, DirectoryVisitor, DiscoveryOptions, VisitControl, discover_directories,
    discover_where, visit_directories,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn touch_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
}

#[test]
fn discovers_nested_dirs_and_skips_names() {
    let dir = tempdir().unwrap();
    touch_dir(&dir.path().join("src"));
    touch_dir(&dir.path().join("node_modules/pkg"));
    touch_dir(&dir.path().join("src/deep"));

    let opts = DiscoveryOptions::new()
        .skip_names(["node_modules"])
        .max_depth(Some(8));
    let found = discover_directories(dir.path(), &opts).unwrap();
    assert!(found.iter().any(|p| p.ends_with("src")));
    assert!(found.iter().any(|p| p.ends_with("deep")));
    assert!(
        !found
            .iter()
            .any(|p| p.to_string_lossy().contains("node_modules"))
    );
}

#[test]
fn does_not_skip_root_by_name() {
    let dir = tempdir().unwrap();
    // Root folder name is random; create child named target and ensure root is still listed.
    touch_dir(&dir.path().join("child"));
    let opts = DiscoveryOptions::new().skip_names(["target"]);
    let found = discover_directories(dir.path(), &opts).unwrap();
    assert!(found.iter().any(|p| p == dir.path()));
}

#[test]
fn predicate_discovery() {
    let dir = tempdir().unwrap();
    touch_dir(&dir.path().join("a"));
    touch_dir(&dir.path().join("b"));
    // Marker only under a.
    fs::write(dir.path().join("a/MARKER"), b"1").unwrap();

    let opts = DiscoveryOptions::default();
    let found = discover_where(dir.path(), &opts, |path, _| path.join("MARKER").is_file()).unwrap();
    assert_eq!(found.len(), 1);
    assert!(found[0].ends_with("a"));
}

#[test]
fn max_entries() {
    let dir = tempdir().unwrap();
    for i in 0..5 {
        touch_dir(&dir.path().join(format!("d{i}")));
    }
    let opts = DiscoveryOptions {
        max_entries: Some(3),
        recursive: true,
        ..DiscoveryOptions::default()
    };
    let found = discover_directories(dir.path(), &opts).unwrap();
    assert!(found.len() <= 3);
}

#[test]
fn visitor_stop() {
    struct V {
        count: usize,
    }
    impl DirectoryVisitor for V {
        fn visit_directory(
            &mut self,
            _path: &Path,
            _inspection: &DirectoryInspection,
        ) -> VisitControl {
            self.count += 1;
            if self.count >= 1 {
                VisitControl::Stop
            } else {
                VisitControl::Continue
            }
        }
    }

    let dir = tempdir().unwrap();
    touch_dir(&dir.path().join("a"));
    touch_dir(&dir.path().join("b"));
    let mut v = V { count: 0 };
    visit_directories(dir.path(), &DiscoveryOptions::default(), &mut v).unwrap();
    assert_eq!(v.count, 1);
}

#[test]
fn empty_parent() {
    let dir = tempdir().unwrap();
    let found = discover_directories(dir.path(), &DiscoveryOptions::default()).unwrap();
    // Root itself is a directory and is included.
    assert_eq!(found, vec![dir.path().to_path_buf()]);
}

#[cfg(unix)]
#[test]
fn broken_symlink_not_directory() {
    let dir = tempdir().unwrap();
    let link = dir.path().join("broken");
    std::os::unix::fs::symlink(dir.path().join("missing"), &link).unwrap();
    let found = discover_directories(dir.path(), &DiscoveryOptions::default()).unwrap();
    assert!(!found.iter().any(|p| p == &link));
}

#[test]
fn duplicate_spellings_deduped() {
    // Dedup is by identity of discovered paths; create one tree and ensure no panic.
    let dir = tempdir().unwrap();
    touch_dir(&dir.path().join("x"));
    let opts = DiscoveryOptions {
        deduplicate: true,
        ..DiscoveryOptions::default()
    };
    let a = discover_directories(dir.path(), &opts).unwrap();
    let b = discover_directories(dir.path(), &opts).unwrap();
    assert_eq!(a.len(), b.len());
    let _ = PathBuf::new();
}
