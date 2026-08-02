use path_rs::{
    CacheMode, CacheOptions, CachePolicy, ListOptions, MemoryCache, SearchRequest, search,
    search_with, search_with_cache,
};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn glob_search_rs_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), b"x").unwrap();
    fs::write(dir.path().join("b.txt"), b"x").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), b"x").unwrap();

    let request = SearchRequest::new(dir.path(), ["**/*.rs"]);
    let hits = search(&request).unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn exclude_patterns() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("keep.rs"), b"x").unwrap();
    fs::write(dir.path().join("skip.rs"), b"x").unwrap();

    let request = SearchRequest::new(dir.path(), ["*.rs"]).exclude(["skip.rs"]);
    let hits = search(&request).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].path.ends_with("keep.rs"));
}

#[test]
fn predicate_search() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), b"xx").unwrap();
    fs::write(dir.path().join("b.rs"), b"x").unwrap();

    let opts = ListOptions::default();
    let hits = search_with(dir.path(), &opts, |e| {
        e.path.extension().is_some_and(|ext| ext == "rs") && e.size.is_some_and(|s| s >= 2)
    })
    .unwrap();
    assert_eq!(hits.len(), 1);
}

#[test]
fn cache_read_through_and_bypass() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), b"x").unwrap();

    let cache = Arc::new(MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_secs(60)),
        max_entries: 16,
        validate_metadata: false,
    }));

    let mut request = SearchRequest::new(dir.path(), ["*.rs"]);
    request.cache = CachePolicy::ReadThrough;

    let first = search_with_cache(&request, Some(cache.as_ref())).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(cache.len().unwrap(), 1);

    fs::write(dir.path().join("b.rs"), b"x").unwrap();
    // Warm cache still returns old result.
    let warm = search_with_cache(&request, Some(cache.as_ref())).unwrap();
    assert_eq!(warm.len(), 1);

    // Bypass does not read or write (custom cache still passed but policy bypass).
    let mut bypass = request.clone();
    bypass.cache = CachePolicy::Bypass;
    let all = search_with_cache(&bypass, Some(cache.as_ref())).unwrap();
    assert_eq!(all.len(), 2);

    // Refresh updates cache.
    let mut refresh = request.clone();
    refresh.cache = CachePolicy::Refresh;
    let refreshed = search_with_cache(&refresh, Some(cache.as_ref())).unwrap();
    assert_eq!(refreshed.len(), 2);
}

#[test]
fn no_global_cache_without_explicit_store() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), b"x").unwrap();

    let mut request = SearchRequest::new(dir.path(), ["*.rs"]);
    request.cache = CachePolicy::ReadThrough;

    // ReadThrough without a cache instance must scan, not use hidden process state.
    let a = search_with_cache(&request, None).unwrap();
    fs::write(dir.path().join("b.rs"), b"x").unwrap();
    let b = search_with_cache(&request, None).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 2);

    // search() never caches either.
    assert_eq!(search(&request).unwrap().len(), 2);
}

#[test]
fn invalid_glob() {
    let dir = tempdir().unwrap();
    let request = SearchRequest::new(dir.path(), ["[unterminated"]);
    assert!(search(&request).is_err());
}
