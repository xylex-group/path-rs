//! Structured error type for `path-rs` operations.

use std::io;
use std::path::Path;

/// Primary error type returned by `path-rs` APIs.
///
/// Errors include enough context to diagnose the failing path or operation.
/// Third-party error types are not exposed as public fields unless necessary.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    /// The provided path input was empty after optional trimming.
    #[error("path input is empty")]
    EmptyInput,

    /// The path contains an embedded NUL byte, which is invalid on all supported platforms.
    #[error("path contains an embedded NUL byte")]
    EmbeddedNul,

    /// An environment variable referenced during expansion is not defined.
    #[error("environment variable is not defined: {name}")]
    UndefinedEnvironmentVariable {
        /// Name of the missing variable.
        name: String,
    },

    /// Environment variable syntax was malformed (e.g. unclosed `%VAR` or `${VAR`).
    #[error("environment variable syntax is malformed: {input}")]
    MalformedEnvironmentVariable {
        /// The malformed input fragment.
        input: String,
    },

    /// Home directory could not be determined.
    #[error("home directory is unavailable")]
    HomeDirectoryUnavailable,

    /// Current working directory could not be determined.
    #[error("current directory is unavailable: {source}")]
    CurrentDirectoryUnavailable {
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// A Windows drive-relative path such as `C:foo` or `C:` was encountered.
    #[error("drive-relative Windows path is not supported: {path}")]
    DriveRelativePath {
        /// Display form of the path.
        path: String,
    },

    /// An absolute child path was supplied where a relative path is required.
    #[error("absolute child path is not allowed: {path}")]
    AbsoluteChildPath {
        /// Display form of the path.
        path: String,
    },

    /// A path escapes the permitted root after normalization.
    #[error("path escapes the permitted root: {path}")]
    RootEscape {
        /// Display form of the path.
        path: String,
    },

    /// A path is not valid UTF-8 and a UTF-8 conversion was requested.
    #[error("path is not valid UTF-8")]
    NotUtf8,

    /// Generic invalid path condition with a human-readable message.
    #[error("invalid path: {message}")]
    InvalidPath {
        /// Description of the problem.
        message: String,
    },

    /// A filesystem operation failed.
    #[error("filesystem operation failed for {path}: {source}")]
    Filesystem {
        /// Display form of the path involved.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Directory traversal failed (permission, depth limit, etc.).
    #[error("directory traversal failed: {message}")]
    Traversal {
        /// Description of the failure.
        message: String,
    },

    /// A glob pattern is invalid.
    #[error("glob pattern is invalid: {message}")]
    InvalidGlob {
        /// Description of the pattern error.
        message: String,
    },

    /// A cache operation failed.
    #[error("cache operation failed: {message}")]
    Cache {
        /// Description of the failure.
        message: String,
    },

    /// Application name validation failed.
    #[error("invalid application name: {message}")]
    InvalidAppName {
        /// Description of the problem.
        message: String,
    },

    /// Expansion exceeded the maximum allowed depth.
    #[error("path expansion exceeded maximum depth ({max_depth})")]
    ExpansionDepthExceeded {
        /// Configured maximum depth.
        max_depth: u32,
    },
}

impl PathError {
    /// Create a filesystem error for `path` wrapping `source`.
    pub fn filesystem(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Filesystem {
            path: path_display(path.as_ref()),
            source,
        }
    }

    /// Create an invalid-path error with a message.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidPath {
            message: message.into(),
        }
    }

    /// Create a traversal error with a message.
    pub fn traversal(message: impl Into<String>) -> Self {
        Self::Traversal {
            message: message.into(),
        }
    }

    /// Create a cache error with a message.
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Create a root-escape error for `path`.
    pub fn root_escape(path: impl AsRef<Path>) -> Self {
        Self::RootEscape {
            path: path_display(path.as_ref()),
        }
    }

    /// Create an absolute-child error for `path`.
    pub fn absolute_child(path: impl AsRef<Path>) -> Self {
        Self::AbsoluteChildPath {
            path: path_display(path.as_ref()),
        }
    }

    /// Create a drive-relative error for `path`.
    pub fn drive_relative(path: impl AsRef<Path>) -> Self {
        Self::DriveRelativePath {
            path: path_display(path.as_ref()),
        }
    }
}

/// Lossy display string for diagnostics only (never for filesystem round-trips).
pub(crate) fn path_display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
