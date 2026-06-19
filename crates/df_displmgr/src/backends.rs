//! Platform-specific backend implementations.
//!
//! This module acts as the internal router for platform-specific implementations.
//! It uses conditional compilation (`#[cfg]`) to ensure only the relevant code
//! for the target operating system is compiled.
//!
//! # Architecture
//!
//! Each platform backend provides a concrete type implementing
//! [`UniversalTopology`](crate::traits::UniversalTopology) and
//! [`OutputEditable`](crate::traits::OutputEditable). The [`NativeTopology`]
//! type alias re-exports the canonical backend for the current platform,
//! allowing the rest of the crate to remain platform-agnostic.
//!
//! | Platform | Module | `NativeTopology` alias |
//! |----------|--------|------------------------|
//! | Windows  | [`windows`] | [`WinDisplayManager`] |
//! | Linux    | [`linux`] | [`LinuxTopology`] (auto-selected) |
//!
//! ## Backend Selection (Linux)
//!
//! On Linux, [`LinuxTopology`] automatically selects the best available backend
//! at runtime based on environment detection:
//!
//! 1. `WAYLAND_DISPLAY` set → Wayland/wlroots backend
//! 2. `KDE_FULL_SESSION` set → KDE Plasma/KScreen backend
//! 3. `/sys/class/drm/` exists → udev/sysfs backend
//! 4. Fallback → DRM/KMS backend
//!
//! You can also force a specific backend using [`LinuxTopology::with_backend()`]
//! or [`LinuxBackendVariant`].
//!
//! ## Shared Utilities
//!
//! The [`overlap`] module provides a canonical geometric overlap detection
//! implementation used by all backend validators.
//!
//! ## Unsupported Platforms
//!
//! The [`unsupported`] module provides a stub implementation for platforms
//! other than Windows and Linux, enabling documentation builds and
//! cross-compilation.
//!
//! ## Safety
//!
//! Platform-specific backends use `unsafe` for FFI (Win32 CCD/GDI calls on
//! Windows, DRM ioctl on Linux, Wayland protocol calls on Linux). Every
//! `unsafe` block is annotated with a SAFETY comment explaining the
//! invariants that must hold.

// ---------------------------------------------------------------------------
// Windows backends
// ---------------------------------------------------------------------------
// SAFETY: The Windows backend uses `unsafe` for Win32 FFI calls (CCD/GDI).
// All unsafe blocks are audited and documented with safety comments.
/// Windows display management backends (CCD + GDI).
///
/// Provides the [`WinDisplayManager`] struct that routes requests between
/// the modern CCD (Connecting and Configuring Displays) API and the legacy
/// GDI (Graphics Device Interface) fallback. This module is only compiled
/// on `target_os = "windows"`.
///
/// # Safety
///
/// Uses `unsafe` for Win32 FFI calls to `SetDisplayConfig`,
/// `QueryDisplayConfig`, `ChangeDisplaySettingsExW`, and other GDI/CCD
/// functions. All unsafe blocks are audited with safety comments.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::WinDisplayManager as NativeTopology;

// ---------------------------------------------------------------------------
// Linux backends
// ---------------------------------------------------------------------------
// SAFETY: The Linux DRM backend uses `unsafe` for kernel ioctl calls.
// All unsafe blocks are audited and documented with safety comments.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::LinuxTopology as NativeTopology;

// ---------------------------------------------------------------------------
// Unsupported platforms — allows documentation builds and cross-compilation
// ---------------------------------------------------------------------------
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub mod unsupported;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub use self::unsupported::StubTopology as NativeTopology;

/// Shared geometric overlap detection for display topology validation.
pub mod overlap;

#[allow(unused_imports)]
use crate::traits::{OutputEditable, UniversalTopology};
