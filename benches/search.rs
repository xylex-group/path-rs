use criterion::{Criterion, black_box, criterion_group, criterion_main};
use path_rs::{
    CacheMode, CacheOptions, CachePolicy, DiscoveryCache, MemoryCache, SearchRequest,
    search_with_cache,
};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn bench_search(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    for i in 0..200 {
        fs::write(dir.path().join(format!("f{i}.rs")), b"fn main(){}").unwrap();
        fs::write(dir.path().join(format!("n{i}.txt")), b"x").unwrap();
    }

    let request = SearchRequest::new(dir.path(), ["**/*.rs"]);
    c.bench_function("glob_search", |b| {
        b.iter(|| path_rs::search(black_box(&request)).unwrap())
    });

    let cache = Arc::new(MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_secs(60)),
        max_entries: 32,
        validate_metadata: false,
    }));

    let mut cached = SearchRequest::new(dir.path(), ["**/*.rs"]);
    cached.cache = CachePolicy::ReadThrough;

    c.bench_function("cache_cold", |b| {
        b.iter(|| {
            cache.clear().unwrap();
            search_with_cache(black_box(&cached), Some(cache.as_ref())).unwrap()
        })
    });

    // Warm once.
    let _ = search_with_cache(&cached, Some(cache.as_ref())).unwrap();
    c.bench_function("cache_warm", |b| {
        b.iter(|| search_with_cache(black_box(&cached), Some(cache.as_ref())).unwrap())
    });

    c.bench_function("cache_refresh", |b| {
        let mut refresh = cached.clone();
        refresh.cache = CachePolicy::Refresh;
        b.iter(|| search_with_cache(black_box(&refresh), Some(cache.as_ref())).unwrap())
    });

    c.bench_function("cache_invalidate", |b| {
        b.iter(|| {
            cache.invalidate(black_box(dir.path())).unwrap();
            search_with_cache(black_box(&cached), Some(cache.as_ref())).unwrap()
        })
    });
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
