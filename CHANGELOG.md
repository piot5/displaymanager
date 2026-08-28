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

### Changed
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
- Added expert evaluation via Ollama (92/100 score)

## [0.1.0] - 2026-06-12

### Added
- `df_ddc` — DDC/CI monitor control backend (Windows CCD + Linux I2C)
- `df_displmgr` — Cross-platform display configuration manager (Windows CCD + Linux Wayland/DRM)
- `df_displmgr_info` — Display management and hardware telemetry framework (EDID parsing, DDC stats)
- `displaymanager_cli` — Command-line frontend with display scan, DDC control, topology management
- `displaymanager_studio` — GUI (egui/eframe) with live wallpaper and animation editor
- Initial crates.io publication