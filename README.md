# path-rs

Cross-platform path expansion, normalization, resolution, traversal, searching, and caching for Rust.

`path-rs` **complements** `std::path::{Path, PathBuf}`. It does **not** replace them.

```rust
use std::path::{Path, PathBuf};
use path_rs::{expand_input, normalize, ExpandOptions};

let expanded = expand_input("~/projects", &ExpandOptions::default())?;
let clean = normalize("foo/./bar/../baz")?;
assert_eq!(clean, PathBuf::from("foo/baz"));
let _ = expanded;
```

## Why this crate exists

CLI tools, build systems, deployers, and workspace managers repeatedly reimplement:

- `~` and environment variable expansion
- platform config/data/cache locations
- Windows drive / UNC / verbatim path quirks
- lexical normalization vs filesystem canonicalization
- safe-ish joining under a root
- directory listing and glob search
- optional discovery caching

`path-rs` provides one coherent, well-tested layer for those jobs while leaving filesystem identity to `Path` / `PathBuf`.

## Path versus PathBuf

| Type | Role |
| --- | --- |
| `std::path::Path` / `PathBuf` | Native path representation and OS APIs |
| `path-rs` | Expansion, lexical ops, platform dirs, listing, search, cache |

Internal representation is always `Path` / `PathBuf` / `OsStr` — never `String` as a filesystem path.

## Installation

```toml
[dependencies]
path-rs = "0.1"
```

Default features: `listing`, `search`.

## Operation matrix

| Operation | Filesystem access | Requires existence | Resolves symlinks |
| --- | ---: | ---: | ---: |
| `expand_input` | No* | No | No |
| `normalize` | No | No | No |
| `resolve_against` | No | No | No |
| `join_relative` | No | No | No |
| `resolve_inside` | No | No | No |
| `canonicalize_existing` | Yes | Yes | Yes |
| `list` | Yes | Root yes | Configurable |
| `search` | Yes | Root yes | Configurable |

\* Home / platform directory resolution may consult environment variables and platform APIs.

## Basic usage

```rust
use path_rs::{
    expand_input, normalize, resolve_inside, ExpandOptions,
};

let input = expand_input("%USERPROFILE%\\Documents", &ExpandOptions::default())?;
let clean = normalize(&input)?;
// Lexical containment — not symlink-safe:
let inside = resolve_inside(r"C:\repo", "src\\main.rs");
let _ = (clean, inside);
```

## Expansion

```rust
use path_rs::{expand_input, ExpandOptions};

let opts = ExpandOptions {
    expand_tilde: true,
    expand_percent_variables: true,
    expand_dollar_variables: true,
    translate_wsl_paths: false,
    reject_undefined_variables: true,
    trim_cli_input: true,
    max_expansion_depth: 8,
};

let path = expand_input("~/work/${PROJECT}", &opts)?;
```

Supported forms:

- `~`, `~/path`, `~\path` (leading component only; no `~user`)
- `%VAR%` (including `%%` escape)
- `$VAR`, `${VAR}`
- Optional WSL: `/mnt/c/...` → `C:\...` when `translate_wsl_paths` is enabled

Never expands `$(command)`, backticks, or shell syntax.

**Undefined variables:** strict mode errors; permissive mode leaves the token unchanged.

## Normalization

```rust
use path_rs::{normalize, canonicalize_existing};

// Lexical only — no I/O:
let n = normalize("foo/./bar/../baz")?;

// Filesystem — must exist, resolves symlinks:
let c = canonicalize_existing(".")?;
```

Do not call `canonicalize` merely to “clean” a path.

## Relative resolution and root containment

```rust
use path_rs::{join_relative, resolve_against, resolve_inside};

let a = resolve_against("/repo", "src/main.rs")?;
let b = join_relative("/repo", "src/main.rs")?; // rejects absolute children
let c = resolve_inside("/repo", "../etc/passwd"); // Err(RootEscape)
```

**Security:** lexical containment is **not** a symlink-safe boundary. See [SECURITY.md](SECURITY.md).

## Windows paths

