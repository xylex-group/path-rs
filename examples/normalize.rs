//! Lexical path normalization (no filesystem access).

use path_rs::normalize;

fn main() -> Result<(), path_rs::PathError> {
    for sample in [
        "foo//bar",
        "foo/./bar",
        "foo/../bar",
        "./foo",
        "foo/../../bar",
    ] {
        let n = normalize(sample)?;
        println!("{sample:>16} => {}", n.display());
    }
    Ok(())
}
