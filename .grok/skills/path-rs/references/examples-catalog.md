# path-rs examples catalog

Canonical sources live in the repository `examples/` directory. This file is a mirror for agents: purpose, required features, and full program text as of skill creation. Prefer the live `examples/*.rs` if they diverge.

Run from repo root with the commands in the table below.

| File | Run | Required features |
| --- | --- | --- |
| `expand.rs` | `cargo run --example expand` | (none beyond defaults) |
| `normalize.rs` | `cargo run --example normalize` | — |
| `resolve.rs` | `cargo run --example resolve` | — |
| `dirs.rs` | `cargo run --example dirs` | — |
| `identity.rs` | `cargo run --example identity` | — |
| `inspect.rs` | `cargo run --example inspect` | — |
| `match_path.rs` | `cargo run --example match_path` | — |
| `platform.rs` | `cargo run --example platform` | — |
| `text_utf8.rs` | `cargo run --example text_utf8` | optional `--features unicode` |
| `list.rs` | `cargo run --example list` | `listing` (default) |
| `discovery.rs` | `cargo run --example discovery` | `listing` |
| `search.rs` | `cargo run --example search` | `search` |
| `cache.rs` | `cargo run --example cache` | `search` |
| `persistent_cache.rs` | `cargo run --example persistent_cache --features persistent-cache` | `search` + `persistent-cache` |

---

## expand.rs

Expand user path input: tilde, `%VAR%`, `$VAR` / `${VAR}`, and the high-level API.

```rust
//! Expand user path input: tilde, `%VAR%`, `$VAR` / `${VAR}`, and the high-level API.

use path_rs::{
    ExpandOptions, expand_dollar_variables, expand_input, expand_percent_variables, expand_tilde,
};

fn main() -> Result<(), path_rs::PathError> {
    let home = expand_tilde("~")?;
    println!("expand_tilde(\"~\")           => {home}");
    let projects = expand_tilde("~/projects")?;
    println!("expand_tilde(\"~/projects\")  => {projects}");

    let percent = expand_percent_variables("%TEMP%\\path-rs", false)?;
    println!("expand_percent_variables     => {percent}");

    let dollar = expand_dollar_variables("$HOME/work", false)?;
    println!("expand_dollar_variables      => {dollar}");

    let opts = ExpandOptions::default();
    let full = expand_input("~", &opts)?;
    println!("expand_input(\"~\")            => {}", full.display());

    let opts_permissive = ExpandOptions {
        reject_undefined_variables: false,
        ..ExpandOptions::default()
    };
    let mixed = expand_input("~/projects", &opts_permissive)?;
    println!("expand_input(\"~/projects\")   => {}", mixed.display());

    let none = ExpandOptions::none();
    let literal = expand_input("~", &none)?;
    println!("ExpandOptions::none()        => {}", literal.display());

    Ok(())
}
```

---

## normalize.rs

Lexical normalization vs filesystem canonicalization.

```rust
//! Lexical normalization vs filesystem canonicalization.

use path_rs::{canonicalize_existing, normalize};

fn main() -> Result<(), path_rs::PathError> {
    for sample in [
        "foo//bar",
        "foo/./bar",
        "foo/../bar",
        "./foo",
        "foo/../../bar",
    ] {
        let n = normalize(sample)?;
        println!("normalize({sample:>16}) => {}", n.display());
    }

    let canon = canonicalize_existing(".")?;
    println!("canonicalize_existing(\".\")  => {}", canon.display());

    Ok(())
}
```

---

## resolve.rs

Absolute paths, relative joining, and lexical root containment.

