//! Memory cache edge cases and optional persistent cache.

use path_rs::{CacheKey, CacheMode, CacheOptions, CacheValue, DiscoveryCache, MemoryCache};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

fn key(pattern: &str) -> CacheKey {
    CacheKey {
        root: PathBuf::from("/repo"),
        patterns: vec![pattern.into()],
        exclude_patterns: vec![],
        recursive: true,
        follow_symlinks: false,
        include_hidden: false,
        include_files: true,
        include_directories: true,
        include_symlinks: true,
        max_depth: None,
        sort: path_rs::SortMode::Path,
    }
}

fn empty_value() -> CacheValue {
    CacheValue {
        entries: vec![],
        stored_at: SystemTime::now(),
        ttl: None,
    }
}

#[test]
fn different_keys_for_different_options() {
    let mut a = key("*.rs");
    let mut b = key("*.rs");
    b.include_hidden = true;
    assert_ne!(a, b);
    a.patterns = vec!["**/*.rs".into()];
    assert_ne!(a, b);
}

#[test]
fn ttl_and_eviction() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_millis(1)),
        max_entries: 2,
        validate_metadata: false,
    });

    let k = key("a");
    cache
        .put(
            k.clone(),
            CacheValue {
                entries: vec![],
                stored_at: SystemTime::now() - Duration::from_secs(10),
                ttl: Some(Duration::from_millis(1)),
            },
        )
        .unwrap();
    assert!(cache.get(&k).unwrap().is_none());

    cache.put(key("1"), empty_value()).unwrap();
    cache.put(key("2"), empty_value()).unwrap();
    cache.put(key("3"), empty_value()).unwrap();
    assert!(cache.len().unwrap() <= 2);
}

#[test]
fn invalidate_root_prefix() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: None,
        max_entries: 16,
        validate_metadata: false,
    });
    let mut k1 = key("*.rs");
    k1.root = PathBuf::from("/repo/a");
    let mut k2 = key("*.rs");
    k2.root = PathBuf::from("/other");
    cache.put(k1.clone(), empty_value()).unwrap();
    cache.put(k2.clone(), empty_value()).unwrap();
    cache.invalidate(std::path::Path::new("/repo")).unwrap();
    assert!(cache.get(&k1).unwrap().is_none());
    assert!(cache.get(&k2).unwrap().is_some());
}

#[cfg(feature = "persistent-cache")]
#[test]
fn persistent_cache_roundtrip() {
    use path_rs::PersistentCache;
    use std::time::Duration;

    let name = format!(
        "path-rs-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let file = format!("{name}.json");
    let cache = PersistentCache::open(
        &name,
        &file,
        CacheOptions {
            mode: CacheMode::Persistent,
            ttl: Some(Duration::from_secs(3600)),
            max_entries: 32,
            validate_metadata: false,
        },
    )
    .unwrap();

    let k = key("**/*.rs");
    cache.put(k.clone(), empty_value()).unwrap();
    assert!(cache.get(&k).unwrap().is_some());
    cache.clear().unwrap();
    assert!(cache.get(&k).unwrap().is_none());
}

#[cfg(feature = "persistent-cache")]
#[test]
fn corrupt_persistent_cache_is_miss() {
    use path_rs::PersistentCache;

    let name = format!(
        "path-rs-corrupt-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let cache = PersistentCache::open(
        &name,
        "corrupt.json",
        CacheOptions {
            mode: CacheMode::Persistent,
            ttl: None,
            max_entries: 8,
            validate_metadata: false,
        },
    )
    .unwrap();
    // Write garbage into the cache file path
    let path = cache.path().to_path_buf();
    std::fs::write(&path, b"not-json{{{").unwrap();

    let cache2 = PersistentCache::open(
        &name,
        "corrupt.json",
        CacheOptions {
            mode: CacheMode::Persistent,
            ttl: None,
            max_entries: 8,
            validate_metadata: false,
        },
    )
    .unwrap();
    assert!(cache2.get(&key("x")).unwrap().is_none());
}
