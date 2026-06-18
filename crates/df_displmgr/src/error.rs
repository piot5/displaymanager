//! Centralized error types for the `df_displmgr` crate.
//!
//! All display-management operations return [`DisplayError`] variants, ensuring
//! callers can match on precise failure modes without resorting to string
//! inspection or `anyhow` erasure.
//!
//! # Platform Parity
//!
//! The [`UnsupportedPlatform`](DisplayError::UnsupportedPlatform) variant
//! guarantees that the public API surface is **100% identical** across Windows
//! and Linux: if a feature is physically impossible on one platform (e.g., HDR
//! metadata over DDC on a bare-metal DRM session), it returns this typed error
//! rather than omitting the method or changing the signature.

use thiserror::Error;

use crate::types::DisplayId;

/// Central error type for all display management operations.
///
/// Every public API in this crate returns `Result<T, DisplayError>` so that
/// callers can handle failures via exhaustive pattern matching. Backend
/// implementations wrap platform-specific errors into the appropriate variant.
///
/// # Variant Summary
///
/// | Variant | Meaning |
/// |---------|---------|
/// | [`ConnectionFailed`](DisplayError::ConnectionFailed) | Subsystem unreachable |
/// | [`NotFound`](DisplayError::NotFound) | Output missing |
/// | [`ConfigurationRejected`](DisplayError::ConfigurationRejected) | HW rejected settings |
/// | [`HdrError`](DisplayError::HdrError) | HDR operation failure |
/// | [`UnsupportedFeature`](DisplayError::UnsupportedFeature) | Feature not implemented |
/// | [`UnsupportedHardware`](DisplayError::UnsupportedHardware) | HW lacks capability |
/// | [`UnsupportedPlatform`](DisplayError::UnsupportedPlatform) | Feature impossible on OS |
/// | [`BackendError`](DisplayError::BackendError) | Raw platform error |
/// | [`OutputDisabled`](DisplayError::OutputDisabled) | Output is off |
/// | [`StaleTopology`](DisplayError::StaleTopology) | Need re-acquire |
/// | [`PermissionDenied`](DisplayError::PermissionDenied) | Missing privileges |
/// | [`Timeout`](DisplayError::Timeout) | Operation timed out |
/// | [`Serialization`](DisplayError::Serialization) | Parse/encode failure |
/// | [`Io`](DisplayError::Io) | I/O subsystem error |
///
/// # Examples
///
/// ```rust
/// use df_displmgr::error::DisplayError;
///
/// let err = DisplayError::NotFound(df_displmgr::DisplayId("HDMI-1".into()));
/// assert!(err.to_string().contains("not found"));
/// ```
#[derive(Error, Debug)]
pub enum DisplayError {
    /// The connection to the graphics subsystem (Wayland, Win32 session, or
    /// DRM device) could not be established.
    ///
    /// On Windows this indicates a failed CCD session or GDI initialisation.
    /// On Linux it indicates a Wayland registry bind failure or a DRM device
    /// open error.
    #[error("failed to connect to the graphics subsystem")]
    ConnectionFailed,

    /// A monitor or output identified by [`DisplayId`] does not exist in the
    /// current topology snapshot.
    #[error("interface or monitor '{0}' not found")]
    NotFound(DisplayId),

    /// The operating system or hardware rejected the requested combination of
    /// resolution, rotation, refresh rate, or spatial layout.
    #[error("the system rejected the requested configuration")]
    ConfigurationRejected,

    /// An error specific to High Dynamic Range metadata, tone-mapping, or
    /// state switching.
    #[error("HDR error: {0}")]
    HdrError(String),

    /// A feature was invoked that is not supported on the current platform or
    /// backend implementation.
    ///
    /// For example, attempting to set [`HdrState`](crate::types::HdrState) on
    /// a Wayland compositor that does not support `color-management-v1`.
    #[error("feature not supported on this platform: {0}")]
    UnsupportedFeature(String),

    /// A feature was invoked that is not supported by the specific hardware
    /// attached to the system (e.g., HDR on an SDR-only panel, or DDC/CI on
    /// a USB dongle that only forwards EDID).
    #[error("hardware does not support requested feature: {0}")]
    UnsupportedHardware(String),

    /// The requested operation is physically impossible on the current
    /// operating system. This variant ensures **strict platform parity**:
    /// the public API is identical across Windows and Linux; backends that
    /// cannot fulfil a capability return this instead of changing the
    /// method signature.
    ///
    /// # Examples
    ///
    /// - `force_all()` (CCD wake) on Linux — Linux has no equivalent to
    ///   Windows' CCD wake-and-move paradigm.
    /// - DPI scaling on bare-metal DRM — DRM/KMS exposes no per-monitor
    ///   DPI concept; the compositor handles scaling.
    #[error("operation not supported on this platform: {0}")]
    UnsupportedPlatform(String),

    /// Wrapper for raw backend error codes such as Win32 `HRESULT`, DRM
    /// `errno`, or Wayland protocol errors.
    #[error("platform-specific backend error: {0}")]
    BackendError(String),

    /// The requested display output is currently disabled and cannot be
    /// queried or modified without first being activated.
    #[error("display '{0}' is currently disabled")]
    OutputDisabled(DisplayId),

    /// A staging or commit operation failed because the topology was not
    /// properly acquired or has been invalidated by another process.
    #[error("topology stale — re-acquire before committing")]
    StaleTopology,

    /// The requested operation requires elevated privileges that the current
    /// process does not hold.
    ///
    /// On Linux this typically means root or `video` group membership for
    /// DRM atomic commits. On Windows it means administrator privileges
    /// for certain CCD operations.
    #[error("insufficient privileges for this operation")]
    PermissionDenied,

    /// A timeout expired while waiting for the display subsystem to respond.
    #[error("operation timed out after {timeout_ms} ms")]
    Timeout {
        /// The timeout threshold in milliseconds.
        timeout_ms: u64,
    },

    /// A serialization or deserialization error occurred while reading
    /// hardware maps, EDID data, or configuration files.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Standard I/O errors, mapped for compatibility with file-based
    /// configuration tasks such as reading EDID dumps or hardware maps.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Specialized [`Result`] type for display operations.
///
/// This is the canonical return type used throughout the crate. Every public
/// API function returns `DisplayResult<T>` so that callers can operate on a
/// single, unified error type.
///
/// # Example
///
/// ```rust
/// use df_displmgr::{DisplayResult, DisplayId, NativeTopology};
/// use df_displmgr::traits::UniversalTopology;
///
/// fn example() -> DisplayResult<()> {
///     let topo = NativeTopology::acquire()?;
///     println!("Detected {} outputs", topo.get_outputs().len());
///     Ok(())
/// }
/// ```
pub type DisplayResult<T> = Result<T, DisplayError>;