Handles drive-absolute (`C:\`, `C:/`), rejects drive-relative (`C:foo`, `C:`) by default, preserves UNC and verbatim prefixes, and classifies device namespaces (`\\.\`).

```rust
use path_rs::{is_drive_relative, is_unc, is_verbatim};
use std::path::Path;

#[cfg(windows)]
{
    assert!(is_drive_relative(Path::new(r"C:foo")));
    assert!(is_unc(Path::new(r"\\server\share")));
    assert!(is_verbatim(Path::new(r"\\?\C:\repo")));
}
```

Windows reserved device names (`CON`, `NUL`, `COM1`, …) can be detected for app-name validation and policy checks.

Long path behavior follows the OS and process configuration; this crate does not invent MAX_PATH guarantees.

## WSL paths

Translation is **opt-in** (`ExpandOptions::translate_wsl_paths` or `translate_wsl_path`).

Only `/mnt/<single-letter-drive>/...` is recognized. `/mnt/data` is not treated as a drive.

## UTF-8 behavior

```rust
use path_rs::{path_to_utf8, path_to_string_lossy};

let s = path_to_utf8(std::path::Path::new("foo"))?;
let log = path_to_string_lossy(std::path::Path::new("foo")); // logs/UI only
```

Never round-trip lossy UTF-8 back into filesystem operations.

With feature `unicode`, `logical_path_key` provides NFC comparison keys (not filesystem paths).

## Listing

```rust
use path_rs::{list, ListOptions};

let entries = list(
    ".",
    &ListOptions::new()
        .recursive(true)
        .include_hidden(false)
        .max_depth(Some(8)),
)?;
```

Defaults: no symlink following, no cache, hidden files excluded, fail-fast errors, deterministic path sort when enabled.

## Glob searching

```rust
use path_rs::{search, SearchRequest};

let hits = search(&SearchRequest::new(".", ["**/*.rs", "**/Cargo.toml"]))?;
```

Patterns match paths relative to the search root. Use `search_with` for predicate-based filters.

## Application roots

```rust
use path_rs::{app_paths_with_options, AppPathsOptions};

let paths = app_paths_with_options(AppPathsOptions {
    application_name: "my-tool".into(),
    environment_override: Some("MY_TOOL_HOME".into()),
    create_directories: true,
    root_policy: None,
})?;
// Product-specific subdirs (e.g. "mutations") stay in the application layer:
let _custom = paths.root_dir.join("custom-subdir");
```

Never conflate the **global application root** with a **repository root** or the process CWD.

## Path identity keys

```rust
use path_rs::{path_identity_key, PathIdentityOptions, CaseNormalization};

let key = path_identity_key(
    r"C:\Users\Floris\Repo",
    PathIdentityOptions {
        case: CaseNormalization::AsciiLowercase,
        ..PathIdentityOptions::default()
    },
)?;
// Comparison only — not a filesystem path.
```

## Directory discovery

```rust
use path_rs::{discover_where, DiscoveryOptions};

// Application supplies the domain predicate (e.g. "has a marker file"):
let roots = discover_where(".", &DiscoveryOptions::default(), |path, _| {
    path.join("Cargo.toml").is_file()
})?;
```

Skip lists (`node_modules`, `.git`, …) are **caller-configured**, not hardcoded.

## Caching

Caching is **opt-in** (`CachePolicy::Bypass` by default).

```rust
use path_rs::{
    search_with_cache, CacheMode, CacheOptions, CachePolicy, MemoryCache, SearchRequest,
};
use std::sync::Arc;
use std::time::Duration;

let cache = Arc::new(MemoryCache::new(CacheOptions {
    mode: CacheMode::Memory,
    ttl: Some(Duration::from_secs(60)),
    max_entries: 128,
    validate_metadata: false,
}));

let mut req = SearchRequest::new(".", ["**/*.rs"]);
req.cache = CachePolicy::ReadThrough;
let hits = search_with_cache(&req, Some(cache.as_ref()))?;
```

Cache keys include root, patterns, and traversal options — not the root alone.

Feature `persistent-cache` enables a versioned on-disk cache under the platform cache directory.

**Never** use cache hits as a security boundary.

## Security limitations

- Lexical normalization ≠ canonicalization
- Lexical root containment ≠ symlink-safe containment
- Cached results are not authoritative
- Environment expansion is untrusted input
- Glob/search can produce large result sets — set limits

See [SECURITY.md](SECURITY.md).

## Platform support

| Platform | Status |
| --- | --- |
| Windows | Supported |
| macOS | Supported |
| Linux | Supported |
| Other Unix-like | Best-effort via `dirs` / `std` |

CI runs on Ubuntu, macOS, and Windows.

## Feature flags

| Feature | Default | Description |
| --- | :---: | --- |
| `listing` | yes | Directory listing (`walkdir`) |
| `search` | yes | Glob/predicate search (`globset`, implies `listing`) |
| `persistent-cache` | no | On-disk discovery cache |
| `unicode` | no | NFC logical path keys |
| `async` | no | `spawn_blocking` list/search wrappers |

## MSRV

**Rust 1.85** (edition 2024).

## Examples

```bash
cargo run --example expand
cargo run --example normalize
cargo run --example list
cargo run --example search
cargo run --example cache
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
