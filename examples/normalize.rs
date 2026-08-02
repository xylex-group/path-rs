//! Lexical normalization vs filesystem canonicalization.

use path_rs::{canonicalize_existing, normalize};

fn main() -> Result<(), path_rs::PathError> {
    // --- normalize (lexical only — no I/O, no symlink resolution) ---
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

    // --- canonicalize_existing (must exist; resolves symlinks) ---
    let canon = canonicalize_existing(".")?;
    println!("canonicalize_existing(\".\")  => {}", canon.display());

    Ok(())
}
