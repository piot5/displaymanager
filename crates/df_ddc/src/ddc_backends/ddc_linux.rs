use crate::ddc_trait::DdcControl;
use crate::error::{DdcError, AccessDeniedSnafu, CommunicationFailedSnafu, BackendNotAvailableSnafu, UnsupportedFeatureSnafu};
use crate::ddc_types::MonitorCapabilities;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::Duration;
use std::sync::Mutex; // Required for thread-safe hardware access
use snafu::prelude::*;

pub struct LinuxBackend {
    pub path: String,
    // Global lock for this specific I2C bus to prevent command interleaving
    lock: Mutex<()>, 
}

impl LinuxBackend {
    const DDC_I2C_ADDR: u16 = 0x37;
    const HOST_ADDRESS: u8 = 0x51; 
    
    const WRITE_DELAY: Duration = Duration::from_millis(50); 
    const RETRY_ATTEMPTS: usize = 3;

    pub fn new(path: String) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    fn open_device(&self) -> Result<File, DdcError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => AccessDeniedSnafu.build(),
                _ => BackendNotAvailableSnafu { details: e.to_string() }.build(),
            })?;

        unsafe {
            const I2C_SLAVE: libc::c_ulong = 0x0703;
            if libc::ioctl(file.as_raw_fd(), I2C_SLAVE, Self::DDC_I2C_ADDR as libc::c_ulong) < 0 {
                return CommunicationFailedSnafu {
                    reason: format!("I2C_SLAVE ioctl failed for {}", self.path),
                }.fail();
            }
        }
        Ok(file)
    }

    #[inline(always)]
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        let mut sum: u8 = 0x6E; 
        for &b in data {
            sum = sum.wrapping_add(b);
        }
        sum
    }
}

impl DdcControl for LinuxBackend {
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
        // Critical: Ensure only one thread communicates with the monitor at a time
        let _guard = self.lock.lock().unwrap();
        
        let mut file = self.open_device()?;
        let mut request = [Self::HOST_ADDRESS, 0x82, 0x01, code, 0x00];
        request[4] = self.calculate_checksum(&request[..4]);

        for i in 0..Self::RETRY_ATTEMPTS {
            if file.write_all(&request).is_ok() {
                thread::sleep(Self::WRITE_DELAY);
                
                let mut response = [0u8; 11];
                if file.read_exact(&mut response).is_ok() {
                    if response[4] == code {
                        let max = ((response[6] as u32) << 8) | (response[7] as u32);
                        let cur = ((response[8] as u32) << 8) | (response[9] as u32);
                        return Ok((cur, max));
                    }
                }
            }
            thread::sleep(Duration::from_millis(20 * (i as u64 + 1)));
        }

        CommunicationFailedSnafu { 
            reason: format!("I2C Read timeout on {}", self.path) 
        }.fail()
    }

    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
        let _guard = self.lock.lock().unwrap();
        
        let mut file = self.open_device()?;
        let mut request = [Self::HOST_ADDRESS, 0x84, 0x03, code, (value >> 8) as u8, value as u8, 0x00];
        request[6] = self.calculate_checksum(&request[..6]);

        file.write_all(&request).map_err(|e| CommunicationFailedSnafu {
            reason: format!("I2C Write Error: {}", e),
        }.build())?;

        // Monitor needs time to process the command before the next one arrives
        thread::sleep(Self::WRITE_DELAY);
        Ok(())
    }
}