```rust
//! Absolute paths, relative joining, and lexical root containment.

use path_rs::{
    absolute, ensure_inside, is_lexically_inside, join_relative, resolve_against, resolve_inside,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    let abs = absolute("src/lib.rs")?;
    println!("absolute(\"src/lib.rs\")      => {}", abs.display());

    #[cfg(windows)]
    let resolve_base = Path::new(r"C:\repo");
    #[cfg(not(windows))]
    let resolve_base = Path::new("/repo");

    let against = resolve_against(resolve_base, "src/main.rs")?;
    println!("resolve_against             => {}", against.display());

    #[cfg(windows)]
    let abs_input = Path::new(r"D:\other\file.txt");
    #[cfg(not(windows))]
    let abs_input = Path::new("/etc/passwd");
    let abs_in = resolve_against(resolve_base, abs_input)?;
    println!("resolve_against abs input   => {}", abs_in.display());

    #[cfg(windows)]
    let base = Path::new(r"C:\repo");
    #[cfg(not(windows))]
    let base = Path::new("/repo");

    let joined = join_relative(base, "src/main.rs")?;
    println!("join_relative               => {}", joined.display());

    #[cfg(windows)]
    let absolute_child = Path::new(r"C:\Windows\System32");
    #[cfg(not(windows))]
    let absolute_child = Path::new("/etc/passwd");

    match join_relative(base, absolute_child) {
        Ok(p) => println!("unexpected join ok: {}", p.display()),
        Err(e) => println!("join_relative rejects abs => {e}"),
    }

    let inside = resolve_inside(base, "src/main.rs")?;
    println!("resolve_inside ok           => {}", inside.display());
    match resolve_inside(base, "../escape") {
        Ok(p) => println!("unexpected escape ok: {}", p.display()),
        Err(e) => println!("resolve_inside escape     => {e}"),
    }

    let child = base.join("src");
    println!(
        "is_lexically_inside          => {}",
        is_lexically_inside(&child, base)
    );
    let ensured = ensure_inside(base, base.join("src").join("lib.rs"))?;
    println!("ensure_inside               => {}", ensured.display());

    Ok(())
}
```

---

## dirs.rs

Platform directories and application path roots.

```rust
//! Platform directories and application path roots.

use path_rs::{
    AppPathsOptions, AppRootPolicy, app_paths, app_paths_with_options, app_paths_with_policy,
    cache_dir, config_dir, data_dir, platform_dirs, temp_dir,
};

fn main() -> Result<(), path_rs::PathError> {
    let dirs = platform_dirs()?;
    println!("home   = {}", dirs.home.display());
    println!("config = {}", dirs.config.display());
    println!("data   = {}", dirs.data.display());
    println!("cache  = {}", dirs.cache.display());
    if let Some(state) = &dirs.state {
        println!("state  = {}", state.display());
    }
    if let Some(runtime) = &dirs.runtime {
        println!("runtime= {}", runtime.display());
    }
    println!("temp   = {}", dirs.temp.display());

    println!("config_dir(demo) = {}", config_dir("path-rs-demo")?.display());
    println!("data_dir(demo)   = {}", data_dir("path-rs-demo")?.display());
    println!("cache_dir(demo)  = {}", cache_dir("path-rs-demo")?.display());
    println!("temp_dir()       = {}", temp_dir().display());

    let paths = app_paths("path-rs-demo")?;
    println!("app root   = {}", paths.root_dir.display());
    println!("app config = {}", paths.config_dir.display());
    println!("app data   = {}", paths.data_dir.display());
    println!("app cache  = {}", paths.cache_dir.display());

    let opts = AppPathsOptions::new("path-rs-demo");
    let with_opts = app_paths_with_options(opts)?;
    println!("with_options root = {}", with_opts.root_dir.display());

    let with_policy = app_paths_with_policy(
        "path-rs-demo",
        AppRootPolicy::PlatformDefault,
        false,
    )?;
    println!("with_policy root  = {}", with_policy.root_dir.display());

    Ok(())
}
```

---

## identity.rs

Path identity keys, display strings, records, and deduplication.

```rust
//! Path identity keys, display strings, records, and deduplication.

use path_rs::{
    CaseNormalization, PathIdentityOptions, PathRecord, deduplicate_paths, path_display_string,
    path_identity_key,
};
use std::path::{Path, PathBuf};

fn main() -> Result<(), path_rs::PathError> {
    let opts = PathIdentityOptions {
        case: CaseNormalization::PlatformDefault,
        ..PathIdentityOptions::new()
    };

    let key_a = path_identity_key(Path::new("Foo/./Bar"), opts)?;
    let key_b = path_identity_key(Path::new("Foo/Bar"), opts)?;
    println!("path_identity_key(\"Foo/./Bar\") = {key_a}");
    println!("path_identity_key(\"Foo/Bar\")   = {key_b}");
    println!("keys equal = {}", key_a == key_b);

    let display = path_display_string(Path::new("src/lib.rs"));
    println!("path_display_string = {display}");

    let record = PathRecord::from_path("src/lib.rs", Some(opts))?;
    println!(
        "PathRecord path={}, display={}, key={:?}",
        record.path.display(),
        record.display,
        record.identity_key
    );

    let paths = [
        PathBuf::from("Foo/Bar"),
        PathBuf::from("Foo/./Bar"),
        PathBuf::from("other"),
    ];
    let unique = deduplicate_paths(paths, opts)?;
    println!(
        "deduplicate_paths => {:?}",
        unique
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    Ok(())
}
```

