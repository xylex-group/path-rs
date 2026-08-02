# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |

## Reporting a vulnerability

Please report security issues privately via GitHub Security Advisories for
[xylex-group/path-rs](https://github.com/xylex-group/path-rs) or by contacting
the maintainers through the organization.

Do not open public issues for exploitable vulnerabilities until a fix is available.

## Important limitations (not bugs by themselves)

### Lexical normalization is not canonicalization

`normalize` does not access the filesystem and does not resolve symlinks.
`canonicalize_existing` does both and requires the path to exist.

### Lexical root containment is not symlink-safe

`resolve_inside` / `ensure_inside` / `is_lexically_inside` operate on path
components only. A lexically contained path may still escape a root after
symlink resolution. For security-sensitive containment:

1. Canonicalize the root.
2. Resolve the candidate on the filesystem.
3. Re-check that the canonical candidate is under the canonical root.
4. Prefer OS facilities (`openat`, handle-based APIs) where available.

### Cached discovery results are not authoritative

Caches improve performance only. Entries may be stale. Directory modification
timestamps are imperfect invalidation signals (resolution differences, network
filesystems, child changes that do not update parent mtime, renames).

**Never** use cache results as a security boundary. Re-validate before
destructive or privileged operations.

### TOCTOU

Paths can change between validation and use. Avoid assuming a check remains
true until a later open/write.

### Environment-variable expansion is untrusted input

Expanded values come from the process environment (or user input). Treat them
as untrusted for path policy decisions.

### Glob expansion and traversal

Large trees, deep recursion, and broad globs can exhaust memory or time.
Always set `max_depth` and `max_entries` for untrusted roots. Expansion depth
and pattern length limits exist but are not a substitute for application policy.

### Path identity keys are not filesystem identity

`path_identity_key` produces a comparison key under an explicit policy. It does
not prove two paths refer to the same inode/file ID. Case folding, separator
normalization, WSL translation, and symlink resolution each change the key's
meaning. Never use identity keys as filesystem paths.

### Command-line matching is heuristic

`command_line_contains_path` is not a full shell parser. Basename matching is
intentionally fuzzy. Prefer component-boundary matching to reduce false positives.

### Archive extraction and executable resolution

This crate does not implement archive extraction or executable PATH search
policies. Those require stronger, domain-specific validation.

### Windows reserved names and trailing dots/spaces

Windows reserved device names (`CON`, `NUL`, `COM1`, …) and names with trailing
dots/spaces can behave unexpectedly. Validate when creating files from untrusted
names.

## Defensive limits

The crate applies defensive limits including:

- `max_expansion_depth` (default 8)
- maximum glob pattern length
- optional `max_depth` / `max_entries` for listing and search
- cache `max_entries`
