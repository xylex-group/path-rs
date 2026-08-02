//! User path input expansion: `~`, `%VAR%`, `$VAR`, `${VAR}`, and optional WSL translation.
//!
//! Expansion does **not** invoke a shell, interpret command substitution, or execute commands.
//! It never expands `$(...)` or backticks.

use crate::error::PathError;
use crate::internal::validation::{MAX_EXPANSION_DEPTH, reject_nul};
use crate::platform::translate_wsl_path;
use std::env;
use std::path::PathBuf;

/// Options controlling how user path input is expanded.
///
/// # Defaults
///
/// - Expand `~` at the start of the path
/// - Expand `%VAR%` and `$VAR` / `${VAR}`
/// - Reject undefined variables
/// - Trim surrounding whitespace
/// - Do **not** translate WSL `/mnt/<drive>/...` paths
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandOptions {
    /// Expand a leading `~` or `~/` / `~\` to the home directory.
    pub expand_tilde: bool,
    /// Expand `%VAR%` style variables (generic; works on any platform if set).
    pub expand_percent_variables: bool,
    /// Expand `$VAR` and `${VAR}` style variables.
    pub expand_dollar_variables: bool,
    /// Translate `/mnt/<drive>/...` to a Windows path when applicable.
    pub translate_wsl_paths: bool,
    /// When `true`, undefined variables produce an error; when `false`, they are left unchanged.
    pub reject_undefined_variables: bool,
    /// Trim leading and trailing whitespace from the input.
    pub trim_cli_input: bool,
    /// Maximum nested expansion passes (defensive limit).
    pub max_expansion_depth: u32,
}

impl Default for ExpandOptions {
    fn default() -> Self {
        Self {
            expand_tilde: true,
            expand_percent_variables: true,
            expand_dollar_variables: true,
            translate_wsl_paths: false,
            reject_undefined_variables: true,
            trim_cli_input: true,
            max_expansion_depth: MAX_EXPANSION_DEPTH,
        }
    }
}

impl ExpandOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create options with all expansion disabled.
    pub fn none() -> Self {
        Self {
            expand_tilde: false,
            expand_percent_variables: false,
            expand_dollar_variables: false,
            translate_wsl_paths: false,
            reject_undefined_variables: true,
            trim_cli_input: false,
            max_expansion_depth: MAX_EXPANSION_DEPTH,
        }
    }
}

/// Expand user path input according to `options`.
///
/// Pipeline:
/// 1. Optional trim
/// 2. Optional WSL translation (before other expansion when enabled)
/// 3. Optional tilde expansion
/// 4. Optional percent and dollar variable expansion (with depth limit)
///
/// # Filesystem access
///
/// Home directory resolution may consult environment variables / platform APIs
/// via the `dirs` crate. No other filesystem I/O is performed.
///
/// # Security
///
/// Expanded environment values are untrusted input. Do not treat expansion as
/// path validation or containment.
pub fn expand_input(input: &str, options: &ExpandOptions) -> Result<PathBuf, PathError> {
    let mut s = if options.trim_cli_input {
        input.trim().to_string()
    } else {
        input.to_string()
    };

    reject_nul(&s)?;

    if s.is_empty() {
        return Err(PathError::EmptyInput);
    }

    if options.translate_wsl_paths {
        if let Some(translated) = translate_wsl_path(&s)? {
            return Ok(translated);
        }
    }

    if options.expand_tilde {
        s = expand_tilde(&s)?;
    }

    // Multi-pass expansion with a hard depth limit (DoS protection).
    // Each pass expands percent then dollar forms once; stop when stable.
    for _ in 0..options.max_expansion_depth {
        let prev = s.clone();
        if options.expand_percent_variables {
            s = expand_percent_variables(&s, options.reject_undefined_variables)?;
        }
        if options.expand_dollar_variables {
            s = expand_dollar_variables(&s, options.reject_undefined_variables)?;
        }
        if s == prev {
            reject_nul(&s)?;
            return Ok(PathBuf::from(s));
        }
    }

    // Budget exhausted: error only if another pass would still change the value.
    let prev = s.clone();
    if options.expand_percent_variables {
        s = expand_percent_variables(&s, options.reject_undefined_variables)?;
    }
    if options.expand_dollar_variables {
        s = expand_dollar_variables(&s, options.reject_undefined_variables)?;
    }
    if s != prev {
        return Err(PathError::ExpansionDepthExceeded {
            max_depth: options.max_expansion_depth,
        });
    }

    reject_nul(&s)?;
    Ok(PathBuf::from(s))
}

