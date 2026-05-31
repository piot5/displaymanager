use crate::error::EdidError;
use crate::edid_types::EdidData;

/// The hardware abstraction layer. Every platform-specific backend 
/// must implement this to provide access to the raw 128-byte (or larger) data.
pub trait EdidControl: Send + Sync {
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError>;
}

/// A handle representing a physical display. It wraps the dynamic backend 
/// and provides the high-level fetch-and-parse logic.
pub struct DisplayDevice {
    pub info: String,
    pub inner: Box<dyn EdidControl>,
}

impl DisplayDevice {
    /// Executes the backend's raw fetch and passes the result to the parser.
    pub fn fetch_edid(&self) -> Result<EdidData, EdidError> {
        let raw = self.inner.get_edid_raw()?;
        crate::edid_parser::EdidParser::parse(&raw)
    }
}