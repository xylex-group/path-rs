use path_rs::{
    CacheKey, CacheMode, CacheOptions, CachePolicy, CacheValue, DiscoveryCache, MemoryCache,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

fn sample_key(root: &str, pattern: &str) -> CacheKey {
    CacheKey {
        root: PathBuf::from(root),
        patterns: vec![pattern.to_string()],
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

#[test]
fn disabled_cache_noop() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Disabled,
        ..CacheOptions::default()
    });
    let key = sample_key("/tmp", "*.rs");
    cache
        .put(
            key.clone(),
            CacheValue {
                entries: vec![],
                stored_at: SystemTime::now(),
                ttl: None,
            },
        )
        .unwrap();
    assert!(cache.get(&key).unwrap().is_none());
}

#[test]
fn put_get_invalidate_clear() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: None,
        max_entries: 8,
        validate_metadata: false,
    });
    let key = sample_key("/repo", "**/*.rs");
    cache
        .put(
            key.clone(),
            CacheValue {
                entries: vec![],
                stored_at: SystemTime::now(),
                ttl: None,
            },
        )
        .unwrap();
    assert!(cache.get(&key).unwrap().is_some());

    cache.invalidate(std::path::Path::new("/repo")).unwrap();
    assert!(cache.get(&key).unwrap().is_none());

    cache
        .put(
            key.clone(),
            CacheValue {
                entries: vec![],
                stored_at: SystemTime::now(),
                ttl: None,
            },
        )
        .unwrap();
    cache.clear().unwrap();
    assert!(cache.is_empty().unwrap());
}

#[test]
fn ttl_expiration() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_millis(1)),
        max_entries: 8,
        validate_metadata: false,
    });
    let key = sample_key("/repo", "*.txt");
    cache
        .put(
            key.clone(),
            CacheValue {
                entries: vec![],
                stored_at: SystemTime::now() - Duration::from_secs(5),
                ttl: Some(Duration::from_millis(1)),
            },
        )
        .unwrap();
    assert!(cache.get(&key).unwrap().is_none());
}

#[test]
fn different_options_different_keys() {
    let a = sample_key("/repo", "*.rs");
    let mut b = sample_key("/repo", "*.rs");
    b.include_hidden = true;
    assert_ne!(a, b);
}

#[test]
fn max_entries_eviction() {
    let cache = MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: None,
        max_entries: 2,
        validate_metadata: false,
    });
    for i in 0..3 {
        let key = sample_key("/repo", &format!("p{i}"));
        cache
            .put(
                key,
                CacheValue {
                    entries: vec![],
                    stored_at: SystemTime::now(),
                    ttl: None,
                },
            )
            .unwrap();
    }
    assert!(cache.len().unwrap() <= 2);
}

#[test]
fn cache_policy_enum_defaults() {
    assert_eq!(CachePolicy::default(), CachePolicy::Bypass);
    assert_eq!(CacheMode::default(), CacheMode::Disabled);
}
