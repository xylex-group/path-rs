//! Text token normalization and UTF-8 path helpers.

use path_rs::{
    CaseNormalization, TextNormalizationOptions, normalize_path_token, path_to_string_lossy,
    path_to_utf8,
};
use std::path::Path;

fn main() -> Result<(), path_rs::PathError> {
    // --- normalize_path_token ---
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

    // CaseNormalization::apply
    println!(
        "UnicodeLowercase     => {}",
        CaseNormalization::UnicodeLowercase.apply("Straße")
    );

    // --- path_to_utf8 (strict) ---
    let utf8 = path_to_utf8(Path::new("src/lib.rs"))?;
    println!("path_to_utf8         => {utf8}");

    // --- path_to_string_lossy (logs / UI only — never re-use for I/O) ---
    let lossy = path_to_string_lossy(Path::new("src/lib.rs"));
    println!("path_to_string_lossy => {lossy}");

    // --- logical_path_key (requires `unicode` feature) ---
    #[cfg(feature = "unicode")]
    {
        use path_rs::logical_path_key;
        let key = logical_path_key(Path::new("café/path"));
        println!("logical_path_key     => {key}");
    }

    Ok(())
}
