# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-08-02

### Added

- Path input expansion: `~`, `%VAR%`, `$VAR` / `${VAR}`, optional WSL `/mnt/<drive>/...`
- Platform directory helpers (`platform_dirs`, `config_dir`, `data_dir`, `cache_dir`)
- Application roots (`AppPaths`, `app_paths`, `AppRootPolicy`, env overrides) without product-specific subdirs
- Lexical normalization (`normalize`) and filesystem canonicalization (`canonicalize_existing`)
- Path identity keys, deduplication, and `PathRecord` display/identity split
- Text/token normalization (`normalize_path_token`) without VCS semantics
- Directory inspection (`inspect_directory`, `is_existing_directory`, `require_directory`)
- Generic directory discovery (`discover_directories`, `discover_where`, visitors)
- Executable and command-line path matching helpers (no process inspection)
- Resolution helpers: `absolute`, `resolve_against`, `join_relative`, `resolve_inside`
- Lexical root containment checks
- Windows path classification (drive-relative, UNC, verbatim, device namespace, reserved names)
- Directory listing and recursive traversal (`listing` feature)
- Glob and predicate search (`search` feature)
- Opt-in in-memory discovery cache; optional persistent cache (`persistent-cache`)
- UTF-8 helpers; optional Unicode logical keys (`unicode`)
- Optional async wrappers (`async`)
- Examples, Criterion benchmarks, cross-platform CI, dual MIT/Apache-2.0 licensing

### Fixed

- Expansion depth accounting no longer treats a single successful pass as a depth-exceeded failure
- Path identity key generation no longer infinite-loops on UNC-style `//` prefixes
- `walk` is a real streaming iterator (no longer materializes then wraps)
- Search no longer uses a process-global default cache; caching requires an explicit `DiscoveryCache`
- Unified listing/discovery depth model (walkdir depth 0 = root)
- Removed dead private `StripVerbatimHelper` trait
- `CacheKey.sort` stores `SortMode` instead of an opaque `u8`
- Complete fluent builders on `ListOptions` / `DiscoveryOptions`; `new()` on option structs
- `directory_exists` is the primary name (`is_existing_directory` remains an alias)
- `DirectoryVisitor` blanket impl for `FnMut` closures

### Security

- Documented limitations of lexical containment, caching, identity keys, and environment expansion
- Defensive limits for expansion depth, glob pattern length, traversal depth/entries, and cache size
- Architectural boundary: generic path mechanics only (no VCS/product business logic)
