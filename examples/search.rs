//! Glob search under the current directory.

use path_rs::{SearchRequest, search};

fn main() -> Result<(), path_rs::PathError> {
    let request = SearchRequest::new(".", ["**/*.rs", "**/Cargo.toml"]);
    let hits = search(&request)?;
    for entry in hits {
        let rel = entry
            .relative_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| entry.path.display().to_string());
        println!("{rel}");
    }
    Ok(())
}
