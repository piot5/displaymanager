// error.rs
use thiserror::Error;
use crate::types::DisplayId;

/// Central error type for all display management operations.
/// Encapsulates backend errors, configuration rejections, and hardware limitations.
#[derive(Error, Debug)]
pub enum DisplayError {
    /// Occurs when the connection to the graphics subsystem (e.g., Wayland or Win32 session) fails.
    #[error("Failed to connect to the graphics subsystem.")]
    ConnectionFailed,

    /// Triggered when a monitor ID provided by the user does not exist in the current topology.
    #[error("Interface or monitor '{0:?}' not found.")]
    NotFound(DisplayId),

    /// The OS or hardware rejected the requested combination of resolution, rotation, or frequency.
    #[error("The system rejected the requested configuration.")]
    ConfigurationRejected,

    /// Specific errors related to HDR metadata or state switching.
    #[error("HDR Error: {0}")]
    HdrError(String),

    /// Used when a platform-specific feature is invoked on an incompatible backend.
    #[error("Feature not supported on this platform: {0}")]
    UnsupportedFeature(String),

    /// Wrapper for raw backend codes (e.g., Win32 HRESULT or errno).
    #[error("Platform-specific backend error: {0}")]
    BackendError(String),

    /// Standard IO errors, mapped for compatibility with file-based configuration tasks.
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

/// Specialized Result type for display operations.
pub type DisplayResult<T> = Result<T, DisplayError>;