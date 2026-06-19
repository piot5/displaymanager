//! Linux DDC/CI backend using raw I²C via `i2c-dev`.
//!
//! Communicates with DDC/CI-capable monitors over the I²C bus using
//! raw `ioctl` calls. Implements the VESA DDC/CI packet protocol with
//! checksum validation and retry logic.
//!
//! # Safety
//!
//! This module uses `unsafe` for `libc::ioctl` calls to set the I²C
//! slave address. These calls are required because the Linux `i2c-dev`
//! kernel interface exposes no safe userspace abstraction.
//!
//! All unsafe blocks are documented with safety comments explaining
//! why the operation is sound in context.

use crate::ddc_trait::DdcControl;
use crate::error::DdcError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// Linux DDC/CI backend using raw I²C via `i2c-dev`.
///
/// Wraps a handle to an I²C bus device (e.g., `/dev/i2c-3`) and
/// communicates with monitors using the VESA DDC/CI packet protocol.
/// A `Mutex` ensures that only one thread accesses the bus at a time,
/// preventing command interleaving.
pub struct LinuxBackend {
    /// Path to the I²C device file (e.g., `/dev/i2c-3`).
    pub path: String,
    /// Global lock for this specific I²C bus to prevent command interleaving.
    lock: Mutex<()>,
}

impl LinuxBackend {
    /// Standard DDC/CI I²C address for monitors (0x37 shifted left 1).
    const DDC_I2C_ADDR: u16 = 0x37;
    /// The DDC/CI host address byte (EDID emulation address).
    const HOST_ADDRESS: u8 = 0x51;

    /// Delay between write and read operations (milliseconds).
    const WRITE_DELAY: Duration = Duration::from_millis(50);
    /// Number of I²C read retry attempts before giving up.
    const RETRY_ATTEMPTS: usize = 3;

    /// Creates a new Linux DDC/CI backend for the given I²C device path.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path of the I²C device (e.g., `/dev/i2c-3`).
    pub fn new(path: String) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Opens the I²C device and sets the slave address for DDC/CI communication.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::AccessDenied`] if the process lacks permission
    /// to open the device. Returns [`DdcError::BackendNotAvailable`] if
    /// the device does not exist. Returns
    /// [`DdcError::CommunicationFailed`] if the `I2C_SLAVE` ioctl fails.
    fn open_device(&self) -> Result<File, DdcError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => DdcError::AccessDenied,
                _ => DdcError::BackendNotAvailable {
                    details: e.to_string(),
                },
            })?;

        // SAFETY: `libc::ioctl` with `I2C_SLAVE` is the standard way to
        // set the I²C slave address on Linux. The file descriptor is
        // valid because it was just opened above and is not yet shared.
        // This ioctl is required by the `i2c-dev` kernel interface and
        // there is no safe alternative in userspace.
        unsafe {
            const I2C_SLAVE: libc::c_ulong = 0x0703;
            if libc::ioctl(
                file.as_raw_fd(),
                I2C_SLAVE,
                Self::DDC_I2C_ADDR as libc::c_ulong,
            ) < 0
            {
                return Err(DdcError::CommunicationFailed {
                    reason: format!("I2C_SLAVE ioctl failed for {}", self.path),
                });
            }
        }
        Ok(file)
    }

    /// Calculates the DDC/CI checksum for a packet.
    ///
    /// The checksum is the 2's complement of the sum of all data bytes,
    /// such that the sum of all packet bytes (including checksum) equals
    /// 0 modulo 256.
    #[inline(always)]
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        let sum: u8 = data.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        (256u16 - sum as u16).wrapping_rem(256) as u8
    }
}

impl DdcControl for LinuxBackend {
    /// Reads a VCP feature value from the monitor over I²C.
    ///
    /// Sends a DDC/CI request packet, waits for the monitor to respond,
    /// and parses the VCP value from the response. Retries up to
    /// [`RETRY_ATTEMPTS`](Self::RETRY_ATTEMPTS) times with exponential
    /// backoff.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::CommunicationFailed`] if the I²C read times
    /// out or the monitor's response is malformed.
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
        // Ensure only one thread communicates with the monitor at a time
        let _guard = self
            .lock
            .lock()
            .map_err(|_| DdcError::CommunicationFailed {
                reason: "Mutex lock poisoned in linux backend (get_vcp_feature)".to_string(),
            })?;

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

        Err(DdcError::CommunicationFailed {
            reason: format!("I2C read timeout on {}", self.path),
        })
    }

    /// Writes a VCP feature value to the monitor over I²C.
    ///
    /// Sends a DDC/CI set-command packet and waits for the monitor to
    /// process it before returning.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::CommunicationFailed`] if the I²C write fails.
    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| DdcError::CommunicationFailed {
                reason: "Mutex lock poisoned in linux backend (set_vcp_feature)".to_string(),
            })?;

        let mut file = self.open_device()?;
        let mut request = [
            Self::HOST_ADDRESS,
            0x84,
            0x03,
            code,
            (value >> 8) as u8,
            value as u8,
            0x00,
        ];
        request[6] = self.calculate_checksum(&request[..6]);

        file.write_all(&request)
            .map_err(|e| DdcError::CommunicationFailed {
                reason: format!("I2C write error: {}", e),
            })?;

        // Monitor needs time to process the command before the next one arrives
        thread::sleep(Self::WRITE_DELAY);
        Ok(())
    }
}
