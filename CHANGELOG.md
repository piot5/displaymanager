# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with GitHub Actions (check, test, clippy, fmt, build-release on Windows/Linux)
- 37 new unit tests for `df_displmgr` (types/serialization) and `df_displmgr_info` (EDID types/errors)
- `Default` implementation for `DebugBackend` in `df_ddc`

### Fixed
- Missing `DisplayDevice` struct in `df_ddc::ddc_trait` causing compilation error
- EDID parser: null-byte stripping for monitor descriptor strings (EDID null-padding)
- EDID parser tests: corrected byte values for video interface digital/analog test cases
- Doc-test in `df_displmgr_info` using non-existent field names
- Example `display_output_listing` using deprecated `OutputState.mode` field
- Benchmark `ddc_operations` calling methods directly on `DisplayDevice` instead of `inner`
- All Clippy warnings resolved (struct initializers, `is_none_or`, `.values()`, etc.)

### Changed
- Removed unused dependencies: `async-trait`/`log` from `df_ddc`, `clap`/`anyhow`/`log` from `df_displmgr`, `anyhow`/`log`/`tokio`/`base64` from `df_displmgr_info`

## [0.1.0] - 2026-06-12

### Added
- `df_ddc` — DDC/CI monitor control backend (Windows CCD + Linux I2C)
- `df_displmgr` — Cross-platform display configuration manager (Windows CCD + Linux Wayland/DRM)
- `df_displmgr_info` — Display management and hardware telemetry framework (EDID parsing, DDC stats)
- `displaymanager_cli` — Command-line frontend with display scan, DDC control, topology management
- `displaymanager_studio` — GUI (egui/eframe) with live wallpaper and animation editor
- Initial crates.io publication