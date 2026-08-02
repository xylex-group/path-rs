//! Path component utilities for lexical operations.

use std::ffi::OsStr;
use std::path::{Component, Path, Prefix};

/// Return true if `path` begins with a Windows disk prefix without a root directory
/// (drive-relative form such as `C:foo` or bare `C:`).
pub(crate) fn is_drive_relative_path(path: &Path) -> bool {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {
                !matches!(components.next(), Some(Component::RootDir))
            }
            _ => false,
        },
        _ => false,
    }
}

/// Compare two paths component-wise without converting to strings.
pub(crate) fn starts_with_components(path: &Path, root: &Path) -> bool {
    let mut path_components = path.components();
    for root_component in root.components() {
        match path_components.next() {
            Some(pc) if component_eq(pc, root_component) => {}
            _ => return false,
        }
    }
    true
}

fn component_eq(a: Component<'_>, b: Component<'_>) -> bool {
    a.as_os_str() == b.as_os_str()
}

/// Whether the OsStr looks like a Windows reserved base name (CON, NUL, COM1, ...).
pub(crate) fn reserved_base_name(name: &OsStr) -> Option<String> {
    let s = name.to_string_lossy();
    let base = s.split(['.', ' ']).next().unwrap_or(&s);
    let upper = base.to_ascii_uppercase();
    if is_reserved_token(&upper) {
        Some(upper)
    } else {
        None
    }
}

fn is_reserved_token(upper: &str) -> bool {
    matches!(
        upper,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "CLOCK$"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
