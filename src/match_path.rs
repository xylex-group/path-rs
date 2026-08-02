//! Generic executable and command-line path matching (no process or shell execution).

use crate::error::PathError;
use crate::identity::{PathIdentityOptions, path_identity_key};
use crate::internal::validation::reject_nul_path;
use crate::platform::{is_verbatim, simplify_for_display};
use crate::text::CaseNormalization;
use std::path::Path;

/// Options for comparing two executable path strings/paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutableMatchOptions {
    /// Attempt to canonicalize existing paths (resolve symlinks).
    pub resolve_existing_symlinks: bool,
    /// Apply platform-default case folding when true; preserve when false.
    pub normalize_case: bool,
    /// Strip Windows verbatim prefixes for comparison when true.
    pub strip_verbatim_prefix: bool,
}

impl Default for ExecutableMatchOptions {
    fn default() -> Self {
        Self {
            resolve_existing_symlinks: false,
            normalize_case: true,
            strip_verbatim_prefix: true,
        }
    }
}

impl ExecutableMatchOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Returns true if two executable paths match under `options`.
///
/// Handles ordinary Windows paths, forward slashes, and optional verbatim prefixes.
/// Does **not** inspect running processes.
///
/// # Filesystem access
///
/// Only when `resolve_existing_symlinks` is true.
pub fn executable_paths_match(
    expected: impl AsRef<Path>,
    actual: impl AsRef<Path>,
    options: ExecutableMatchOptions,
) -> Result<bool, PathError> {
    let expected = expected.as_ref();
    let actual = actual.as_ref();
    reject_nul_path(expected)?;
    reject_nul_path(actual)?;

    let case = if options.normalize_case {
        CaseNormalization::PlatformDefault
    } else {
        CaseNormalization::Preserve
    };

    let identity = PathIdentityOptions {
        normalize_lexically: true,
        normalize_separators: true,
        case,
        resolve_existing_symlinks: options.resolve_existing_symlinks,
        strip_windows_verbatim_prefix: options.strip_verbatim_prefix,
        translate_wsl_paths: false,
    };

    let a = path_identity_key(expected, identity)?;
    let b = path_identity_key(actual, identity)?;
    Ok(a == b)
}

/// Options for detecting a path inside a command-line string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandLinePathMatchOptions {
    /// Compare with ASCII case folding when true.
    pub case_insensitive: bool,
    /// Normalize `\` and `/` before matching.
    pub normalize_separators: bool,
    /// Parse double- and single-quoted tokens.
    pub support_quotes: bool,
    /// Translate `/mnt/<drive>/...` tokens before matching.
    pub support_wsl_translation: bool,
    /// Also match the path basename as a token (intentionally fuzzy).
    pub match_basename: bool,
    /// Require path separators at token boundaries (reduces `repo` vs `repository` false positives).
    pub require_component_boundary: bool,
}

impl Default for CommandLinePathMatchOptions {
    fn default() -> Self {
        Self {
            case_insensitive: cfg!(windows),
            normalize_separators: true,
            support_quotes: true,
            support_wsl_translation: false,
            match_basename: false,
            require_component_boundary: true,
        }
    }
}

