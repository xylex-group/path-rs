//! In-memory discovery cache demo.

use path_rs::{
    CacheMode, CacheOptions, CachePolicy, MemoryCache, SearchRequest, search_with_cache,
};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), path_rs::PathError> {
    let cache = Arc::new(MemoryCache::new(CacheOptions {
        mode: CacheMode::Memory,
        ttl: Some(Duration::from_secs(30)),
        max_entries: 64,
        validate_metadata: false,
    }));

    let mut request = SearchRequest::new(".", ["**/*.rs"]);
    request.cache = CachePolicy::ReadThrough;

    let cold = search_with_cache(&request, Some(cache.as_ref()))?;
    let warm = search_with_cache(&request, Some(cache.as_ref()))?;
    println!("cold hits = {}, warm hits = {}", cold.len(), warm.len());
    println!("cache keys = {}", cache.len()?);
    Ok(())
}
