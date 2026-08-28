# Contributing to DisplayManager

Thank you for your interest in contributing to DisplayManager! This document provides guidelines and instructions for contributing.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/displaymanager.git
   cd displaymanager
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/my-feature
   ```

## Development Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- Platform-specific dependencies:
  - **Windows**: Visual Studio Build Tools with C++ workload
  - **Linux**: `libdrm-dev`, `libwayland-dev`, `libi2c-dev`

### Build

```bash
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Code Style

- Follow standard Rust conventions (`rustfmt` defaults)
- Use `///` doc comments on all public items
- Prefer `thiserror`/`snafu` for error types
- Use `async-trait` for async trait methods
- Keep platform-specific code behind `#[cfg(target_os = "...")]` gates

## Commit Messages

- Use clear, descriptive commit messages
- Start with a verb in imperative mood (e.g., "Add", "Fix", "Update")
- Reference issues where applicable (e.g., `Fixes #42`)

## Pull Request Process

1. Ensure all tests pass: `cargo test --workspace`
2. Ensure no lint warnings: `cargo clippy --workspace --all-targets -- -D warnings`
3. Ensure code is formatted: `cargo fmt --all -- --check`
4. Update documentation if your changes affect the public API
5. Add an entry to `CHANGELOG.md` under `[Unreleased]`
6. Submit your pull request with a clear description of the changes

## Reporting Issues

- Use the GitHub issue tracker
- Include your OS, Rust version (`rustc --version`), and steps to reproduce
- For build issues, include the full error output

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).