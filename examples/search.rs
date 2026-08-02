//! Glob search, predicate search, and built-in search predicates.

use path_rs::{
    EntryKind, ListOptions, SearchRequest, search,
    search::predicates,
    search_with,
};

fn main() -> Result<(), path_rs::PathError> {
    // --- search (glob include / exclude) ---
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

    // --- search_with (predicate) ---
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

    // Other built-in predicates (size / modified / hidden) compose the same way:
    let _ = predicates::min_size(0);
    let _ = predicates::max_size(u64::MAX);
    let _ = predicates::is_hidden();

    Ok(())
}
