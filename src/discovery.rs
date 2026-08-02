//! Generic recursive directory discovery (no VCS or product-specific roots).
//!
//! Enabled by the `listing` feature. Callers supply predicates for domain rules
//! (for example, detecting a project marker directory).

use crate::error::PathError;
use crate::identity::{PathIdentityOptions, path_identity_key};
use crate::inspect::{DirectoryInspection, inspect_directory};
use crate::internal::validation::reject_nul_path;
use crate::metadata::{SortMode, TraversalErrorPolicy};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Options for recursive directory discovery.
///
/// Skip rules are **caller-configured**. This crate does not hardcode
/// `node_modules`, `.git`, `target`, etc.
///
/// # Depth model
///
/// Same as [`crate::listing::ListOptions`]:
///
/// - **Depth 0** — the walk root
/// - **Depth 1** — immediate children
/// - `recursive: false` limits the walk to depth 0 only (the root)
/// - `recursive: true` with `max_depth: Some(n)` uses walkdir `max_depth = n`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryOptions {
    /// Recurse into subdirectories.
    pub recursive: bool,
    /// Follow symbolic links (best-effort cycle detection via walkdir).
    pub follow_symlinks: bool,
    /// Maximum walk depth relative to the root (`None` = unlimited when recursive).
    ///
    /// Depth `0` is the root itself; `1` is immediate children.
    /// Ignored when `recursive` is false (only the root is considered).
    pub max_depth: Option<usize>,
    /// Maximum number of directories returned.
    pub max_entries: Option<usize>,
    /// Directory **names** to skip (exact component match). Never applied to the root itself.
    pub skip_directory_names: Vec<String>,
    /// Relative path prefixes (using `/` separators) to skip under the root.
    pub skip_relative_prefixes: Vec<String>,
    /// Glob patterns matched against relative paths (requires `search` / `globset`).
    #[cfg(feature = "search")]
    pub skip_globs: Vec<String>,
    /// Error handling policy.
    pub error_policy: TraversalErrorPolicy,
    /// Deduplicate results by identity key.
    pub deduplicate: bool,
    /// Identity options used when `deduplicate` is true.
    pub identity: PathIdentityOptions,
    /// Sort mode for the returned list.
    pub sort: SortMode,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            follow_symlinks: false,
            max_depth: None,
            max_entries: None,
            skip_directory_names: Vec::new(),
            skip_relative_prefixes: Vec::new(),
            #[cfg(feature = "search")]
            skip_globs: Vec::new(),
            error_policy: TraversalErrorPolicy::FailFast,
            deduplicate: true,
            identity: PathIdentityOptions::default(),
            sort: SortMode::Path,
        }
    }
}

impl DiscoveryOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable recursion.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Follow symlinks when walking.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Skip exact directory names (e.g. `"node_modules"`).
    pub fn skip_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.skip_directory_names
            .extend(names.into_iter().map(Into::into));
        self
    }

    /// Skip relative path prefixes under the root.
    pub fn skip_relative_prefixes(
        mut self,
        prefixes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.skip_relative_prefixes
            .extend(prefixes.into_iter().map(Into::into));
        self
    }

    /// Set maximum depth (see module / type docs).
    pub fn max_depth(mut self, depth: Option<usize>) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set maximum number of returned directories.
    pub fn max_entries(mut self, max: Option<usize>) -> Self {
        self.max_entries = max;
        self
    }

    /// Set error policy.
    pub fn error_policy(mut self, policy: TraversalErrorPolicy) -> Self {
        self.error_policy = policy;
        self
    }

    /// Enable or disable identity-key deduplication.
    pub fn deduplicate(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
    }

    /// Identity options used when deduplicating.
    pub fn identity(mut self, identity: PathIdentityOptions) -> Self {
        self.identity = identity;
        self
    }

    /// Sort mode for the returned list.
    pub fn sort(mut self, sort: SortMode) -> Self {
        self.sort = sort;
        self
    }
}

/// Control flow for visitor-based traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VisitControl {
    /// Continue walking.
    Continue,
    /// Do not descend into the current directory's children.
    SkipChildren,
    /// Stop the walk entirely.
    Stop,
}

