# Contributing to path-rs

Thanks for your interest in contributing.

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo doc --all-features --no-deps
```

## Guidelines

1. Keep operations explicit — do not hide filesystem I/O behind lexical APIs.
2. Use `Path` / `PathBuf` / `OsStr` internally; never string-concatenate filesystem paths.
3. Do not follow symlinks by default.
4. Do not enable caching by default.
5. Add regression tests for bug fixes and new edge cases.
6. Document public items with rustdoc (filesystem access, symlink behavior, existence requirements).
7. Prefer small, reviewable pull requests.

## Commit messages

Use clear, imperative subjects (e.g. `fix: reject drive-relative paths in join_relative`).

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Security reports

See [SECURITY.md](SECURITY.md).
