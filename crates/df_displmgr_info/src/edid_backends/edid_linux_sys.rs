use crate::edid_trait::EdidControl;
use crate::error::EdidError;
use std::fs;
use std::path::{Path, PathBuf};

/// Accessing EDID via /sys/class/drm/. This is the preferred Linux method.
pub struct LinuxSysfsBackend {
    pub connector_hint: Option<String>, // e.g., "card0-HDMI-A-1"
}

impl EdidControl for LinuxSysfsBackend {
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError> {
        let drm_path = Path::new("/sys/class/drm/");

        // Strategy A: Targeted probe using a known connector name.
        if let Some(ref hint) = self.connector_hint {
            let path = drm_path.join(hint).join("edid");
            if let Ok(data) = self.try_read_edid(&path) {
                return Ok(data);
            }
        }

        // Strategy B: Iterative scan through all connectors to find the first valid EDID.
        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let edid_path = entry.path().join("edid");
                if let Ok(data) = self.try_read_edid(&edid_path) {
                    return Ok(data);
                }
            }
        }

        Err(EdidError::NotFound)
    }
}

impl LinuxSysfsBackend {
    /// Validates the structure: size must be >= 128 bytes and the magic header must match.
    fn try_read_edid(&self, path: &Path) -> Result<Vec<u8>, std::io::Error> {
        let data = fs::read(path)?;

        if data.len() >= 128 && data[0..8] == [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
            Ok(data)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid EDID header",
            ))
        }
    }
}
