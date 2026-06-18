//! Linux display management backends.
//!
//! Provides four distinct backends for Linux display management, each
//! implementing the [`UniversalTopology`] and [`OutputEditable`] traits:
//!
//! | Backend | Enum Variant | Environment | Description |
//! |---------|-------------|-------------|-------------|
//! | Wayland/wlroots | [`WlrTopology`](displmgr_wlr::WlrTopology) | Wayland compositor | Uses `zwlr-output-management-v1` |
//! | DRM/KMS | [`DrmTopology`](displmgr_drm::DrmTopology) | Bare metal / X11 | Direct kernel mode-setting |
//! | KDE Plasma | [`KdeTopology`](displmgr_kde::KdeTopology) | KDE Plasma 5+ | D-Bus via KScreen |
//! | udev/sysfs | [`UdevTopology`](displmgr_udev::UdevTopology) | Any Linux | sysfs enumeration |
//!
//! ## Backend Selection
//!
//! Use [`LinuxTopology`] to automatically select the appropriate backend at
//! runtime, or construct a specific backend directly:
//!
//! ```rust,no_run
//! use df_displmgr::traits::UniversalTopology;
//! use df_displmgr::backends::linux::{LinuxTopology, LinuxBackendVariant};
//!
//! // Auto-select (recommended):
//! let mut topo = LinuxTopology::acquire().unwrap();
//!
//! // Or force a specific backend:
//! let mut topo = LinuxTopology::with_backend(LinuxBackendVariant::Wlr).unwrap();
//! ```
//!
//! ## Safety
//!
//! The DRM and Wayland backends use `unsafe` for kernel ioctl and protocol
//! FFI calls. Every unsafe block is annotated with a SAFETY comment.
//!
//! ## Feature Parity
//!
//! All four backends provide the same [`UniversalTopology`] interface.
//! Features that require compositor cooperation (e.g., HDR via
//! `color-management-v1`) return [`DisplayError::UnsupportedFeature`]
//! on backends that cannot support them.

use async_trait::async_trait;
use std::fmt;

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{UniversalTopology, OutputEditable};
use crate::types::{OutputState, DisplayId};

/// Full Wayland/wlroots implementation using the zwlr-output-management-v1 protocol.
///
/// Contains the production-grade event-loop-based topology with
/// color-management support. This is the canonical Linux backend with
/// real Wayland connection and output enumeration.
pub mod displmgr_wlr;

/// DRM/KMS backend for X11 and bare-metal Linux environments (no compositor).
///
/// Uses Atomic KMS for flicker-free, synchronised updates. Provides direct
/// kernel-level display control without compositor intermediation. Performs
/// real hardware probing via DRM ioctls.
pub mod displmgr_drm;

/// KDE Plasma / KScreen backend for display management.
///
/// Communicates with the KDE Plasma desktop environment via D-Bus to
/// enumerate and configure display outputs.
pub mod displmgr_kde;

/// Generic Linux udev backend for display management.
///
/// Uses `udevadm` and sysfs to enumerate display outputs on systems without
/// a full compositor environment.
pub mod displmgr_udev;

// ---------------------------------------------------------------------------
// Backend selection enum
// ---------------------------------------------------------------------------

/// Identifies which Linux display backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxBackendVariant {
    /// Wayland/wlroots protocol (`zwlr-output-management-v1`).
    Wlr,
    /// Direct DRM/KMS atomic mode-setting.
    Drm,
    /// KDE Plasma D-Bus (KScreen).
    Kde,
    /// udev/sysfs enumeration.
    Udev,
}

impl LinuxBackendVariant {
    /// Detects the best available backend for the current environment.
    ///
    /// Priority order:
    /// 1. `WAYLAND_DISPLAY` set → `Wlr`
    /// 2. `KDE_FULL_SESSION` set → `Kde`
    /// 3. `/sys/class/drm/` exists → `Udev`
    /// 4. Fallback → `Drm`
    pub fn detect() -> Self {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            Self::Wlr
        } else if std::env::var("KDE_FULL_SESSION").is_ok() {
            Self::Kde
        } else if std::path::Path::new("/sys/class/drm/").exists() {
            Self::Udev
        } else {
            Self::Drm
        }
    }
}

