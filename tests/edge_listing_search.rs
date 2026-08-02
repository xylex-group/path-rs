//! Listing and search edge cases.

use path_rs::{
    CacheMode, CacheOptions, CachePolicy, ListOptions, MemoryCache, SearchRequest, SortMode,
    TraversalErrorPolicy, list, search, search_with, search_with_cache, walk,
};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn write(path: &std::path::Path, data: &[u8]) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, data).unwrap();
}

#[test]
fn walk_is_streaming_not_prebuffered() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("a.txt"), b"a");
    write(&dir.path().join("b.txt"), b"b");
    write(&dir.path().join("c.txt"), b"c");

    let mut iter = walk(dir.path(), &ListOptions::new().max_entries(Some(2))).unwrap();
    let first = iter.next().unwrap().unwrap();
    assert!(
        first.path.ends_with("a.txt")
            || first.path.ends_with("b.txt")
            || first.path.ends_with("c.txt")
    );
    let second = iter.next().unwrap().unwrap();
    assert_ne!(first.path, second.path);
    assert!(iter.next().is_none());
}

#[test]
fn list_empty_hidden_recursive_limits() {
    let dir = tempdir().unwrap();
    assert!(
        list(dir.path(), &ListOptions::default())
            .unwrap()
            .is_empty()
    );

    write(&dir.path().join("a.txt"), b"a");
    write(&dir.path().join(".hidden"), b"h");
    write(&dir.path().join("sub/b.txt"), b"b");
    write(&dir.path().join("sub/.also"), b"h");

    let plain = list(dir.path(), &ListOptions::default()).unwrap();
    assert!(plain.iter().any(|e| e.path.ends_with("a.txt")));
    assert!(!plain.iter().any(|e| e.path.ends_with(".hidden")));
    assert!(!plain.iter().any(|e| e.path.ends_with("b.txt")));

    let rec = list(
        dir.path(),
        &ListOptions::new()
            .recursive(true)
            .include_hidden(true)
            .max_depth(Some(8)),
    )
    .unwrap();
    assert!(rec.iter().any(|e| e.path.ends_with("b.txt")));
    assert!(rec.iter().any(|e| e.path.ends_with(".hidden")));

    let limited = list(
        dir.path(),
        &ListOptions::new()
            .recursive(true)
            .include_hidden(true)
            .max_entries(Some(2))
            .sort(SortMode::Name),
    )
    .unwrap();
    assert_eq!(limited.len(), 2);
}

#[test]
fn list_unicode_and_spaces() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("file with spaces.txt"), b"1");
    write(&dir.path().join("ünîcode-名前.txt"), b"2");
    let entries = list(dir.path(), &ListOptions::default()).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn list_include_root() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("x"), b"1");
    let entries = list(
        dir.path(),
        &ListOptions {
            include_root: true,
            include_directories: true,
            include_files: true,
            ..ListOptions::default()
        },
    )
    .unwrap();
    assert!(entries.iter().any(|e| e.path == dir.path()));
}

#[test]
fn list_skip_errors_policy() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("ok.txt"), b"1");
    let entries = list(
        dir.path(),
        &ListOptions::new()
            .recursive(true)
            .error_policy(TraversalErrorPolicy::SkipErrors),
    )
    .unwrap();
    assert!(entries.iter().any(|e| e.path.ends_with("ok.txt")));
}

#[cfg(unix)]
#[test]
fn list_symlinks() {
    use path_rs::EntryKind;

    let dir = tempdir().unwrap();
    write(&dir.path().join("target.txt"), b"t");
    let link = dir.path().join("link.txt");
    std::os::unix::fs::symlink(dir.path().join("target.txt"), &link).unwrap();
    let entries = list(
        dir.path(),
        &ListOptions::new()
            .include_symlinks(true)
            .include_files(true),
    )
    .unwrap();
    assert!(entries.iter().any(|e| e.kind == EntryKind::Symlink));
}

#[test]
fn search_globs_excludes_and_predicates() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("a.rs"), b"xx");
    write(&dir.path().join("b.rs"), b"x");
    write(&dir.path().join("c.txt"), b"x");
    write(&dir.path().join("src/main.rs"), b"x");
    write(&dir.path().join("skip.rs"), b"x");

    let hits = search(&SearchRequest::new(dir.path(), ["**/*.rs"])).unwrap();
    assert_eq!(hits.len(), 4);

    let hits = search(&SearchRequest::new(dir.path(), ["*.rs"]).exclude(["skip.rs"])).unwrap();
    assert!(hits.iter().all(|e| !e.path.ends_with("skip.rs")));
    assert!(hits.iter().any(|e| e.path.ends_with("a.rs")));

    let pred = search_with(dir.path(), &ListOptions::default(), |e| {
        e.path.extension().is_some_and(|ext| ext == "rs") && e.size.is_some_and(|s| s >= 2)
    })
    .unwrap();
    assert_eq!(pred.len(), 1);
    assert!(pred[0].path.ends_with("a.rs"));
}

#[test]
fn search_invalid_pattern() {
    let dir = tempdir().unwrap();
    assert!(search(&SearchRequest::new(dir.path(), ["[unterminated"])).is_err());
}

#[test]
fn search_requires_pattern() {
    let dir = tempdir().unwrap();
    let req = SearchRequest {
        root: dir.path().to_path_buf(),
        patterns: vec![],
        exclude_patterns: vec![],
        options: ListOptions::default(),
        cache: CachePolicy::Bypass,
    };
    assert!(search(&req).is_err());
}

#[test]
fn cache_policies_matrix() {
    let dir = tempdir().unwrap();
    write(&dir.path().join("a.rs"), b"1");

    let cache = Arc::new(MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_secs(60)),
        max_entries: 32,
        validate_metadata: false,
    }));

    let mut req = SearchRequest::new(dir.path(), ["*.rs"]);
    req.cache = CachePolicy::ReadThrough;

    let first = search_with_cache(&req, Some(cache.as_ref())).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(cache.len().unwrap(), 1);

    write(&dir.path().join("b.rs"), b"1");
    // Warm hit is stale by design
    assert_eq!(
        search_with_cache(&req, Some(cache.as_ref())).unwrap().len(),
        1
    );

    let mut bypass = req.clone();
    bypass.cache = CachePolicy::Bypass;
    assert_eq!(
        search_with_cache(&bypass, Some(cache.as_ref()))
            .unwrap()
            .len(),
        2
    );

    let mut refresh = req.clone();
    refresh.cache = CachePolicy::Refresh;
    assert_eq!(
        search_with_cache(&refresh, Some(cache.as_ref()))
            .unwrap()
            .len(),
        2
    );

    use path_rs::DiscoveryCache;
    cache.invalidate(dir.path()).unwrap();
    assert_eq!(cache.len().unwrap(), 0);
    cache.clear().unwrap();
}

#[test]
fn search_max_entries() {
    let dir = tempdir().unwrap();
    for i in 0..10 {
        write(&dir.path().join(format!("f{i}.rs")), b"x");
    }
    let mut req = SearchRequest::new(dir.path(), ["*.rs"]);
    req.options.max_entries = Some(3);
    let hits = search(&req).unwrap();
    assert!(hits.len() <= 3);
}
