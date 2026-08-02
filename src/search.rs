//! Glob and predicate-based path search.
//!
//! Enabled by the `search` feature (default). Depends on `listing`.

use crate::cache::{CacheKey, CachePolicy, CacheValue, DiscoveryCache};
use crate::error::PathError;
use crate::internal::validation::{MAX_PATTERN_LENGTH, reject_nul_path};
use crate::listing::{ListOptions, list};
use crate::metadata::{EntryKind, FileEntry};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Request for a glob-based search.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// Root directory to search.
    pub root: PathBuf,
    /// Include glob patterns (matched against paths relative to `root`).
    pub patterns: Vec<String>,
    /// Optional exclude patterns.
    pub exclude_patterns: Vec<String>,
    /// Listing / traversal options.
    pub options: ListOptions,
    /// Cache policy for this request.
    ///
    /// Caching only occurs when an explicit [`DiscoveryCache`] is passed to
    /// [`search_with_cache`]. [`search`] never reads or writes a cache.
    pub cache: CachePolicy,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            options: ListOptions {
                recursive: true,
                ..ListOptions::default()
            },
            cache: CachePolicy::Bypass,
        }
    }
}

impl SearchRequest {
    /// Create a search request with include patterns and default options.
    pub fn new(
        root: impl Into<PathBuf>,
        patterns: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            root: root.into(),
            patterns: patterns.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Add exclude patterns.
    pub fn exclude(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.exclude_patterns
            .extend(patterns.into_iter().map(Into::into));
        self
    }

    /// Set list options.
    pub fn options(mut self, options: ListOptions) -> Self {
        self.options = options;
        self
    }

    /// Set cache policy (only effective with an explicit cache in [`search_with_cache`]).
    pub fn cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache = policy;
        self
    }
}

/// Search for entries under `request.root` matching include patterns.
///
/// Patterns are matched against the path relative to the search root using `globset`.
///
/// This function **never** uses a cache (equivalent to [`CachePolicy::Bypass`]).
/// For caching, call [`search_with_cache`] with an explicit [`DiscoveryCache`].
///
/// # Filesystem access
///
/// **Yes.** Requires the root to exist.
pub fn search(request: &SearchRequest) -> Result<Vec<FileEntry>, PathError> {
    search_uncached(request)
}

/// Search using an explicit discovery cache.
///
/// # Cache rules
///
/// | Policy | Without `cache` | With `cache` |
/// | --- | --- | --- |
/// | [`CachePolicy::Bypass`] | Scan only | Scan only (no read/write) |
/// | [`CachePolicy::ReadThrough`] | Scan only | Read then maybe write |
/// | [`CachePolicy::Refresh`] | Scan only | Scan and write |
///
/// There is **no** process-global default cache. Callers that want caching must
/// supply their own [`MemoryCache`] or [`crate::cache::PersistentCache`].
///
/// # Filesystem access
///
/// **Yes.** Requires the root to exist.
///
/// # Security
///
/// Cache hits are performance artifacts only. Re-validate paths before security-sensitive use.
pub fn search_with_cache(
    request: &SearchRequest,
    cache: Option<&dyn DiscoveryCache>,
) -> Result<Vec<FileEntry>, PathError> {
    reject_nul_path(&request.root)?;
    validate_patterns(request)?;

    let key = CacheKey::from_search(request);

    // Without an explicit cache, never use global state — always scan.
    let Some(cache) = cache else {
        return search_uncached(request);
    };

    match request.cache {
        CachePolicy::Bypass => search_uncached(request),
        CachePolicy::ReadThrough => {
            if let Some(value) = cache.get(&key)? {
                if !value.is_expired() {
                    return Ok(value.entries);
                }
            }
            let entries = search_uncached(request)?;
            cache.put(
                key,
                CacheValue {
                    entries: entries.clone(),
                    stored_at: SystemTime::now(),
                    ttl: None,
                },
            )?;
            Ok(entries)
        }
        CachePolicy::Refresh => {
            let entries = search_uncached(request)?;
            cache.put(
                key,
                CacheValue {
                    entries: entries.clone(),
                    stored_at: SystemTime::now(),
                    ttl: None,
                },
            )?;
            Ok(entries)
        }
    }
}