---

## inspect.rs

Directory existence checks and inspection summaries.

```rust
//! Directory existence checks and inspection summaries.

use path_rs::{
    directory_exists, inspect_directory, is_existing_directory, require_directory,
};

fn main() -> Result<(), path_rs::PathError> {
    println!("directory_exists(\".\")       = {}", directory_exists("."));
    println!(
        "is_existing_directory(\".\")  = {}",
        is_existing_directory(".")
    );
    println!(
        "directory_exists(\"nope\")    = {}",
        directory_exists("nope-missing-path-rs")
    );

    let info = inspect_directory(".")?;
    println!("inspect_directory(\".\"):");
    println!("  exists       = {}", info.exists);
    println!("  is_directory = {}", info.is_directory);
    println!("  is_symlink   = {}", info.is_symlink);
    println!("  is_readable  = {:?}", info.is_readable);
    if let Some(meta) = &info.metadata {
        println!("  len          = {}", meta.len);
        println!("  modified     = {:?}", meta.modified);
        println!("  readonly     = {}", meta.readonly);
    }

    let dir = require_directory(".")?;
    println!("require_directory(\".\")      = {}", dir.display());

    Ok(())
}
```

---

## match_path.rs

Executable path comparison and command-line path detection.

```rust
//! Executable path comparison and command-line path detection.

use path_rs::{
    CommandLinePathMatchOptions, ExecutableMatchOptions, command_line_contains_path,
    executable_paths_match,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    let match_opts = ExecutableMatchOptions::new();
    let same = executable_paths_match(
        Path::new("C:/Tools/app.exe"),
        Path::new(r"C:\Tools\app.exe"),
        match_opts,
    )?;
    println!("executable_paths_match (slash variants) = {same}");

    let different = executable_paths_match(
        Path::new("/usr/bin/true"),
        Path::new("/usr/bin/false"),
        match_opts,
    )?;
    println!("executable_paths_match (different)      = {different}");

    let cli_opts = CommandLinePathMatchOptions::new();
    let cmdline = r#"tool --input "C:\repo\src\main.rs" --flag"#;
    let found = command_line_contains_path(cmdline, Path::new(r"C:\repo\src\main.rs"), cli_opts)?;
    println!("command_line_contains_path              = {found}");

    let missing =
        command_line_contains_path(cmdline, Path::new(r"C:\other\file.txt"), cli_opts)?;
    println!("command_line_contains_path (missing)    = {missing}");

    Ok(())
}
```

---

## platform.rs

Windows path classification, reserved names, WSL translation, display simplify.

```rust
//! Windows path classification, reserved names, WSL translation, display simplify.

use path_rs::{
    is_device_namespace, is_drive_relative, is_reserved_windows_name, is_unc, is_verbatim,
    path_contains_reserved_name, simplify_for_display, translate_wsl_path,
};
use std::ffi::OsStr;
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    let samples: &[(&str, &str)] = &[
        (r"C:\repo", "drive-absolute"),
        (r"C:foo", "drive-relative"),
        (r"\\server\share\path", "UNC"),
        (r"\\?\C:\repo", "verbatim"),
        (r"\\.\pipe\name", "device namespace"),
        (r"folder\NUL.txt", "reserved component"),
    ];

    for (raw, label) in samples {
        let p = Path::new(raw);
        println!("{label}: {raw}");
        println!("  is_drive_relative          = {}", is_drive_relative(p));
        println!("  is_unc                     = {}", is_unc(p));
        println!("  is_verbatim                = {}", is_verbatim(p));
        println!("  is_device_namespace        = {}", is_device_namespace(p));
        println!(
            "  path_contains_reserved_name = {}",
            path_contains_reserved_name(p)
        );
    }

    println!(
        "is_reserved_windows_name(\"CON\") = {}",
        is_reserved_windows_name(OsStr::new("CON"))
    );
    println!(
        "is_reserved_windows_name(\"file\") = {}",
        is_reserved_windows_name(OsStr::new("file.txt"))
    );

    match translate_wsl_path("/mnt/c/Users/demo")? {
        Some(win) => println!("translate_wsl_path => {}", win.display()),
        None => println!("translate_wsl_path => None (not a WSL mount)"),
    }
    println!(
        "translate_wsl_path(\"/home/x\") => {:?}",
        translate_wsl_path("/home/x")?
    );

    let simplified = simplify_for_display(Path::new(r"\\?\C:\repo"));
    println!("simplify_for_display(verbatim) => {}", simplified.display());

    Ok(())
}
```

