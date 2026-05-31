// edid_linux_ddc.rs
use crate::edid_trait::EdidControl;
use crate::error::EdidError;
use std::process::Command;
use std::{thread, time::Duration};

pub struct LinuxDdcBackend {
    pub bus_id: Option<u32>,
}

impl EdidControl for LinuxDdcBackend {
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError> {
        // If no bus ID is provided, attempt to scan for the first available bus using ddcutil
        let target_bus = match self.bus_id {
            Some(id) => id.to_string(),
            None => self.find_first_ddc_bus()?,
        };

        // DDC is prone to failure: 3 attempts with increasing delay
        for attempt in 0..3 {
            if attempt > 0 { thread::sleep(Duration::from_millis(200 * attempt)); }

            let output = Command::new("ddcutil")
                .args(["-b", &target_bus, "getedid", "--hex", "--noverify"])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let raw = self.parse_hex(&String::from_utf8_lossy(&out.stdout));
                    if raw.len() >= 128 { return Ok(raw); }
                }
            }
        }
        
        Err(EdidError::CommunicationFailed)
    }
}

impl LinuxDdcBackend {
    fn find_first_ddc_bus(&self) -> Result<String, EdidError> {
        let out = Command::new("ddcutil").arg("detect").output().map_err(|_| EdidError::BackendNotAvailable)?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        
        // Search for pattern "I2C bus: /dev/i2c-X"
        for line in stdout.lines() {
            if line.contains("I2C bus:") {
                if let Some(bus) = line.split('-').last() {
                    return Ok(bus.trim().to_string());
                }
            }
        }
        Err(EdidError::NotFound)
    }

    fn parse_hex(&self, hex_str: &str) -> Vec<u8> {
        let mut raw = Vec::new();
        // Extract hex characters and ignore metadata/headers in ddcutil output
        for word in hex_str.split_whitespace() {
            if word.len() == 2 && word.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(byte) = u8::from_str_radix(word, 16) {
                    raw.push(byte);
                }
            }
        }
        raw
    }
}