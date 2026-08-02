//! Cross-platform path expansion, normalization, resolution,
//! traversal, searching, and caching.
//!
//! `path-rs` complements [`std::path::Path`] and [`std::path::PathBuf`].
//! It does **not** replace them.
//!
//! # Distinguished operations
//!
//! | Operation | Filesystem access | Requires existence | Resolves symlinks |
//! | --- | ---: | ---: | ---: |
//! | [`expand_input`] | No\* | No | No |
//! | [`normalize`] | No | No | No |
//! | [`resolve_against`] | No | No | No |
//! | [`canonicalize_existing`] | Yes | Yes | Yes |
//! | [`path_identity_key`] | Configurable | No† | Configurable |
//! | [`app_paths`] | Platform APIs | No | No |
//! | [`inspect_directory`] | Yes | No | Partial |
//! | `list` / `discover_*` (feature `listing`) | Yes | Root yes | Configurable |
//! | `search` (feature `search`) | Yes | Root yes | Configurable |
//!
//! \* Home directory resolution may consult environment / platform APIs.  
//! † Unless symlink resolution is enabled on the identity options.
//!
//! # Architectural boundary
//!
//! This crate provides **generic path mechanics** only. Product-specific logic
//! (repository inventories, VCS roots, watchers, cloud remotes, mutation stores)
//! belongs in application adapters that *compose* these APIs.
//!
//! # Example
//!
//! ```
//! use path_rs::{expand_input, normalize, ExpandOptions};
//!
//! let expanded = expand_input("~", &ExpandOptions::default()).unwrap();
//! let normalized = normalize("foo/./bar/../baz").unwrap();
//! assert_eq!(normalized, std::path::PathBuf::from("foo/baz"));
//! let _ = expanded;
//! ```
//!
//! # Security
//!
//! Lexical normalization is not canonicalization. Lexical root containment is
//! not symlink-safe. Cached discovery results are not authoritative. Path
//! identity keys are comparison keys, not filesystem identity proofs. See
//! `SECURITY.md` in the repository.
//!
//! # Feature flags
//!
//! - `listing` (default): directory listing / discovery (`walkdir`)
//! - `search` (default): glob and predicate search (`globset`, implies `listing`)
//! - `persistent-cache`: on-disk discovery cache (`serde`, `serde_json`)
//! - `unicode`: Unicode NFC logical path keys
//! - `async`: `spawn_blocking` wrappers for list/search
//!
//! # Parallelism
//!
//! APIs do not spawn threads unless an explicit `async` helper is used.
//! Returned types and the in-memory cache are safe to use from parallel callers
//! (`Send` + `Sync` where applicable). The crate does not invoke external
//! processes, network clients, or VCS tools.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

mod containment;
mod dirs;
mod error;
mod expand;
mod identity;
mod inspect;
mod internal;
mod match_path;
mod metadata;
mod normalize;
mod platform;
mod resolve;
mod text;
mod utf8;

#[cfg(feature = "listing")]
#[cfg_attr(docsrs, doc(cfg(feature = "listing")))]
pub mod discovery;

#[cfg(feature = "listing")]
#[cfg_attr(docsrs, doc(cfg(feature = "listing")))]
pub mod listing;

#[cfg(feature = "search")]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
pub mod search;

pub mod cache;

pub use containment::{ensure_inside, is_lexically_inside};
pub use dirs::{
    AppPaths, AppPathsOptions, AppRootPolicy, PlatformDirs, app_paths, app_paths_with_options,
    app_paths_with_policy, cache_dir, config_dir, data_dir, platform_dirs, temp_dir,
};
pub use error::PathError;
pub use expand::{
    ExpandOptions, expand_dollar_variables, expand_input, expand_percent_variables, expand_tilde,
};
pub use identity::{
    PathIdentityOptions, PathRecord, deduplicate_paths, path_display_string, path_identity_key,
};
pub use inspect::{
    DirectoryInspection, MetadataSummary, directory_exists, inspect_directory,
    is_existing_directory, require_directory,
};
pub use match_path::{
    CommandLinePathMatchOptions, ExecutableMatchOptions, command_line_contains_path,
    executable_paths_match,
};
pub use metadata::{EntryKind, FileEntry, SortMode, TraversalErrorPolicy, sort_entries};
pub use normalize::{canonicalize_existing, normalize};
pub use platform::{
    is_device_namespace, is_drive_relative, is_reserved_windows_name, is_unc, is_verbatim,
    path_contains_reserved_name, simplify_for_display, translate_wsl_path,
};
pub use resolve::{absolute, join_relative, resolve_against, resolve_inside};
pub use text::{CaseNormalization, TextNormalizationOptions, normalize_path_token};
pub use utf8::{path_to_string_lossy, path_to_utf8};

#[cfg(feature = "unicode")]
#[cfg_attr(docsrs, doc(cfg(feature = "unicode")))]
pub use utf8::logical_path_key;

pub use cache::{
    CacheKey, CacheMode, CacheOptions, CachePolicy, CacheValue, DiscoveryCache, MemoryCache,
};

#[cfg(feature = "persistent-cache")]
#[cfg_attr(docsrs, doc(cfg(feature = "persistent-cache")))]
pub use cache::PersistentCache;

#[cfg(feature = "listing")]
#[cfg_attr(docsrs, doc(cfg(feature = "listing")))]
pub use discovery::{
    DirectoryVisitor, DiscoveryOptions, VisitControl, discover_directories, discover_where,
    visit_directories,
};

#[cfg(feature = "listing")]
#[cfg_attr(docsrs, doc(cfg(feature = "listing")))]
pub use listing::{ListOptions, WalkIter, list, walk};

#[cfg(feature = "search")]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
pub use search::{SearchRequest, search, search_with, search_with_cache};
