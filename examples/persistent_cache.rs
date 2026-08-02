//! On-disk discovery cache (feature `persistent-cache`).

use path_rs::{
    CacheMode, CacheOptions, CachePolicy, DiscoveryCache, PersistentCache, SearchRequest,
    search_with_cache,
};
use std::time::Duration;

fn main() -> Result<(), path_rs::PathError> {
    let cache = PersistentCache::open(
        "path-rs-demo",
        "search-cache.json",
        CacheOptions {
            mode: CacheMode::Persistent,
            ttl: Some(Duration::from_secs(60)),
            max_entries: 128,
            validate_metadata: false,
        },
    )?;

    println!("persistent cache path = {}", cache.path().display());

    let request = SearchRequest::new(".", ["**/*.rs"]).cache_policy(CachePolicy::ReadThrough);
    let cold = search_with_cache(&request, Some(&cache))?;
    let warm = search_with_cache(&request, Some(&cache))?;
    println!("cold hits = {}, warm hits = {}", cold.len(), warm.len());

    cache.clear()?;
    println!("cleared persistent cache");

    Ok(())
}