---

## text_utf8.rs

Text token normalization and UTF-8 path helpers.

```rust
//! Text token normalization and UTF-8 path helpers.

use path_rs::{
    CaseNormalization, TextNormalizationOptions, normalize_path_token, path_to_string_lossy,
    path_to_utf8,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    let opts = TextNormalizationOptions {
        trim_whitespace: true,
        trim_trailing_separators: true,
        normalize_separators: true,
        case: CaseNormalization::AsciiLowercase,
    };
    let token = normalize_path_token(r"  Foo\Bar\  ", &opts);
    println!("normalize_path_token => {token:?}");

    let preserve = TextNormalizationOptions::new();
    println!(
        "default token        => {:?}",
        normalize_path_token("Repo/Name/", &preserve)
    );

    println!(
        "UnicodeLowercase     => {}",
        CaseNormalization::UnicodeLowercase.apply("StraAYe")
    );

    let utf8 = path_to_utf8(Path::new("src/lib.rs"))?;
    println!("path_to_utf8         => {utf8}");

    let lossy = path_to_string_lossy(Path::new("src/lib.rs"));
    println!("path_to_string_lossy => {lossy}");

    #[cfg(feature = "unicode")]
    {
        use path_rs::logical_path_key;
        let key = logical_path_key(Path::new("café/path"));
        println!("logical_path_key     => {key}");
    }

    Ok(())
}
```

---

## list.rs

Directory listing: `list`, streaming `walk`, and `sort_entries`. Requires feature `listing`.

```rust
//! Directory listing: `list`, streaming `walk`, and `sort_entries`.

use path_rs::{EntryKind, FileEntry, ListOptions, SortMode, list, sort_entries, walk};

fn main() -> Result<(), path_rs::PathError> {
    let opts = ListOptions::new()
        .recursive(false)
        .include_hidden(false)
        .max_depth(Some(1))
        .sort(SortMode::DirsFirst);
    let entries = list(".", &opts)?;
    println!("list ({} entries):", entries.len());
    for entry in entries.iter().take(8) {
        println!("  {:?} {}", entry.kind, entry.path.display());
    }

    let walk_opts = ListOptions::new().recursive(true).max_depth(Some(2));
    let mut count = 0usize;
    for entry in walk(".", &walk_opts)? {
        let entry = entry?;
        count += 1;
        if count <= 5 {
            println!("walk: {:?} {}", entry.kind, entry.path.display());
        }
    }
    println!("walk total = {count}");

    let mut batch = vec![
        FileEntry::new("z.txt".into(), EntryKind::File),
        FileEntry::new("a".into(), EntryKind::Directory),
        FileEntry::new("m.rs".into(), EntryKind::File),
    ];
    sort_entries(&mut batch, SortMode::DirsFirst);
    println!(
        "sort_entries DirsFirst => {:?}",
        batch
            .iter()
            .map(|e| e.path.display().to_string())
            .collect::<Vec<_>>()
    );

    Ok(())
}
```

---

## discovery.rs

Directory discovery and visitor walks. Requires feature `listing`.

