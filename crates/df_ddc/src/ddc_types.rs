//! DDC/CI VCP (Virtual Control Panel) data types.

use serde::{Deserialize, Serialize};

/// Brightness and contrast capabilities reported by a monitor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorCapabilities {
    /// Current brightness value (VCP code 0x10).
    pub brightness: u32,
    /// Maximum brightness value supported by the monitor.
    pub brightness_max: u32,
    /// Current contrast value (VCP code 0x12).
    pub contrast: u32,
    /// Maximum contrast value supported by the monitor.
    pub contrast_max: u32,
}

/// Monitor power state as defined by VCP code 0xD6 (VESA MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// Monitor is powered on and displaying an image.
    On = 0x01,
    /// Monitor is in soft-off mode (VCP code D6h).
    Off = 0x04,
}

/// Monitor input source as defined by VCP code 0x60 (VESA MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSource {
    /// DisplayPort input 1.
    DisplayPort1 = 0x0F,
    /// DisplayPort input 2.
    DisplayPort2 = 0x10,
    /// HDMI input 1.
    Hdmi1 = 0x11,
    /// HDMI input 2.
    Hdmi2 = 0x12,
}

/// VESA MCCS VCP (Virtual Control Panel) code constants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpCode {
    /// Brightness control (VCP code 0x10).
    Brightness = 0x10,
    /// Contrast control (VCP code 0x12).
    Contrast = 0x12,
    /// Input source selection (VCP code 0x60).
    InputSource = 0x60,
    /// Volume control (VCP code 0x62).
    Volume = 0x62,
    /// Red color gain (VCP code 0x16).
    RedGain = 0x16,
    /// Green color gain (VCP code 0x18).
    GreenGain = 0x18,
    /// Blue color gain (VCP code 0x1A).
    BlueGain = 0x1A,
    /// Power mode control (VCP code 0xD6).
    PowerMode = 0xD6,
}