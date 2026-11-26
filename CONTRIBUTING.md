# Contributing to Repub

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## Reporting issues

- Check existing issues first.
- Use the issue templates.
- Include reproduction steps, your OS, and your Rust version.

## Submitting changes

1. Fork the repository and branch from `main`.
2. Make the change and add tests for new functionality.
3. Update documentation if behavior or design changed.
4. Confirm the checks below pass.
5. Open a pull request.

```bash
cargo fmt --check
cargo clippy --workspace
cargo test --workspace
```

## Development setup

Requires Rust 1.90 or later (`rust-toolchain.toml` selects stable with rustfmt and clippy).

```bash
git clone https://github.com/YOUR_USERNAME/repub.git
cd repub
cargo build --workspace
```

Crates live in `crates/` and are arranged in layers. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Coding style

- Follow `cargo fmt` and Clippy.
- Document public items with `///` and modules with `//!`.
- Keep comments in sync with the code.
- Put unit tests in a `tests` module in the same file. Name tests for the behavior they check, and cover edge cases and error conditions.

## Commit messages

- Use the imperative mood ("Add feature", not "Added feature").
- Keep the first line under 72 characters.
- Reference issues where applicable (`Fixes #123`).

```text
Add text frame column support

- Implement ColumnSpec for multi-column layout
- Add column gap and width customization

Fixes #42
```

## License

Contributions are licensed under the project terms (MIT OR Apache-2.0).
