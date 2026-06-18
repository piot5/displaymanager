//! Error types for EDID and display management operations.
//!
//! This module defines [`EdidError`], the canonical error type for all
//! display-information and EDID operations. Each variant is carefully
//! mapped to a specific failure mode, and platform-specific types are
//! isolated behind `#[cfg]` guards to maintain cross-compilation.

use thiserror::Error;

/// Error type for EDID and display management operations.
///
/// Every public API in this crate returns `Result<T, EdidError>` so that
/// callers can handle failures via exhaustive pattern matching.
///
/// # Examples
///
/// ```rust
/// use df_displmgr_info::error::EdidError;
///
/// let err = EdidError::NotFound;
/// assert!(err.to_string().contains("not found"));
/// ```
#[derive(Debug, Error)]
pub enum EdidError {
    /// The platform-specific EDID backend is not available.
    ///
    /// This may occur when the system lacks a supported display
    /// subsystem (e.g., no compositor running on Linux, or missing
    /// CCD support on Windows).
    #[error("EDID backend not available on this system")]
    BackendNotAvailable,

    /// Communication with the display hardware failed.
    ///
    /// This includes DDC/CI I²C timeouts, bus errors, or unexpected
    /// disconnections during a read operation.
    ///
    /// # Errors
    ///
    /// Returns this variant when an I/O or protocol-level error
    /// prevents the EDID block from being retrieved.
    #[error("communication with display hardware failed")]
    CommunicationFailed,

    /// A DDC/CI error occurred.
    ///
    /// Contains a human-readable description of the underlying
    /// DDC/CI subsystem failure.
    #[error("DDC error: {0}")]
    DdcError(String),

    /// Access to the display device was denied.
    ///
    /// On Linux this typically means the process lacks `i2c` group
    /// membership or root privileges. On Windows it indicates
    /// insufficient session privileges.
    #[error("access to display device was denied")]
    AccessDenied,

    /// Failed to parse EDID data.
    ///
    /// The raw EDID block failed checksum validation or contained
    /// structural inconsistencies (e.g., truncated extension blocks,
    /// invalid descriptor tags).
    #[error("EDID data could not be parsed — checksum or structural error")]
    ParseError,

    /// The requested display or EDID data was not found.
    ///
    /// This can indicate a disconnected monitor, a disabled output,
    /// or an identifier that does not correspond to any known device.
    #[error("requested display or EDID data not found")]
    NotFound,

    /// An I/O error occurred while accessing display hardware or files.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A UTF-8 conversion error occurred when decoding string fields.
    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// A platform-specific Windows API error occurred.
    ///
    /// Only present when targeting `windows`. Maps HRESULT and
    /// NTSTATUS failures to a typed error.
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    WindowsError(#[from] windows::core::Error),
}