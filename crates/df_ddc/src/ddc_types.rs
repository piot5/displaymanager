use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorCapabilities {
    pub brightness: u32,
    pub brightness_max: u32,
    pub contrast: u32,
    pub contrast_max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On = 0x01,
    Off = 0x04, // "D6" VCP Soft-Off
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    DisplayPort1 = 0x0F,
    DisplayPort2 = 0x10,
    Hdmi1 = 0x11,
    Hdmi2 = 0x12,
}

#[derive(Debug, Clone, Copy)]
pub enum VcpCode {
    Brightness = 0x10,
    Contrast = 0x12,
    InputSource = 0x60,
    Volume = 0x62,
    RedGain = 0x16,
    GreenGain = 0x18,
    BlueGain = 0x1A,
    PowerMode = 0xD6,
}