/// Expand a leading `~` component to the home directory.
///
/// Only expands when `~` is the entire path or the first component (`~/...`, `~\...`).
/// Does not expand `~user` or mid-path `~/`.
///
/// # Filesystem access
///
/// Resolves the home directory via platform APIs / environment.
pub fn expand_tilde(input: &str) -> Result<String, PathError> {
    reject_nul(input)?;

    if input == "~" {
        return home_string();
    }

    // ~/ or ~\ only
    let bytes = input.as_bytes();
    if bytes.first() == Some(&b'~') && bytes.get(1).is_some_and(|b| *b == b'/' || *b == b'\\') {
        let home = home_string()?;
        let mut out = home;
        // Keep the separator from the input for readability; PathBuf will normalize later.
        out.push_str(&input[1..]);
        return Ok(out);
    }

    Ok(input.to_string())
}

/// Expand `%VAR%` style environment variables.
///
/// - `%%` becomes a literal `%`
/// - Incomplete forms like `%APPDATA` or bare `%` error when `reject_undefined` is true;
///   when false, incomplete forms are left as-is where possible
/// - Undefined variables error when `reject_undefined` is true; otherwise left unchanged
///
/// Does not recursively re-scan beyond a single left-to-right pass. Callers that need
/// multi-pass expansion should use [`expand_input`].
pub fn expand_percent_variables(input: &str, reject_undefined: bool) -> Result<String, PathError> {
    reject_nul(input)?;
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // %% -> literal %
        if i + 1 < chars.len() && chars[i + 1] == '%' {
            out.push('%');
            i += 2;
            continue;
        }

        // Find closing %
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != '%' {
            end += 1;
        }

        if end >= chars.len() {
            // Unclosed %VAR
            let fragment: String = chars[i..].iter().collect();
            if reject_undefined {
                return Err(PathError::MalformedEnvironmentVariable { input: fragment });
            }
            out.push_str(&fragment);
            break;
        }

        if end == start {
            // %% already handled; empty %% name means adjacent % after non-escape path.
            // Lone empty name %% handled above. Here `%%` at i would have been caught.
            // `% %` empty between: treat as malformed / empty var name.
            if reject_undefined {
                return Err(PathError::MalformedEnvironmentVariable { input: "%%".into() });
            }
            out.push('%');
            i = end + 1;
            continue;
        }

        let name: String = chars[start..end].iter().collect();
        if !is_valid_env_name(&name) {
            if reject_undefined {
                return Err(PathError::MalformedEnvironmentVariable {
                    input: format!("%{name}%"),
                });
            }
            out.push('%');
            out.push_str(&name);
            out.push('%');
            i = end + 1;
            continue;
        }

        match env::var(&name) {
            Ok(value) => out.push_str(&value),
            Err(env::VarError::NotPresent) => {
                if reject_undefined {
                    return Err(PathError::UndefinedEnvironmentVariable { name });
                }
                // Permissive: leave the original token unchanged.
                out.push('%');
                out.push_str(&name);
                out.push('%');
            }
            Err(env::VarError::NotUnicode(_)) => {
                return Err(PathError::invalid(format!(
                    "environment variable {name} is not valid Unicode"
                )));
            }
        }
        i = end + 1;
    }

    Ok(out)
}

