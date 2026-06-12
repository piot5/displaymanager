# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with GitHub Actions (fmt, clippy, tests on Linux/Windows/macOS)
- LICENSE-MIT and LICENSE-APACHE files
- Crate-level documentation (`//!`) for docs.rs
- CHANGELOG.md and CONTRIBUTING.md

### Changed
- Improved README documentation with badges and examples

## [0.1.0] - 2026-06-12

### Added
- `df_ddc` — DDC/CI monitor control backend (Windows CCD + Linux I2C)
- `df_displmgr` — Cross-platform display configuration manager (Windows CCD + Linux Wayland/DRM)
- `df_displmgr_info` — Display management and hardware telemetry framework (EDID parsing, DDC stats)
- Initial crates.io publication