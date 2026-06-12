use crate::error::EdidError;
use crate::edid_types::EdidData;

/// Platform-specific EDID backend.
pub trait EdidControl: Send + Sync {
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError>;
}

/// Display handle: wraps a backend and parses the EDID.
pub struct DisplayDevice {
    pub info: String,
    pub inner: Box<dyn EdidControl>,
}

impl DisplayDevice {
    /// Fetch the raw bytes from the backend and pass them to the parser.
    pub fn fetch_edid(&self) -> Result<EdidData, EdidError> {
        let raw = self.inner.get_edid_raw()?;
        crate::edid_parser::EdidParser::parse(&raw)
    }
}
