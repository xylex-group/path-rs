//! Absolute paths, relative joining, and lexical root containment.

use path_rs::{
    absolute, ensure_inside, is_lexically_inside, join_relative, resolve_against, resolve_inside,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    // --- absolute ---
    let abs = absolute("src/lib.rs")?;
    println!("absolute(\"src/lib.rs\")      => {}", abs.display());

    // --- resolve_against ---
    #[cfg(windows)]
    let resolve_base = Path::new(r"C:\repo");
    #[cfg(not(windows))]
    let resolve_base = Path::new("/repo");

    let against = resolve_against(resolve_base, "src/main.rs")?;
    println!("resolve_against             => {}", against.display());

    // Absolute input ignores the base:
    #[cfg(windows)]
    let abs_input = Path::new(r"D:\other\file.txt");
    #[cfg(not(windows))]
    let abs_input = Path::new("/etc/passwd");
    let abs_in = resolve_against(resolve_base, abs_input)?;
    println!("resolve_against abs input   => {}", abs_in.display());

    // --- join_relative (rejects absolute children) ---
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

    // --- resolve_inside (lexical containment) ---
    let inside = resolve_inside(base, "src/main.rs")?;
    println!("resolve_inside ok           => {}", inside.display());
    match resolve_inside(base, "../escape") {
        Ok(p) => println!("unexpected escape ok: {}", p.display()),
        Err(e) => println!("resolve_inside escape     => {e}"),
    }

    // --- is_lexically_inside / ensure_inside ---
    let child = base.join("src");
    println!(
        "is_lexically_inside          => {}",
        is_lexically_inside(&child, base)
    );
    let ensured = ensure_inside(base, base.join("src").join("lib.rs"))?;
    println!("ensure_inside               => {}", ensured.display());

    Ok(())
}