/// Callback interface for directory walks.
pub trait DirectoryVisitor {
    /// Called for each directory entry.
    fn visit_directory(&mut self, path: &Path, inspection: &DirectoryInspection) -> VisitControl;

    /// Called when an error is encountered for `path`.
    fn visit_error(&mut self, path: &Path, error: &PathError) -> VisitControl {
        let _ = (path, error);
        VisitControl::Continue
    }
}

/// Adapter so closures can be used with [`visit_directories`].
///
/// Errors always continue (use a custom type for fail-fast via the visitor).
impl<F> DirectoryVisitor for F
where
    F: FnMut(&Path, &DirectoryInspection) -> VisitControl,
{
    fn visit_directory(&mut self, path: &Path, inspection: &DirectoryInspection) -> VisitControl {
        self(path, inspection)
    }
}

/// Discover directories under `root` (all directories, subject to skip rules).
///
/// # Filesystem access
///
/// **Yes.** Does not follow symlinks by default. Does not use a cache.
/// Does not spawn threads.
pub fn discover_directories(
    root: impl AsRef<Path>,
    options: &DiscoveryOptions,
) -> Result<Vec<PathBuf>, PathError> {
    discover_where(root, options, |_, inspection| inspection.is_directory)
}

/// Discover directories for which `predicate` returns true.
///
/// The predicate receives the directory path and a [`DirectoryInspection`].
/// Skip rules are applied before the predicate (except the walk root is never
/// skipped by name rules).
///
/// # Filesystem access
///
/// **Yes.**
pub fn discover_where(
    root: impl AsRef<Path>,
    options: &DiscoveryOptions,
    mut predicate: impl FnMut(&Path, &DirectoryInspection) -> bool,
) -> Result<Vec<PathBuf>, PathError> {
    let root = root.as_ref();
    reject_nul_path(root)?;
    let root_inspection = inspect_directory(root)?;
    if !root_inspection.exists || !root_inspection.is_directory {
        return Err(PathError::invalid(format!(
            "discovery root is not a directory: {}",
            root.to_string_lossy()
        )));
    }

    #[cfg(feature = "search")]
    let skip_globset = build_skip_globset(&options.skip_globs)?;

    let max_depth = if options.recursive {
        options.max_depth.unwrap_or(usize::MAX)
    } else {
        0
    };

    let walker = WalkDir::new(root)
        .min_depth(0)
        .max_depth(max_depth)
        .follow_links(options.follow_symlinks);

    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();
    let mut skip_prefixes: HashSet<PathBuf> = HashSet::new();

    for item in walker {
        if let Some(max) = options.max_entries {
            if out.len() >= max {
                break;
            }
        }

        let entry = match item {
            Ok(e) => e,
            Err(err) => {
                let path = err
                    .path()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| root.to_path_buf());
                let pe = PathError::traversal(format!("{} ({})", err, path.display()));
                match options.error_policy {
                    TraversalErrorPolicy::FailFast => return Err(pe),
                    TraversalErrorPolicy::SkipErrors => continue,
                }
            }
        };

        let path = entry.path();
        let file_type = entry.file_type();
        if !(file_type.is_dir() || (options.follow_symlinks && file_type.is_symlink())) {
            continue;
        }

        // Skip children of previously skipped trees.
        if skip_prefixes
            .iter()
            .any(|p| path.starts_with(p) && path != p)
        {
            continue;
        }

        let is_root = path == root;
        let name = entry.file_name().to_string_lossy();

        // Never skip the root merely because its name matches a skip rule.
        if !is_root {
            if options
                .skip_directory_names
                .iter()
                .any(|n| n.as_str() == name)
            {
                skip_prefixes.insert(path.to_path_buf());
                continue;
            }

            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if options
                    .skip_relative_prefixes
                    .iter()
                    .any(|p| rel_str == *p || rel_str.starts_with(&format!("{p}/")))
                {
                    skip_prefixes.insert(path.to_path_buf());
                    continue;
                }

                #[cfg(feature = "search")]
                if let Some(gs) = &skip_globset {
                    if gs.is_match(rel_str.as_str()) || gs.is_match(name.as_ref()) {
                        skip_prefixes.insert(path.to_path_buf());
                        continue;
                    }
                }
            }
        }

        let inspection = match inspect_directory(path) {
            Ok(i) => i,
            Err(e) => match options.error_policy {
                TraversalErrorPolicy::FailFast => return Err(e),
                TraversalErrorPolicy::SkipErrors => continue,
            },
        };

        if !inspection.is_directory {
            continue;
        }

        if !predicate(path, &inspection) {
            continue;
        }

        if options.deduplicate {
            let key = path_identity_key(path, options.identity)?;
            if !seen_keys.insert(key) {
                continue;
            }
        }

        out.push(path.to_path_buf());
    }

    sort_paths(&mut out, options.sort);
    Ok(out)
}

