//! Directory listing: `list`, streaming `walk`, and `sort_entries`.

use path_rs::{EntryKind, FileEntry, ListOptions, SortMode, list, sort_entries, walk};

fn main() -> Result<(), path_rs::PathError> {
    // --- list (sorted, materialised) ---
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

    // --- walk (streaming, walk order) ---
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

    // --- sort_entries (standalone re-sort of FileEntry slices) ---
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
