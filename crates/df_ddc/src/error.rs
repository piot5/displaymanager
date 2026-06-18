//! Error types for DDC/CI monitor control operations.
//!
//! This module defines [`DdcError`], the canonical error type for all
//! DDC/CI (VESA MCCS) Virtual Control Panel operations. Each variant maps
//! to a specific failure mode ranging from backend unavailability to
//! hardware communication failures.
//!
//! # Examples
//!
//! ```rust
//! use df_ddc::error::DdcError;
//!
//! let err = DdcError::AccessDenied;
//! assert!(err.to_string().contains("permissions"));
//! ```
//!
//! Every public-facing DDC operation returns `Result<T, DdcError>` to
//! allow callers to handle failures via exhaustive pattern matching.

use thiserror::Error;

/// Error type for all DDC/CI (VESA MCCS) monitor control operations.
///
/// This enum replaces raw string-based errors with typed variants,
/// enabling precise error handling without string inspection.
///
/// # Variants
///
/// * [`BackendNotAvailable`](DdcError::BackendNotAvailable) — The DDC/CI
///   backend is not present on the current platform or device.
/// * [`CommunicationFailed`](DdcError::CommunicationFailed) — A hardware
///   I²C or Win32 API call failed.
/// * [`AccessDenied`](DdcError::AccessDenied) — The process lacks the
///   necessary permissions (e.g., `i2c` group membership on Linux).
/// * [`UnsupportedFeature`](DdcError::UnsupportedFeature) — The monitor
///   does not support the requested VCP code.
/// * [`InvalidDevice`](DdcError::InvalidDevice) — The device path or
///   handle does not correspond to a valid DDC/CI-capable monitor.
#[derive(Debug, Error)]
pub enum DdcError {
    /// The DDC/CI backend is not available on this platform or device.
    ///
    /// This can occur when no DDC/CI-capable monitors are connected, or
    /// when the underlying platform does not support DDC/CI communication.
    ///
    /// # Errors
    ///
    /// Returned when the initialisation or detection of a DDC/CI backend
    /// fails.
    #[error("DDC/CI backend not available: {details}")]
    BackendNotAvailable {
        /// Human-readable detail about why the backend is unavailable.
        details: String,
    },

    /// Hardware communication over DDC/CI failed.
    ///
    /// This includes I²C bus timeouts, Win32 `GetVCPFeatureAndVCPFeatureReply`
    /// failures, or unexpected disconnections during a read or write operation.
    ///
    /// # Errors
    ///
    /// Returned when an I/O or protocol-level error prevents DDC/CI
    /// communication.
    #[error("DDC/CI hardware communication failed: {reason}")]
    CommunicationFailed {
        /// Machine-readable description of the communication failure.
        reason: String,
    },

    /// The process lacks the necessary permissions for DDC/CI access.
    ///
    /// On Linux this typically means the process is not a member of the
    /// `i2c` group or does not have root privileges. On Windows it
    /// indicates insufficient session rights.
    ///
    /// # Errors
    ///
    /// Returned when an `EACCES` / `ERROR_ACCESS_DENIED` error is
    /// encountered during device open or I/O.
    #[error("access denied: check I2C group permissions or admin rights")]
    AccessDenied,

    /// The requested VCP feature is not supported by the monitor.
    ///
    /// Monitors may only implement a subset of the VESA MCCS VCP codes.
    /// Attempting to read or write an unsupported code will return
    /// this error.
    ///
    /// # Errors
    ///
    /// Returned when the monitor returns a "not supported" response
    /// for the requested VCP feature.
    #[error("the requested VCP feature is not supported by this monitor")]
    UnsupportedFeature,

    /// The DDC/CI device at the given path or handle is invalid.
    ///
    /// This can occur if the device has been disconnected since
    /// enumeration, or if the path does not refer to a DDC/CI-capable
    /// monitor.
    ///
    /// # Errors
    ///
    /// Returned when a device open or I/O operation fails because the
    /// device identifier is invalid.
    #[error("invalid DDC/CI device at {path}")]
    InvalidDevice {
        /// The filesystem path or handle identifier of the invalid device.
        path: String,
    },
}