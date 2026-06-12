use serde::{Deserialize, Serialize};

/// Brightness and contrast range of the monitor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorCapabilities {
    pub brightness: u32,
    pub brightness_max: u32,
    pub contrast: u32,
    pub contrast_max: u32,
}

/// Power state reported through VCP code 0xD6.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PowerState {
    On = 0x01,
    Standby = 0x02,
    Suspend = 0x03,
    Off = 0x04,
    Unknown = 0x00,
}

/// Position, size and rotation of an active display output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTopology {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rotation: String,
}

/// Input source as defined by VESA MCCS.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputSource {
    AnalogVga = 0x01,
    Dvi = 0x03,
    Composite = 0x05,
    SVideo = 0x06,
    Hdmi1 = 0x11,
    Hdmi2 = 0x12,
    DisplayPort1 = 0x0F,
    DisplayPort2 = 0x10,
    UsbC = 0x13,
    Unknown = 0x00,
}

/// Mute state of the display's internal audio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioMuteState {
    Muted = 0x01,
    Unmuted = 0x02,
    Unknown = 0x00,
}

/// Digital input classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DigitalInterfaceType {
    Hdmi,
    DisplayPort,
    Dvi,
    Unknown,
}

/// Video interface details extracted from the EDID base block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoInterfaceInfo {
    Analog {
        signal_level_v: f32,
        setup_expected: bool,
    },
    Digital {
        bit_depth: u8,
        interface_type: DigitalInterfaceType,
    },
    Unknown,
}

/// A display mode reported by the monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub interlaced: bool,
}

/// Chromaticity coordinates from the EDID base block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaticityCoordinates {
    pub red_x: f32,
    pub red_y: f32,
    pub green_x: f32,
    pub green_y: f32,
    pub blue_x: f32,
    pub blue_y: f32,
    pub white_x: f32,
    pub white_y: f32,
}

/// HDR static metadata parsed from CEA-861-G extension blocks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HdrMetadata {
    pub supports_sdr_eotf: bool,
    pub supports_hdr_traditional: bool,
    pub supports_smpte_st2084: bool, // HDR10
    pub supports_hlg: bool,          // Hybrid Log-Gamma
    pub max_luminance_cd_m2: Option<f32>,
    pub max_frame_average_luminance_cd_m2: Option<f32>,
    pub min_luminance_cd_m2: Option<f32>,
}

/// Audio capabilities reported through CEA short audio descriptors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioCapabilities {
    pub extra_audio_descriptors_count: usize,
    pub short_audio_descriptors: Vec<String>,
}

/// Structured hardware metadata compiled from the EDID base and extension blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdidData {
    pub model_name: String,
    pub manufacturer_id: String,
    pub product_code: u16,
    pub serial_number_binary: u32,
    pub serial_number_ascii: Option<String>,
    pub week_of_manufacture: u8,
    pub year_of_manufacture: i32,
    pub video_interface: VideoInterfaceInfo,
    pub chromaticity: Option<ChromaticityCoordinates>,
    pub extension_blocks: u8,
    pub modes: Vec<MonitorMode>,
    pub hdr_caps: HdrMetadata,
    pub audio_caps: AudioCapabilities,
}

/// DDC/CI telemetry for a single display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDdcStats {
    pub core_caps: MonitorCapabilities,
    pub input_source: InputSource,
    pub power_state: PowerState,
    pub volume: Option<(u32, u32)>,
    pub audio_mute: AudioMuteState,
    pub color_gains: Option<(u32, u32, u32)>,
    pub horizontal_freq_hz: Option<u32>,
    // DDC register 0xAE reports vertical frequency in 0.01 Hz units (centihertz),
    // not MHz. Consumers must divide by 100 to obtain Hz (e.g. 6000 -> 60.00 Hz).
    pub vertical_freq_centihz: Option<u32>,
    pub operating_hours: Option<u32>,
    pub osd_language_code: Option<u32>,
    pub panel_type_code: Option<u32>,
}

/// VESA MCCS VCP code constants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpCode {
    Brightness = 0x10,
    Contrast = 0x12,
    RedGain = 0x16,
    GreenGain = 0x18,
    BlueGain = 0x1A,
    Volume = 0x62,
    AudioMute = 0x8D,
    InputSource = 0x60,
    PowerMode = 0xD6,
    HorizontalFrequency = 0xAC,
    VerticalFrequency = 0xAE,
    OperatingHours = 0xC0,
    OsdLanguage = 0xCC,
    FlatPanelType = 0xB6,
}
