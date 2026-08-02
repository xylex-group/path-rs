//! Optional discovery caching.
//!
//! Caching is **opt-in** and must never be used as a security boundary.
//! Stale entries are performance artifacts, not authoritative filesystem state.

use crate::error::PathError;
use crate::metadata::FileEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[cfg(feature = "search")]
use crate::search::SearchRequest;

/// Whether / how a search should consult the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CachePolicy {
    /// Do not read or write the cache.
    #[default]
    Bypass,
    /// Return a fresh cached value when present; otherwise scan and store.
    ReadThrough,
    /// Ignore any existing value, scan, and store the new result.
    Refresh,
}

/// Cache storage mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CacheMode {
    /// Caching disabled.
    #[default]
    Disabled,
    /// In-memory process-local cache.
    Memory,
    /// Persistent on-disk cache (requires `persistent-cache` feature).
    Persistent,
}

/// Configuration for a discovery cache instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheOptions {
    /// Storage mode.
    pub mode: CacheMode,
    /// Optional time-to-live for entries.
    pub ttl: Option<Duration>,
    /// Maximum number of cached keys.
    pub max_entries: usize,
    /// Reserved for future metadata revalidation (directory mtime, etc.).
    ///
    /// Directory timestamps are **not** a perfect invalidation mechanism and are
    /// not used as a security signal even when enabled in future versions.
    pub validate_metadata: bool,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            mode: CacheMode::Disabled,
            ttl: None,
            max_entries: 128,
            validate_metadata: false,
        }
    }
}

impl CacheOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Cache key capturing all inputs that affect discovery results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Search / list root.
    pub root: PathBuf,
    /// Glob patterns (empty for pure listing cache use).
    pub patterns: Vec<String>,
    /// Exclude patterns.
    pub exclude_patterns: Vec<String>,
    /// Recursive walk.
    pub recursive: bool,
    /// Follow symlinks.
    pub follow_symlinks: bool,
    /// Include hidden.
    pub include_hidden: bool,
    /// Include files.
    pub include_files: bool,
    /// Include directories.
    pub include_directories: bool,
    /// Include symlinks.
    pub include_symlinks: bool,
    /// Max depth.
    pub max_depth: Option<usize>,
    /// Sort mode used for the discovery result.
    pub sort: crate::metadata::SortMode,
}

impl CacheKey {
    /// Build a key from a search request.
    #[cfg(feature = "search")]
    pub fn from_search(request: &SearchRequest) -> Self {
        Self {
            root: request.root.clone(),
            patterns: request.patterns.clone(),
            exclude_patterns: request.exclude_patterns.clone(),
            recursive: request.options.recursive,
            follow_symlinks: request.options.follow_symlinks,
            include_hidden: request.options.include_hidden,
            include_files: request.options.include_files,
            include_directories: request.options.include_directories,
            include_symlinks: request.options.include_symlinks,
            max_depth: request.options.max_depth,
            sort: request.options.sort,
        }
    }
}

/// Cached discovery result.
#[derive(Debug, Clone)]
pub struct CacheValue {
    /// Cached entries.
    pub entries: Vec<FileEntry>,
    /// When the value was stored.
    pub stored_at: SystemTime,
    /// Optional TTL override for this value.
    pub ttl: Option<Duration>,
}

impl CacheValue {
    /// Returns true if the entry is past its TTL.
    pub fn is_expired(&self) -> bool {
        let Some(ttl) = self.ttl else {
            return false;
        };
        self.stored_at.elapsed().map(|e| e > ttl).unwrap_or(true)
    }

    /// Returns true if expired given a default TTL when `self.ttl` is `None`.
    pub fn is_expired_with_default(&self, default_ttl: Option<Duration>) -> bool {
        let ttl = self.ttl.or(default_ttl);
        let Some(ttl) = ttl else {
            return false;
        };
        self.stored_at.elapsed().map(|e| e > ttl).unwrap_or(true)
    }
}

/// Trait for discovery result caches.
pub trait DiscoveryCache: Send + Sync {
    /// Get a cached value by key.
    fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>, PathError>;
    /// Store a value.
    fn put(&self, key: CacheKey, value: CacheValue) -> Result<(), PathError>;
    /// Invalidate all keys whose root equals or is under `root`.
    fn invalidate(&self, root: &Path) -> Result<(), PathError>;
    /// Clear the entire cache.
    fn clear(&self) -> Result<(), PathError>;
}

/// Thread-safe in-memory discovery cache.
#[derive(Debug)]
pub struct MemoryCache {
    options: CacheOptions,
    inner: Mutex<HashMap<CacheKey, CacheValue>>,
}

