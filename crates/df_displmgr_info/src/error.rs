use thiserror::Error;

#[derive(Error, Debug)]
pub enum EdidError {
    #[error("The requested EDID backend is not available")]
    BackendNotAvailable,

    #[error("Communication with the display hardware failed")]
    CommunicationFailed,

    #[error("DDC/CI operation failed: {0}")]
    DdcError(String),

    #[error("Permission denied when accessing EDID data")]
    AccessDenied,

    #[error("Failed to parse EDID: invalid header or checksum")]
    ParseError,

    #[error("No EDID data found for this device")]
    NotFound,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // Fixed: Path is std::string::FromUtf8Error
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}