//! List files in the current directory.

use path_rs::{ListOptions, list};

fn main() -> Result<(), path_rs::PathError> {
    let opts = ListOptions::new()
        .recursive(false)
        .include_hidden(false)
        .max_depth(Some(1));
    let entries = list(".", &opts)?;
    for entry in entries {
        println!("{:?} {}", entry.kind, entry.path.display());
    }
    Ok(())
}
