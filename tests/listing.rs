use path_rs::{ListOptions, SortMode, list};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"data").unwrap();
}

#[test]
fn empty_directory() {
    let dir = tempdir().unwrap();
    let entries = list(dir.path(), &ListOptions::default()).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn nested_tree_and_hidden() {
    let dir = tempdir().unwrap();
    touch(&dir.path().join("a.txt"));
    touch(&dir.path().join("sub/b.txt"));
    touch(&dir.path().join(".hidden"));

    let non_recursive = list(dir.path(), &ListOptions::default()).unwrap();
    assert!(non_recursive.iter().any(|e| e.path.ends_with("a.txt")));
    assert!(!non_recursive.iter().any(|e| e.path.ends_with("b.txt")));
    assert!(!non_recursive.iter().any(|e| e.path.ends_with(".hidden")));

    let recursive = list(
        dir.path(),
        &ListOptions::new().recursive(true).include_hidden(true),
    )
    .unwrap();
    assert!(recursive.iter().any(|e| e.path.ends_with("b.txt")));
    assert!(recursive.iter().any(|e| e.path.ends_with(".hidden")));
}

#[test]
fn max_entries_and_sort() {
    let dir = tempdir().unwrap();
    touch(&dir.path().join("c.txt"));
    touch(&dir.path().join("a.txt"));
    touch(&dir.path().join("b.txt"));

    let opts = ListOptions::new().sort(SortMode::Name).max_entries(Some(2));
    let entries = list(dir.path(), &opts).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn spaces_and_unicode_names() {
    let dir = tempdir().unwrap();
    touch(&dir.path().join("file with spaces.txt"));
    touch(&dir.path().join("ünîcode.txt"));
    let entries = list(dir.path(), &ListOptions::default()).unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e.path.ends_with("file with spaces.txt"))
    );
    assert!(entries.iter().any(|e| e.path.ends_with("ünîcode.txt")));
}

#[cfg(unix)]
#[test]
fn symlink_and_broken() {
    use path_rs::EntryKind;

    let dir = tempdir().unwrap();
    let target = dir.path().join("target.txt");
    touch(&target);
    let link = dir.path().join("link.txt");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let broken = dir.path().join("broken");
    std::os::unix::fs::symlink(dir.path().join("missing"), &broken).unwrap();

    let entries = list(
        dir.path(),
        &ListOptions::new()
            .include_symlinks(true)
            .include_files(true),
    )
    .unwrap();
    assert!(entries.iter().any(|e| e.kind == EntryKind::Symlink));
}

#[cfg(unix)]
#[test]
fn skip_errors_policy() {
    use path_rs::TraversalErrorPolicy;

    // Create a directory and verify SkipErrors does not panic on walk.
    let dir = tempdir().unwrap();
    touch(&dir.path().join("ok.txt"));
    let opts = ListOptions::new()
        .recursive(true)
        .error_policy(TraversalErrorPolicy::SkipErrors);
    let _ = list(dir.path(), &opts).unwrap();
}
