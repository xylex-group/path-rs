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

    // --- discover_directories ---
    let all = discover_directories(".", &opts)?;
    println!("discover_directories => {} dirs", all.len());
    for p in all.iter().take(5) {
        println!("  {}", p.display());
    }

    // --- discover_where (predicate) ---
    let named = discover_where(".", &opts, |path, _| {
        path.file_name()
            .is_some_and(|n| n == "src" || n == "examples" || n == "tests")
    })?;
    println!("discover_where (src/examples/tests) => {} dirs", named.len());
    for p in &named {
        println!("  {}", p.display());
    }

    // --- visit_directories (visitor + closure adapter) ---
    let mut counter = Counter { seen: 0 };
    visit_directories(".", &opts, &mut counter)?;
    println!("visit_directories visitor saw {}", counter.seen);

    // Closure adapter (higher-ranked lifetimes via explicit signature):
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