impl MemoryCache {
    /// Create a new memory cache with the given options.
    ///
    /// If `mode` is [`CacheMode::Disabled`], get always returns `None` and put is a no-op.
    pub fn new(options: CacheOptions) -> Self {
        Self {
            options,
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Number of entries currently stored (for tests).
    pub fn len(&self) -> Result<usize, PathError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;
        Ok(guard.len())
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> Result<bool, PathError> {
        Ok(self.len()? == 0)
    }

    #[cfg(feature = "persistent-cache")]
    pub(crate) fn snapshot(&self) -> Result<Vec<(CacheKey, CacheValue)>, PathError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;
        Ok(guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
}

impl DiscoveryCache for MemoryCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>, PathError> {
        if self.options.mode == CacheMode::Disabled {
            return Ok(None);
        }
        let mut guard: std::sync::MutexGuard<'_, HashMap<CacheKey, CacheValue>> = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;

        if let Some(value) = guard.get(key) {
            if value.is_expired_with_default(self.options.ttl) {
                guard.remove(key);
                return Ok(None);
            }
            return Ok(Some(value.clone()));
        }
        Ok(None)
    }

    fn put(&self, key: CacheKey, mut value: CacheValue) -> Result<(), PathError> {
        if self.options.mode == CacheMode::Disabled {
            return Ok(());
        }
        if value.ttl.is_none() {
            value.ttl = self.options.ttl;
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;

        if guard.len() >= self.options.max_entries && !guard.contains_key(&key) {
            // Evict an arbitrary entry to enforce the bound (not LRU).
            if let Some(evict) = guard.keys().next().cloned() {
                guard.remove(&evict);
            }
        }
        guard.insert(key, value);
        Ok(())
    }

    fn invalidate(&self, root: &Path) -> Result<(), PathError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;
        guard.retain(|k, _| k.root != root && !k.root.starts_with(root));
        Ok(())
    }

    fn clear(&self) -> Result<(), PathError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PathError::cache("memory cache lock poisoned"))?;
        guard.clear();
        Ok(())
    }
}

/// Persistent on-disk cache (feature `persistent-cache`).
///
/// Schema is versioned. Corrupt or mismatched files are treated as a miss
/// (and optionally replaced on next put). Atomic writes use a temp file + rename.
///
/// Store persistent cache under the platform cache directory, not beside source files.
#[cfg(feature = "persistent-cache")]
#[derive(Debug)]
pub struct PersistentCache {
    path: PathBuf,
    options: CacheOptions,
    memory: MemoryCache,
}

#[cfg(feature = "persistent-cache")]
mod persistent_schema {
    use serde::{Deserialize, Serialize};

    pub const SCHEMA_VERSION: u32 = 1;

    #[derive(Serialize, Deserialize)]
    pub struct PersistentCacheFile {
        pub schema_version: u32,
        pub entries: Vec<PersistentCacheRecord>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PersistentCacheRecord {
        pub key: PersistentKey,
        pub paths: Vec<String>,
        pub stored_at_epoch_ms: u128,
        pub ttl_ms: Option<u128>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct PersistentKey {
        pub root: String,
        pub patterns: Vec<String>,
        pub exclude_patterns: Vec<String>,
        pub recursive: bool,
        pub follow_symlinks: bool,
        pub include_hidden: bool,
        pub include_files: bool,
        pub include_directories: bool,
        pub include_symlinks: bool,
        pub max_depth: Option<usize>,
        pub sort: u8,
    }
}

#[cfg(feature = "persistent-cache")]
fn sort_to_u8(sort: crate::metadata::SortMode) -> u8 {
    use crate::metadata::SortMode;
    match sort {
        SortMode::None => 0,
        SortMode::Path => 1,
        SortMode::Name => 2,
        SortMode::DirsFirst => 3,
    }
}

#[cfg(feature = "persistent-cache")]
fn sort_from_u8(v: u8) -> crate::metadata::SortMode {
    use crate::metadata::SortMode;
    match v {
        0 => SortMode::None,
        2 => SortMode::Name,
        3 => SortMode::DirsFirst,
        _ => SortMode::Path,
    }
}

#[cfg(feature = "persistent-cache")]
impl PersistentCache {
    /// Open or create a persistent cache file under the platform cache directory.
    pub fn open(app_name: &str, file_name: &str, options: CacheOptions) -> Result<Self, PathError> {
        use std::fs;
        let dir = crate::dirs::cache_dir(app_name)?;
        fs::create_dir_all(&dir).map_err(|e| PathError::filesystem(&dir, e))?;
        let path = dir.join(file_name);
        let memory = MemoryCache::new(CacheOptions {
            mode: CacheMode::Memory,
            ttl: options.ttl,
            max_entries: options.max_entries,
            validate_metadata: options.validate_metadata,
        });
        let cache = Self {
            path,
            options,
            memory,
        };
        let _ = cache.load_into_memory();
        Ok(cache)
    }

