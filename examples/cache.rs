//! In-memory discovery cache: put/get/invalidate/clear and `search_with_cache`.

use path_rs::{
    CacheKey, CacheMode, CacheOptions, CachePolicy, CacheValue, DiscoveryCache, MemoryCache,
    SearchRequest, search_with_cache,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

fn main() -> Result<(), path_rs::PathError> {
    let cache = Arc::new(MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_secs(30)),
        max_entries: 64,
        validate_metadata: false,
    }));

    // --- search_with_cache (ReadThrough) ---
    let mut request = SearchRequest::new(".", ["**/*.rs"]);
    request.cache = CachePolicy::ReadThrough;

    let cold = search_with_cache(&request, Some(cache.as_ref()))?;
    let warm = search_with_cache(&request, Some(cache.as_ref()))?;
    println!("cold hits = {}, warm hits = {}", cold.len(), warm.len());
    println!("cache keys = {}", cache.len()?);
    println!("is_empty = {}", cache.is_empty()?);

    // --- DiscoveryCache trait methods ---
    let key = CacheKey::from_search(&request);
    if let Some(value) = cache.get(&key)? {
        println!(
            "get: entries={}, expired={}",
            value.entries.len(),
            value.is_expired()
        );
        println!(
            "is_expired_with_default(None) = {}",
            value.is_expired_with_default(None)
        );
    }

    // Manual put / invalidate / clear
    let manual_key = CacheKey::from_search(&SearchRequest::new("src", ["**/*"]));
    cache.put(
        manual_key.clone(),
        CacheValue {
            entries: Vec::new(),
            stored_at: SystemTime::now(),
            ttl: Some(Duration::from_secs(5)),
        },
    )?;
    println!("after put, len = {}", cache.len()?);

    cache.invalidate(std::path::Path::new("src"))?;
    println!("after invalidate(src), len = {}", cache.len()?);

    cache.clear()?;
    println!("after clear, is_empty = {}", cache.is_empty()?);

    // Disabled mode is a no-op store
    let disabled = MemoryCache::new(CacheOptions {
        mode: CacheMode::Disabled,
        ..CacheOptions::new()
    });
    let _ = disabled.put(
        key,
        CacheValue {
            entries: Vec::new(),
            stored_at: SystemTime::now(),
            ttl: None,
        },
    )?;
    println!("disabled cache len = {}", disabled.len()?);

    Ok(())
}