impl CommandLinePathMatchOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Returns true if `command_line` appears to reference `path`.
///
/// This is a **heuristic** matcher for tooling, not a full shell parser.
/// It does not invoke a shell or execute command-line content.
///
/// # Filesystem access
///
/// **No** (identity-key path uses lexical options only).
///
/// # Behavior
///
/// 1. Rejects empty target paths.
/// 2. Tokenizes the command line (optional quotes).
/// 3. Matches full path, separator-normalized form, and optional identity key.
/// 4. Optionally matches basename (document as fuzzy when enabled).
/// 5. Avoids pure substring matches when `require_component_boundary` is true.
pub fn command_line_contains_path(
    command_line: &str,
    path: impl AsRef<Path>,
    options: CommandLinePathMatchOptions,
) -> Result<bool, PathError> {
    let path = path.as_ref();
    reject_nul_path(path)?;
    if path.as_os_str().is_empty() {
        return Err(PathError::EmptyInput);
    }

    let path_str = path.to_string_lossy();
    if path_str.is_empty() {
        return Err(PathError::EmptyInput);
    }

    let case = if options.case_insensitive {
        CaseNormalization::AsciiLowercase
    } else {
        CaseNormalization::Preserve
    };

    let mut candidates = Vec::new();
    candidates.push(path_str.as_ref().to_owned());
    if options.normalize_separators {
        candidates.push(path_str.replace('\\', "/"));
        candidates.push(path_str.replace('/', "\\"));
    }

    if is_verbatim(path) {
        let simplified = simplify_for_display(path);
        let s = simplified.to_string_lossy().into_owned();
        candidates.push(s.clone());
        if options.normalize_separators {
            candidates.push(s.replace('\\', "/"));
        }
    }

    if options.support_wsl_translation {
        if let Ok(Some(wsl)) = crate::platform::translate_wsl_path(path_str.as_ref()) {
            let s = wsl.to_string_lossy().into_owned();
            candidates.push(s.clone());
            candidates.push(s.replace('\\', "/"));
        }
        // Also allow reverse: if path is Windows-like, skip; if path is /mnt/c/...
        // translate_wsl already handled when path is WSL form.
    }

    // Lexical identity key (no FS).
    let key = path_identity_key(
        path,
        PathIdentityOptions {
            normalize_lexically: true,
            normalize_separators: true,
            case,
            resolve_existing_symlinks: false,
            strip_windows_verbatim_prefix: true,
            translate_wsl_paths: options.support_wsl_translation,
        },
    )?;
    candidates.push(key);

    if options.match_basename {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !name.is_empty() && name != "." && name != ".." {
                candidates.push(name.to_owned());
            }
        }
    }

    let tokens = if options.support_quotes {
        tokenize_command_line(command_line)
    } else {
        command_line.split_whitespace().map(str::to_owned).collect()
    };

    for token in &tokens {
        let token_cmp = prepare_token(token, options, case);
        for cand in &candidates {
            let cand_cmp = prepare_token(cand, options, case);
            if cand_cmp.is_empty() {
                continue;
            }
            if token_equals_or_contains(&token_cmp, &cand_cmp, options) {
                return Ok(true);
            }
        }
    }

    // Also scan whole line with boundary-aware search when configured.
    let line = prepare_token(command_line, options, case);
    for cand in &candidates {
        let cand_cmp = prepare_token(cand, options, case);
        if cand_cmp.is_empty() {
            continue;
        }
        if options.require_component_boundary {
            if boundary_contains(&line, &cand_cmp) {
                return Ok(true);
            }
        } else if line.contains(&cand_cmp) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn prepare_token(s: &str, options: CommandLinePathMatchOptions, case: CaseNormalization) -> String {
    let mut t = s.trim().trim_matches('"').trim_matches('\'').to_owned();
    if options.normalize_separators {
        t = t.replace('\\', "/");
    }
    case.apply(&t)
}

fn token_equals_or_contains(
    token: &str,
    candidate: &str,
    options: CommandLinePathMatchOptions,
) -> bool {
    if token == candidate {
        return true;
    }
    if options.require_component_boundary {
        boundary_contains(token, candidate)
    } else {
        token.contains(candidate)
    }
}

/// True when `needle` appears in `haystack` at a component-ish boundary.
///
/// Boundaries are start/end of string or a path separator / whitespace / quote.
fn boundary_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(needle) {
        let idx = start + rel;
        let before_ok = idx == 0 || is_boundary_char(haystack.as_bytes()[idx - 1]);
        let end = idx + needle.len();
        let after_ok = end == haystack.len() || is_boundary_char(haystack.as_bytes()[end]);
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

fn is_boundary_char(b: u8) -> bool {
    matches!(
        b,
        b'/' | b'\\' | b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'\'' | b'=' | b':' | b',' | b';'
    )
}

/// Minimal tokenizer: whitespace-separated tokens with `"..."` and `'...'` support.
///
/// Does not implement full shell syntax (escapes are best-effort for `\"` only).
fn tokenize_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();
    let mut in_double = false;
    let mut in_single = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' if in_double => {
                if let Some(&next) = chars.peek() {
                    if next == '"' || next == '\\' {
                        cur.push(chars.next().unwrap_or(next));
                        continue;
                    }
                }
                cur.push('\\');
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            c if c.is_whitespace() && !in_double && !in_single => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            other => cur.push(other),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_slash_variants() {
        let opts = ExecutableMatchOptions::default();
        assert!(
            executable_paths_match(
                r"C:\Program Files\Tool\app.exe",
                r"c:/program files/tool/app.exe",
                opts
            )
            .unwrap()
        );
    }

    #[test]
    fn command_line_boundary_avoids_prefix_false_positive() {
        let opts = CommandLinePathMatchOptions {
            case_insensitive: true,
            require_component_boundary: true,
            match_basename: false,
            ..CommandLinePathMatchOptions::default()
        };
        // `C:\repo` must not match inside `C:\repository`.
        assert!(!command_line_contains_path(r"tool C:\repository\file", r"C:\repo", opts).unwrap());
        assert!(command_line_contains_path(r"tool C:\repo\file", r"C:\repo", opts).unwrap());
    }

    #[test]
    fn quoted_paths() {
        let opts = CommandLinePathMatchOptions {
            case_insensitive: true,
            ..CommandLinePathMatchOptions::default()
        };
        assert!(
            command_line_contains_path(
                r#"tool "C:\Users\Floris\repo" --flag"#,
                r"C:\Users\Floris\repo",
                opts
            )
            .unwrap()
        );
    }
}
