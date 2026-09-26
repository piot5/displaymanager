# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with GitHub Actions (check, test, clippy, fmt, build-release on Windows/Linux)
- 37 new unit tests for `df_displmgr` (types/serialization) and `df_displmgr_info` (EDID types/errors)
- `Default` implementation for `DebugBackend` in `df_ddc`
- `--format` global CLI flag for JSON output (text/json modes)
- `--dry-run` flag implementation for DDC commands with validation output
- Public accessor methods on `DisplayDevice` for encapsulated DDC operations

### Fixed
- Missing `DisplayDevice` struct in `df_ddc::ddc_trait` causing compilation error
- EDID parser: null-byte stripping for monitor descriptor strings (EDID null-padding)
- EDID parser tests: corrected byte values for video interface digital/analog test cases
- Doc-test in `df_displmgr_info` using non-existent field names
- Example `display_output_listing` using deprecated `OutputState.mode` field
- Benchmark `ddc_operations` calling methods directly on `DisplayDevice` instead of `inner`
- All Clippy warnings resolved (struct initializers, `is_none_or`, `.values()`, etc.)
- `DisplayDevice.inner` field encapsulation: made private, added delegating methods
- `info::scan()` and `info::monitor_info()` now accept optional `OutputFormat` parameter
- Removed direct `.inner` field access from all CLI code and tests
- Doctest in `df_displmgr::error` failed to compile (`E0433`: `DisplayManagerError` was not
  imported) and made `cargo test --workspace` exit non-zero — the doctests now use the
  canonical error type, so the whole workspace test run is green
- Unused imports (`windows_ffi_tests`, `ddc_types_test`, `df_displmgr::error`: `thiserror::Error`
  and `DisplayId` were imported but never used)
- An ignored `BOOL` return value in the EDID Windows backend and a redundant `unsafe` block
  in `displmgr_gdi_api` (writing a union field is a safe operation)
- CI steps that could never succeed: `ci.yml` ran `cargo +1.75.0 check` without installing
  the 1.75.0 toolchain, and installed `cargo-llvm-cov` without the `llvm-tools-preview`
  component
- `displaymanager_cli --version` reported a hardcoded `1.0` for a 0.1.1 crate; it now reports
  `CARGO_PKG_VERSION` (and the integration test asserts against `env!("CARGO_PKG_VERSION")`
  instead of a literal)
- CLI integration tests invoked the binary through `cargo run` per test, which made the suite
  slow and flaky (parallel `cargo run` processes contend on the build-directory lock —
  `test_set_unknown_monitor_returns_error` failed intermittently). They now call the
  Cargo-built binary directly via `CARGO_BIN_EXE_displaymanager_cli`; the dead `run_binary`
  helper was removed
- 17 intra-doc links that pointed at the deprecated aliases or at a non-existent
  `DisplayManagerError::NotFound` variant were corrected (rustdoc warnings)

### Changed
- **Deprecated alias migration (26. September 2026)**
  - All internal code now uses the canonical `df_core::DisplayManagerError`; the
    deprecated aliases `DdcError`, `DisplayError`, `DisplayResult`,
    `DdcErrorWithContext` and `DisplayErrorWithSource` remain exported (with
    `#[allow(deprecated)]` on the crate-root re-export) for backward compatibility
  - `DisplayResult<T>` call sites became `Result<T, DisplayManagerError>` across
    `df_core`, `df_ddc`, `df_displmgr`, `df_displmgr_info` and the CLI (375 lines,
    42 files), including 5 example/binary `main` signatures
  - Identity `.into()` calls in the error-path tests were removed (the alias and the
    canonical type are the same type, so the conversion was a no-op)
  - Result: `cargo clippy --workspace --all-targets --all-features -- -D warnings` is
    clean (was 57 errors / 293 warnings) and `cargo test --workspace` passes
    397 tests + doctests with 0 failures
- `cargo fmt --all` applied; `cargo fmt --all -- --check` now passes
- **CI/CD overhaul**: `ci.yml` split into `lint` (fmt, clippy, `cargo audit`), `test`
  (Windows + Ubuntu), `msrv` (explicit 1.75.0 install) and `coverage`
  (`llvm-tools-preview` + `cargo-llvm-cov` + Codecov) jobs. `test.yml` no longer
  duplicates the lint/coverage jobs and its broken `integration_tests` job (no such
  test target exists) was removed — it now only runs the stable/MSRV matrix and the
  weekly schedule
- **Release pipeline fixed**: every path dependency in `[workspace.dependencies]`
  declares a `version`, so `cargo package` works. `publish.yml` now publishes
  `df_core` first (`df_core` was never published, which blocked all other crates) and
  follows the dependency graph. See `CONTRIBUTING.md` for the release procedure
- `df_displmgr` dev-dependency pin `df_displmgr_info` aligned to `0.1.1`
- Removed the unreliable "92/100 score" claim from this changelog
- **Dependency Optimization (17. Juli 2026)**
  - Reduced Tokio features from `"full"` to `["rt-multi-thread", "macros", "sync", "time"]`
  - Removed unused dependencies: `proptest` and `tempfile`
  - Added feature flags for optional backends in `df_displmgr` (wayland, drm, windows)
  - Marked `zbus` as optional dependency in Linux backend
  - Updated `clap` with `derive` feature for CLI macros
  - Cleaned up dev-dependencies in multiple crates
- **Multi-GPU DRM Support (18. Juli 2026)**
  - `DrmTopology` now supports multiple GPUs with `HashMap<AdapterId, File>` for device tracking
  - Resource mapping updated to `(DisplayId, (AdapterId, DrmResourceIds))` for proper routing
  - `acquire()` uses `probe_all_drm_devices()` for comprehensive multi-GPU enumeration
  - `validate()` iterates across all detected GPU devices

### Documentation
- Updated all docs/ checklists to reflect implemented features (DRM, Wayland, DDC/CI, EDID, Windows API)
- Added comprehensive dependency optimization plan
- Added a "Release Process (crates.io)" section to `CONTRIBUTING.md` documenting the
  version-bump locations, the publish order and why `df_core` has to go first
- Replaced the unverified "expert evaluation via Ollama (92/100)" note with a measured
  code review (see `.clinerules/history.md`)

## [0.1.0] - 2026-06-12

### Added
- `df_ddc` — DDC/CI monitor control backend (Windows CCD + Linux I2C)
- `df_displmgr` — Cross-platform display configuration manager (Windows CCD + Linux Wayland/DRM)
- `df_displmgr_info` — Display management and hardware telemetry framework (EDID parsing, DDC stats)
- `displaymanager_cli` — Command-line frontend with display scan, DDC control, topology management
- `displaymanager_studio` — GUI (egui/eframe) with live wallpaper and animation editor
- Initial crates.io publication