```rust
//! Directory discovery and visitor walks (feature `listing`).

use path_rs::{
    DirectoryInspection, DirectoryVisitor, DiscoveryOptions, VisitControl, discover_directories,
    discover_where, visit_directories,
};
use std::path::Path;

struct Counter {
    seen: usize,
}

impl DirectoryVisitor for Counter {
    fn visit_directory(&mut self, path: &Path, inspection: &DirectoryInspection) -> VisitControl {
        self.seen += 1;
        if self.seen <= 5 {
            println!(
                "visit: {} (dir={}, readable={:?})",
                path.display(),
                inspection.is_directory,
                inspection.is_readable
            );
        }
        if self.seen >= 20 {
            VisitControl::Stop
        } else {
            VisitControl::Continue
        }
    }

    fn visit_error(&mut self, path: &Path, error: &path_rs::PathError) -> VisitControl {
        eprintln!("visit error at {}: {error}", path.display());
        VisitControl::Continue
    }
}

fn main() -> Result<(), path_rs::PathError> {
    let opts = DiscoveryOptions::new()
        .recursive(true)
        .max_depth(Some(2))
        .max_entries(Some(50))
        .skip_names(["target", ".git"]);

    let all = discover_directories(".", &opts)?;
    println!("discover_directories => {} dirs", all.len());
    for p in all.iter().take(5) {
        println!("  {}", p.display());
    }

    let named = discover_where(".", &opts, |path, _| {
        path.file_name()
            .is_some_and(|n| n == "src" || n == "examples" || n == "tests")
    })?;
    println!("discover_where (src/examples/tests) => {} dirs", named.len());
    for p in &named {
        println!("  {}", p.display());
    }

    let mut counter = Counter { seen: 0 };
    visit_directories(".", &opts, &mut counter)?;
    println!("visit_directories visitor saw {}", counter.seen);

    let mut closure_count = 0usize;
    visit_directories(
        ".",
        &opts,
        &mut |path: &Path, inspection: &DirectoryInspection| {
            let _ = (path, inspection);
            closure_count += 1;
            if closure_count >= 10 {
                VisitControl::SkipChildren
            } else {
                VisitControl::Continue
            }
        },
    )?;
    println!("visit_directories closure count = {closure_count}");

    Ok(())
}
```

---

## search.rs

Glob search, predicate search, and built-in search predicates. Requires feature `search`.

```rust
//! Glob search, predicate search, and built-in search predicates.

use path_rs::{
    EntryKind, ListOptions, SearchRequest, search,
    search::predicates,
    search_with,
};

fn main() -> Result<(), path_rs::PathError> {
    let request = SearchRequest::new(".", ["**/*.rs", "**/Cargo.toml"])
        .exclude(["**/target/**"])
        .options(ListOptions::new().recursive(true).max_depth(Some(3)));
    let hits = search(&request)?;
    println!("search globs ({} hits):", hits.len());
    for entry in hits.iter().take(10) {
        let rel = entry
            .relative_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| entry.path.display().to_string());
        println!("  {rel}");
    }

    let opts = ListOptions::new().recursive(true).max_depth(Some(2));
    let rs_only = search_with(".", &opts, predicates::extension("rs"))?;
    println!("search_with extension(\"rs\") => {} entries", rs_only.len());

    let cargo = search_with(".", &opts, predicates::filename("Cargo.toml"))?;
    println!(
        "search_with filename(\"Cargo.toml\") => {} entries",
        cargo.len()
    );

    let dirs = search_with(".", &opts, predicates::kind(EntryKind::Directory))?;
    println!("search_with kind(Directory) => {} entries", dirs.len());

    let _ = predicates::min_size(0);
    let _ = predicates::max_size(u64::MAX);
    let _ = predicates::is_hidden();

    Ok(())
}
```

---

## cache.rs

In-memory discovery cache: put/get/invalidate/clear and `search_with_cache`. Requires feature `search`.

```rust
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

    let mut request = SearchRequest::new(".", ["**/*.rs"]);
    request.cache = CachePolicy::ReadThrough;

    let cold = search_with_cache(&request, Some(cache.as_ref()))?;
    let warm = search_with_cache(&request, Some(cache.as_ref()))?;
    println!("cold hits = {}, warm hits = {}", cold.len(), warm.len());
    println!("cache keys = {}", cache.len()?);
    println!("is_empty = {}", cache.is_empty()?);

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
```

---

## persistent_cache.rs

On-disk discovery cache. Requires features `search` + `persistent-cache`.

```rust
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
```