/// Walk directories with a [`DirectoryVisitor`].
///
/// # Filesystem access
///
/// **Yes.**
pub fn visit_directories(
    root: impl AsRef<Path>,
    options: &DiscoveryOptions,
    visitor: &mut dyn DirectoryVisitor,
) -> Result<(), PathError> {
    let root = root.as_ref();
    reject_nul_path(root)?;

    let max_depth = if options.recursive {
        options.max_depth.unwrap_or(usize::MAX)
    } else {
        0
    };

    let walker = WalkDir::new(root)
        .min_depth(0)
        .max_depth(max_depth)
        .follow_links(options.follow_symlinks);

    let mut skip_prefixes: HashSet<PathBuf> = HashSet::new();

    for item in walker {
        let entry = match item {
            Ok(e) => e,
            Err(err) => {
                let path = err
                    .path()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| root.to_path_buf());
                let pe = PathError::traversal(format!("{err}"));
                match visitor.visit_error(&path, &pe) {
                    VisitControl::Stop => return Ok(()),
                    VisitControl::SkipChildren | VisitControl::Continue => {
                        if matches!(options.error_policy, TraversalErrorPolicy::FailFast) {
                            return Err(pe);
                        }
                        continue;
                    }
                }
            }
        };

        let path = entry.path();
        if !entry.file_type().is_dir() {
            continue;
        }

        if skip_prefixes
            .iter()
            .any(|p| path.starts_with(p) && path != p)
        {
            continue;
        }

        let is_root = path == root;
        if !is_root {
            let name = entry.file_name().to_string_lossy();
            if options
                .skip_directory_names
                .iter()
                .any(|n| n.as_str() == name)
            {
                skip_prefixes.insert(path.to_path_buf());
                continue;
            }
        }

        let inspection = match inspect_directory(path) {
            Ok(i) => i,
            Err(e) => match visitor.visit_error(path, &e) {
                VisitControl::Stop => return Ok(()),
                _ if matches!(options.error_policy, TraversalErrorPolicy::FailFast) => {
                    return Err(e);
                }
                _ => continue,
            },
        };

        match visitor.visit_directory(path, &inspection) {
            VisitControl::Continue => {}
            VisitControl::SkipChildren => {
                skip_prefixes.insert(path.to_path_buf());
            }
            VisitControl::Stop => return Ok(()),
        }
    }

    Ok(())
}

fn sort_paths(paths: &mut [PathBuf], mode: SortMode) {
    match mode {
        SortMode::None => {}
        SortMode::Path | SortMode::DirsFirst => {
            paths.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
        }
        SortMode::Name => {
            paths.sort_by(|a, b| {
                let an = a
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                let bn = b
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                an.cmp(&bn)
                    .then_with(|| a.to_string_lossy().cmp(&b.to_string_lossy()))
            });
        }
    }
}

#[cfg(feature = "search")]
fn build_skip_globset(patterns: &[String]) -> Result<Option<globset::GlobSet>, PathError> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = globset::GlobSetBuilder::new();
    for p in patterns {
        let g = globset::Glob::new(p).map_err(|e| PathError::InvalidGlob {
            message: e.to_string(),
        })?;
        builder.add(g);
    }
    let set = builder.build().map_err(|e| PathError::InvalidGlob {
        message: e.to_string(),
    })?;
    Ok(Some(set))
}
