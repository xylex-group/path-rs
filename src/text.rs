//! Generic text/path-token normalization (no repository or VCS semantics).
//!
//! Callers that need Git-specific rules (e.g. stripping `.git` suffixes) must
//! compose those rules themselves on top of [`normalize_path_token`].

/// How to transform letter case in a comparison key or token.
///
/// # Platform notes
///
/// - Windows is normally case-insensitive for filesystem paths.
/// - Linux is normally case-sensitive.
/// - macOS may be either, depending on the volume.
/// - Case folding is **not** proof of filesystem identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CaseNormalization {
    /// Leave case unchanged.
    #[default]
    Preserve,
    /// ASCII A–Z → a–z only.
    AsciiLowercase,
    /// Unicode lowercase mapping (`str::to_lowercase`).
    UnicodeLowercase,
    /// Platform-oriented default: ASCII lowercase on Windows; preserve elsewhere.
    PlatformDefault,
}

impl CaseNormalization {
    /// Apply this case policy to `value`.
    pub fn apply(self, value: &str) -> String {
        match self {
            Self::Preserve => value.to_owned(),
            Self::AsciiLowercase => value.to_ascii_lowercase(),
            Self::UnicodeLowercase => value.to_lowercase(),
            Self::PlatformDefault => {
                if cfg!(windows) {
                    value.to_ascii_lowercase()
                } else {
                    value.to_owned()
                }
            }
        }
    }
}

/// Options for generic path/token string normalization.
///
/// This is intentionally free of repository, Git, or product-specific rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextNormalizationOptions {
    /// Trim leading and trailing Unicode whitespace.
    pub trim_whitespace: bool,
    /// Strip trailing `/` and `\` characters.
    pub trim_trailing_separators: bool,
    /// Rewrite `\` to `/` for separator-normalized comparison.
    pub normalize_separators: bool,
    /// Case policy for the result.
    pub case: CaseNormalization,
}

impl Default for TextNormalizationOptions {
    fn default() -> Self {
        Self {
            trim_whitespace: true,
            trim_trailing_separators: true,
            normalize_separators: true,
            case: CaseNormalization::Preserve,
        }
    }
}

impl TextNormalizationOptions {
    /// Create default options (`Self::default()`).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Normalize a path-like text token for comparison or matching.
///
/// # Filesystem access
///
/// **No.**
///
/// # Returns
///
/// A `String` suitable for comparison only — not necessarily a valid filesystem path.
///
/// # Examples
///
/// ```
/// use path_rs::{normalize_path_token, CaseNormalization, TextNormalizationOptions};
///
/// let key = normalize_path_token(
///     r"C:\Users\Floris\Repo\",
///     &TextNormalizationOptions {
///         trim_whitespace: true,
///         trim_trailing_separators: true,
///         normalize_separators: true,
///         case: CaseNormalization::AsciiLowercase,
///     },
/// );
/// assert_eq!(key, "c:/users/floris/repo");
/// ```
pub fn normalize_path_token(value: &str, options: &TextNormalizationOptions) -> String {
    let mut s = if options.trim_whitespace {
        value.trim()
    } else {
        value
    }
    .to_owned();

    if options.trim_trailing_separators {
        while s.ends_with('/') || s.ends_with('\\') {
            s.pop();
        }
    }

    if options.normalize_separators {
        s = s.replace('\\', "/");
        // Collapse repeated separators for token comparison (not for FS paths).
        while s.contains("//") {
            s = s.replace("//", "/");
        }
    }

    options.case.apply(&s)
}