/// Expand `$VAR` and `${VAR}` style environment variables.
///
/// Rules:
/// - `${VAR}` requires a closing `}`
/// - `$VAR` matches `[A-Za-z_][A-Za-z0-9_]*`
/// - `$123` is not a variable (digit-leading names are not expanded)
/// - `$(` is never expanded (command substitution is unsupported)
/// - Backticks are never expanded
///
/// Undefined variables: error when `reject_undefined` is true; otherwise leave the token.
pub fn expand_dollar_variables(input: &str, reject_undefined: bool) -> Result<String, PathError> {
    reject_nul(input)?;
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] != '$' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // $$ -> literal $
        if i + 1 < chars.len() && chars[i + 1] == '$' {
            out.push('$');
            i += 2;
            continue;
        }

        if i + 1 >= chars.len() {
            if reject_undefined {
                return Err(PathError::MalformedEnvironmentVariable { input: "$".into() });
            }
            out.push('$');
            break;
        }

        // Never expand command substitution.
        if chars[i + 1] == '(' {
            out.push('$');
            i += 1;
            continue;
        }

        // ${VAR}
        if chars[i + 1] == '{' {
            let start = i + 2;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end >= chars.len() {
                let fragment: String = chars[i..].iter().collect();
                if reject_undefined {
                    return Err(PathError::MalformedEnvironmentVariable { input: fragment });
                }
                out.push_str(&fragment);
                break;
            }
            let name: String = chars[start..end].iter().collect();
            if name.is_empty() || !is_valid_env_name(&name) {
                if reject_undefined {
                    return Err(PathError::MalformedEnvironmentVariable {
                        input: format!("${{{name}}}"),
                    });
                }
                out.push_str(&format!("${{{name}}}"));
                i = end + 1;
                continue;
            }
            match env::var(&name) {
                Ok(value) => out.push_str(&value),
                Err(env::VarError::NotPresent) => {
                    if reject_undefined {
                        return Err(PathError::UndefinedEnvironmentVariable { name });
                    }
                    out.push_str(&format!("${{{name}}}"));
                }
                Err(env::VarError::NotUnicode(_)) => {
                    return Err(PathError::invalid(format!(
                        "environment variable {name} is not valid Unicode"
                    )));
                }
            }
            i = end + 1;
            continue;
        }

        // $VAR
        if is_env_name_start(chars[i + 1]) {
            let start = i + 1;
            let mut end = start + 1;
            while end < chars.len() && is_env_name_continue(chars[end]) {
                end += 1;
            }
            let name: String = chars[start..end].iter().collect();
            match env::var(&name) {
                Ok(value) => out.push_str(&value),
                Err(env::VarError::NotPresent) => {
                    if reject_undefined {
                        return Err(PathError::UndefinedEnvironmentVariable { name });
                    }
                    out.push('$');
                    out.push_str(&name);
                }
                Err(env::VarError::NotUnicode(_)) => {
                    return Err(PathError::invalid(format!(
                        "environment variable {name} is not valid Unicode"
                    )));
                }
            }
            i = end;
            continue;
        }

        // `$` followed by something that is not a name (e.g. `$123`, `$}`).
        out.push('$');
        i += 1;
    }

    Ok(out)
}

fn home_string() -> Result<String, PathError> {
    let home = dirs::home_dir().ok_or(PathError::HomeDirectoryUnavailable)?;
    home.into_os_string()
        .into_string()
        .map_err(|_| PathError::NotUtf8)
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if is_env_name_start(c) => chars.all(is_env_name_continue),
        _ => false,
    }
}

fn is_env_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_env_name_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_only_at_start() {
        // Without a real home we only check that mid-path is unchanged.
        let s = expand_tilde("foo/~/bar").unwrap();
        assert_eq!(s, "foo/~/bar");
        let s = expand_tilde("~other/foo").unwrap();
        assert_eq!(s, "~other/foo");
    }

    #[test]
    fn percent_escape() {
        let s = expand_percent_variables("100%% done", true).unwrap();
        assert_eq!(s, "100% done");
    }

    #[test]
    fn dollar_no_command_sub() {
        let s = expand_dollar_variables("$(whoami)/x", true).unwrap();
        assert_eq!(s, "$(whoami)/x");
    }

    #[test]
    fn dollar_digit_not_var() {
        let s = expand_dollar_variables("$123", true).unwrap();
        assert_eq!(s, "$123");
    }

    #[test]
    fn unclosed_percent_strict() {
        assert!(expand_percent_variables("%APPDATA", true).is_err());
        assert!(expand_percent_variables("%", true).is_err());
    }

    #[test]
    fn unclosed_dollar_strict() {
        assert!(expand_dollar_variables("${HOME", true).is_err());
    }
}