    /// Path to the on-disk cache file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load_into_memory(&self) -> Result<(), PathError> {
        use persistent_schema::{PersistentCacheFile, SCHEMA_VERSION};
        use std::fs;

        if !self.path.exists() {
            return Ok(());
        }
        let data = fs::read(&self.path).map_err(|e| PathError::filesystem(&self.path, e))?;
        let parsed: PersistentCacheFile = match serde_json::from_slice(&data) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        if parsed.schema_version != SCHEMA_VERSION {
            return Ok(());
        }
        for rec in parsed.entries {
            let key = CacheKey {
                root: PathBuf::from(&rec.key.root),
                patterns: rec.key.patterns,
                exclude_patterns: rec.key.exclude_patterns,
                recursive: rec.key.recursive,
                follow_symlinks: rec.key.follow_symlinks,
                include_hidden: rec.key.include_hidden,
                include_files: rec.key.include_files,
                include_directories: rec.key.include_directories,
                include_symlinks: rec.key.include_symlinks,
                max_depth: rec.key.max_depth,
                sort: sort_from_u8(rec.key.sort),
            };
            let stored_at =
                SystemTime::UNIX_EPOCH + Duration::from_millis(rec.stored_at_epoch_ms as u64);
            let ttl = rec.ttl_ms.map(|ms| Duration::from_millis(ms as u64));
            let entries = rec
                .paths
                .into_iter()
                .map(|p| FileEntry::new(PathBuf::from(p), crate::metadata::EntryKind::Other))
                .collect();
            let value = CacheValue {
                entries,
                stored_at,
                ttl,
            };
            if !value.is_expired_with_default(self.options.ttl) {
                self.memory.put(key, value)?;
            }
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), PathError> {
        use persistent_schema::{
            PersistentCacheFile, PersistentCacheRecord, PersistentKey, SCHEMA_VERSION,
        };
        use std::fs;
        use std::io::Write;

        let snapshot = self.memory.snapshot()?;
        let mut records = Vec::new();
        for (key, value) in snapshot {
            if value.is_expired_with_default(self.options.ttl) {
                continue;
            }
            let stored_at_epoch_ms = value
                .stored_at
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            records.push(PersistentCacheRecord {
                key: PersistentKey {
                    root: key.root.to_string_lossy().into_owned(),
                    patterns: key.patterns.clone(),
                    exclude_patterns: key.exclude_patterns.clone(),
                    recursive: key.recursive,
                    follow_symlinks: key.follow_symlinks,
                    include_hidden: key.include_hidden,
                    include_files: key.include_files,
                    include_directories: key.include_directories,
                    include_symlinks: key.include_symlinks,
                    max_depth: key.max_depth,
                    sort: sort_to_u8(key.sort),
                },
                paths: value
                    .entries
                    .iter()
                    .map(|e| e.path.to_string_lossy().into_owned())
                    .collect(),
                stored_at_epoch_ms,
                ttl_ms: value.ttl.map(|d| d.as_millis()),
            });
        }

        let file = PersistentCacheFile {
            schema_version: SCHEMA_VERSION,
            entries: records,
        };
        let json = serde_json::to_vec_pretty(&file)
            .map_err(|e| PathError::cache(format!("serialize failed: {e}")))?;

        let tmp_path = self.path.with_extension("json.tmp");
        {
            let mut f =
                fs::File::create(&tmp_path).map_err(|e| PathError::filesystem(&tmp_path, e))?;
            f.write_all(&json)
                .map_err(|e| PathError::filesystem(&tmp_path, e))?;
            f.sync_all()
                .map_err(|e| PathError::filesystem(&tmp_path, e))?;
        }
        fs::rename(&tmp_path, &self.path).map_err(|e| PathError::filesystem(&self.path, e))?;
        Ok(())
    }
}

#[cfg(feature = "persistent-cache")]
impl DiscoveryCache for PersistentCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>, PathError> {
        if matches!(self.options.mode, CacheMode::Disabled) {
            return Ok(None);
        }
        self.memory.get(key)
    }

    fn put(&self, key: CacheKey, value: CacheValue) -> Result<(), PathError> {
        if matches!(self.options.mode, CacheMode::Disabled) {
            return Ok(());
        }
        self.memory.put(key, value)?;
        self.flush()
    }

    fn invalidate(&self, root: &Path) -> Result<(), PathError> {
        self.memory.invalidate(root)?;
        self.flush()
    }

    fn clear(&self) -> Result<(), PathError> {
        use std::fs;
        self.memory.clear()?;
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|e| PathError::filesystem(&self.path, e))?;
        }
        Ok(())
    }
}