// ---------------------------------------------------------------------------
// LinuxTopology — runtime backend selector
// ---------------------------------------------------------------------------

/// Runtime-selectable Linux display topology.
///
/// Wraps one of the four Linux backends and delegates all trait methods
/// to the active backend. Use [`LinuxTopology::acquire()`] for automatic
/// detection, or [`LinuxTopology::with_backend()`] to force a specific
/// backend.
pub enum LinuxTopology {
    /// Wayland/wlroots backend.
    Wlr(displmgr_wlr::WlrTopology),
    /// DRM/KMS backend.
    Drm(displmgr_drm::DrmTopology),
    /// KDE Plasma backend.
    Kde(displmgr_kde::KdeTopology),
    /// udev/sysfs backend.
    Udev(displmgr_udev::UdevTopology),
}

impl LinuxTopology {
    /// Acquires the topology using automatic backend detection.
    pub fn acquire() -> DisplayResult<Self> {
        let variant = LinuxBackendVariant::detect();
        Self::with_backend(variant)
    }

    /// Acquires the topology using a specific backend variant.
    pub fn with_backend(variant: LinuxBackendVariant) -> DisplayResult<Self> {
        match variant {
            LinuxBackendVariant::Wlr => {
                let inner = displmgr_wlr::WlrTopology::acquire()?;
                Ok(Self::Wlr(inner))
            }
            LinuxBackendVariant::Drm => {
                let inner = displmgr_drm::DrmTopology::acquire()?;
                Ok(Self::Drm(inner))
            }
            LinuxBackendVariant::Kde => {
                let inner = displmgr_kde::KdeTopology::acquire()?;
                Ok(Self::Kde(inner))
            }
            LinuxBackendVariant::Udev => {
                let inner = displmgr_udev::UdevTopology::acquire()?;
                Ok(Self::Udev(inner))
            }
        }
    }

    /// Returns the currently active backend variant.
    pub fn variant(&self) -> LinuxBackendVariant {
        match self {
            Self::Wlr(_) => LinuxBackendVariant::Wlr,
            Self::Drm(_) => LinuxBackendVariant::Drm,
            Self::Kde(_) => LinuxBackendVariant::Kde,
            Self::Udev(_) => LinuxBackendVariant::Udev,
        }
    }
}

impl fmt::Debug for LinuxTopology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wlr(inner) => f.debug_tuple("Wlr").field(inner).finish(),
            Self::Drm(inner) => f.debug_tuple("Drm").field(inner).finish(),
            Self::Kde(inner) => f.debug_tuple("Kde").field(inner).finish(),
            Self::Udev(inner) => f.debug_tuple("Udev").field(inner).finish(),
        }
    }
}

#[async_trait]
impl UniversalTopology for LinuxTopology {
    fn acquire() -> DisplayResult<Self> {
        Self::acquire()
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        match self {
            Self::Wlr(inner) => inner.get_outputs(),
            Self::Drm(inner) => inner.get_outputs(),
            Self::Kde(inner) => inner.get_outputs(),
            Self::Udev(inner) => inner.get_outputs(),
        }
    }

    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        match self {
            Self::Wlr(inner) => inner.edit_output(id),
            Self::Drm(inner) => inner.edit_output(id),
            Self::Kde(inner) => inner.edit_output(id),
            Self::Udev(inner) => inner.edit_output(id),
        }
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        match self {
            Self::Wlr(inner) => { inner.set_persistence(enabled); }
            Self::Drm(inner) => { inner.set_persistence(enabled); }
            Self::Kde(inner) => { inner.set_persistence(enabled); }
            Self::Udev(inner) => { inner.set_persistence(enabled); }
        }
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        match self {
            Self::Wlr(inner) => inner.validate().await,
            Self::Drm(inner) => inner.validate().await,
            Self::Kde(inner) => inner.validate().await,
            Self::Udev(inner) => inner.validate().await,
        }
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        match self {
            Self::Wlr(inner) => inner.commit().await,
            Self::Drm(inner) => inner.commit().await,
            Self::Kde(inner) => inner.commit().await,
            Self::Udev(inner) => inner.commit().await,
        }
    }
}