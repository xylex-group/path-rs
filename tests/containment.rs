use path_rs::{PathError, ensure_inside, is_lexically_inside};
use std::path::Path;

#[test]
fn lexical_inside() {
    let root = Path::new("/repo");
    assert!(is_lexically_inside(Path::new("/repo"), root));
    assert!(is_lexically_inside(Path::new("/repo/src/main.rs"), root));
    assert!(!is_lexically_inside(Path::new("/repo2"), root));
    assert!(!is_lexically_inside(Path::new("/etc/passwd"), root));
}

#[test]
fn ensure_inside_normalizes() {
    let p = ensure_inside("/repo", "/repo/src/../Cargo.toml").unwrap();
    assert_eq!(p, std::path::PathBuf::from("/repo/Cargo.toml"));
    assert!(matches!(
        ensure_inside("/repo", "/etc/passwd"),
        Err(PathError::RootEscape { .. })
    ));
}