fn validate_patterns(request: &SearchRequest) -> Result<(), PathError> {
    if request.patterns.is_empty() {
        return Err(PathError::invalid("search requires at least one pattern"));
    }
    for p in request
        .patterns
        .iter()
        .chain(request.exclude_patterns.iter())
    {
        if p.len() > MAX_PATTERN_LENGTH {
            return Err(PathError::InvalidGlob {
                message: format!("pattern exceeds maximum length ({MAX_PATTERN_LENGTH})"),
            });
        }
    }
    Ok(())
}

fn search_uncached(request: &SearchRequest) -> Result<Vec<FileEntry>, PathError> {
    reject_nul_path(&request.root)?;
    validate_patterns(request)?;

    let include = build_globset(&request.patterns)?;
    let exclude = if request.exclude_patterns.is_empty() {
        None
    } else {
        Some(build_globset(&request.exclude_patterns)?)
    };

    let mut list_opts = request.options.clone();
    if !list_opts.include_files && !list_opts.include_directories && !list_opts.include_symlinks {
        list_opts.include_files = true;
    }

    let listed = list(&request.root, &list_opts)?;
    let mut out = Vec::new();

    for entry in listed {
        let rel = entry
            .relative_path
            .as_ref()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let matched = include.is_match(&rel) || include.is_match(&name);
        if !matched {
            continue;
        }
        if let Some(ex) = &exclude {
            if ex.is_match(&rel) || ex.is_match(&name) {
                continue;
            }
        }
        out.push(entry);
        if let Some(max) = request.options.max_entries {
            if out.len() >= max {
                break;
            }
        }
    }

    Ok(out)
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, PathError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| PathError::InvalidGlob {
            message: e.to_string(),
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|e| PathError::InvalidGlob {
        message: e.to_string(),
    })
}

/// Search with a predicate closure.
///
/// # Filesystem access
///
/// **Yes.**
pub fn search_with(
    root: impl AsRef<Path>,
    options: &ListOptions,
    predicate: impl Fn(&FileEntry) -> bool,
) -> Result<Vec<FileEntry>, PathError> {
    let listed = list(root, options)?;
    let mut out: Vec<FileEntry> = listed.into_iter().filter(|e| predicate(e)).collect();
    if let Some(max) = options.max_entries {
        out.truncate(max);
    }
    Ok(out)
}

/// Built-in entry predicates for common filters.
pub mod predicates {
    use super::*;
    use std::ffi::OsStr;
    use std::time::SystemTime;

    /// Match entries with the given extension (without the dot), case-sensitive.
    pub fn extension(ext: impl AsRef<OsStr>) -> impl Fn(&FileEntry) -> bool {
        let ext = ext.as_ref().to_os_string();
        move |entry: &FileEntry| entry.path.extension() == Some(ext.as_os_str())
    }

    /// Match entries whose file name equals `name`.
    pub fn filename(name: impl AsRef<OsStr>) -> impl Fn(&FileEntry) -> bool {
        let name = name.as_ref().to_os_string();
        move |entry: &FileEntry| entry.path.file_name() == Some(name.as_os_str())
    }

    /// Match a specific entry kind.
    pub fn kind(expected: EntryKind) -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| entry.kind == expected
    }

    /// Match files with size >= `min`.
    pub fn min_size(min: u64) -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| entry.size.is_some_and(|s| s >= min)
    }

    /// Match files with size <= `max`.
    pub fn max_size(max: u64) -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| entry.size.is_some_and(|s| s <= max)
    }

    /// Match entries modified after `t`.
    pub fn modified_after(t: SystemTime) -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| entry.modified.is_some_and(|m| m > t)
    }

    /// Match entries modified before `t`.
    pub fn modified_before(t: SystemTime) -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| entry.modified.is_some_and(|m| m < t)
    }

    /// Match hidden names (leading `.`).
    pub fn is_hidden() -> impl Fn(&FileEntry) -> bool {
        move |entry: &FileEntry| {
            entry
                .path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        }
    }
}

#[cfg(feature = "async")]
/// Async wrapper around [`search`].
pub async fn search_async(request: SearchRequest) -> Result<Vec<FileEntry>, PathError> {
    tokio::task::spawn_blocking(move || search(&request))
        .await
        .map_err(|e| PathError::traversal(format!("async search join failed: {e}")))?
}
