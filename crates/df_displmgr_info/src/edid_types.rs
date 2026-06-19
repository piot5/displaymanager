//! EDID and DDC/CI data types for display information and hardware telemetry.

use serde::{Deserialize, Serialize};

/// Brightness and contrast range of the monitor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorCapabilities {
    /// Current brightness value.
    pub brightness: u32,
    /// Maximum brightness value.
    pub brightness_max: u32,
    /// Current contrast value.
    pub contrast: u32,
    /// Maximum contrast value.
    pub contrast_max: u32,
}

/// Power state reported through VCP code 0xD6.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PowerState {
    /// Monitor is powered on.
    On = 0x01,
    /// Monitor is in standby mode.
    Standby = 0x02,
    /// Monitor is in suspend mode.
    Suspend = 0x03,
    /// Monitor is powered off.
    Off = 0x04,
    /// Power state is unknown.
    Unknown = 0x00,
}

/// Position, size and rotation of an active display output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTopology {
    /// Horizontal position in pixels.
    pub x: i32,
    /// Vertical position in pixels.
    pub y: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Display rotation (e.g. "0", "90", "180", "270").
    pub rotation: String,
}

/// Input source as defined by VESA MCCS.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputSource {
    /// Analog VGA input.
    AnalogVga = 0x01,
    /// DVI input.
    Dvi = 0x03,
    /// Composite video input.
    Composite = 0x05,
    /// S-Video input.
    SVideo = 0x06,
    /// HDMI input 1.
    Hdmi1 = 0x11,
    /// HDMI input 2.
    Hdmi2 = 0x12,
    /// DisplayPort input 1.
    DisplayPort1 = 0x0F,
    /// DisplayPort input 2.
    DisplayPort2 = 0x10,
    /// USB-C input.
    UsbC = 0x13,
    /// Unknown input source.
    Unknown = 0x00,
}

/// Mute state of the display's internal audio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioMuteState {
    /// Audio output is muted.
    Muted = 0x01,
    /// Audio output is unmuted.
    Unmuted = 0x02,
    /// Audio mute state is unknown.
    Unknown = 0x00,
}

/// Digital input classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DigitalInterfaceType {
    /// HDMI interface.
    Hdmi,
    /// DisplayPort interface.
    DisplayPort,
    /// DVI interface.
    Dvi,
    /// Unknown interface.
    Unknown,
}

/// Video interface details extracted from the EDID base block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoInterfaceInfo {
    /// Analog video interface.
    Analog {
        /// Signal level in volts.
        signal_level_v: f32,
        /// Whether setup is expected.
        setup_expected: bool,
    },
    /// Digital video interface.
    Digital {
        /// Color bit depth (e.g. 6, 8).
        bit_depth: u8,
        /// Type of digital interface.
        interface_type: DigitalInterfaceType,
    },
    /// Unknown video interface.
    Unknown,
}

/// A display mode reported by the monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorMode {
    /// Horizontal resolution in pixels.
    pub width: u32,
    /// Vertical resolution in pixels.
    pub height: u32,
    /// Refresh rate in Hz.
    pub refresh_rate: u32,
    /// Whether the mode is interlaced.
    pub interlaced: bool,
}

/// Chromaticity coordinates from the EDID base block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaticityCoordinates {
    /// Red primary x coordinate.
    pub red_x: f32,
    /// Red primary y coordinate.
    pub red_y: f32,
    /// Green primary x coordinate.
    pub green_x: f32,
    /// Green primary y coordinate.
    pub green_y: f32,
    /// Blue primary x coordinate.
    pub blue_x: f32,
    /// Blue primary y coordinate.
    pub blue_y: f32,
    /// White point x coordinate.
    pub white_x: f32,
    /// White point y coordinate.
    pub white_y: f32,
}

/// HDR static metadata parsed from CEA-861-G extension blocks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HdrMetadata {
    /// Whether SDR EOTF (electro-optical transfer function) is supported.
    pub supports_sdr_eotf: bool,
    /// Whether traditional HDR is supported.
    pub supports_hdr_traditional: bool,
    /// Whether SMPTE ST 2084 (HDR10) is supported.
    pub supports_smpte_st2084: bool,
    /// Whether Hybrid Log-Gamma is supported.
    pub supports_hlg: bool,
    /// Maximum luminance in cd/m².
    pub max_luminance_cd_m2: Option<f32>,
    /// Maximum frame-average luminance in cd/m².
    pub max_frame_average_luminance_cd_m2: Option<f32>,
    /// Minimum luminance in cd/m².
    pub min_luminance_cd_m2: Option<f32>,
}

/// Audio capabilities reported through CEA short audio descriptors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioCapabilities {
    /// Number of additional audio descriptors found.
    pub extra_audio_descriptors_count: usize,
    /// Human-readable short audio descriptor strings.
    pub short_audio_descriptors: Vec<String>,
}

/// Structured hardware metadata compiled from the EDID base and extension blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdidData {
    /// Monitor model name.
    pub model_name: String,
    /// Manufacturer ID (3-letter code).
    pub manufacturer_id: String,
    /// Manufacturer product code.
    pub product_code: u16,
    /// Serial number as binary value from EDID.
    pub serial_number_binary: u32,
    /// Serial number as ASCII string (if present in EDID).
    pub serial_number_ascii: Option<String>,
    /// Week of manufacture (1-54).
    pub week_of_manufacture: u8,
    /// Year of manufacture.
    pub year_of_manufacture: i32,
    /// Video interface type.
    pub video_interface: VideoInterfaceInfo,
    /// Chromaticity coordinates (if available).
    pub chromaticity: Option<ChromaticityCoordinates>,
    /// Number of EDID extension blocks.
    pub extension_blocks: u8,
    /// Supported display modes.
    pub modes: Vec<MonitorMode>,
    /// HDR capabilities.
    pub hdr_caps: HdrMetadata,
    /// Audio capabilities.
    pub audio_caps: AudioCapabilities,
}

/// DDC/CI telemetry for a single display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDdcStats {
    /// Brightness and contrast capabilities.
    pub core_caps: MonitorCapabilities,
    /// Current input source.
    pub input_source: InputSource,
    /// Current power state.
    pub power_state: PowerState,
    /// Volume level (current, max).
    pub volume: Option<(u32, u32)>,
    /// Audio mute state.
    pub audio_mute: AudioMuteState,
    /// RGB color gains (red, green, blue).
    pub color_gains: Option<(u32, u32, u32)>,
    /// Horizontal scan frequency in Hz.
    pub horizontal_freq_hz: Option<u32>,
    /// Vertical frequency in centihertz (divide by 100 for Hz).
    pub vertical_freq_centihz: Option<u32>,
    /// Total operating hours.
    pub operating_hours: Option<u32>,
    /// OSD language code.
    pub osd_language_code: Option<u32>,
    /// Flat panel type code.
    pub panel_type_code: Option<u32>,
}

/// VESA MCCS VCP code constants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpCode {
    /// Brightness (VCP 0x10).
    Brightness = 0x10,
    /// Contrast (VCP 0x12).
    Contrast = 0x12,
    /// Red color gain (VCP 0x16).
    RedGain = 0x16,
    /// Green color gain (VCP 0x18).
    GreenGain = 0x18,
    /// Blue color gain (VCP 0x1A).
    BlueGain = 0x1A,
    /// Volume (VCP 0x62).
    Volume = 0x62,
    /// Audio mute (VCP 0x8D).
    AudioMute = 0x8D,
    /// Input source selection (VCP 0x60).
    InputSource = 0x60,
    /// Power mode (VCP 0xD6).
    PowerMode = 0xD6,
    /// Horizontal frequency (VCP 0xAC).
    HorizontalFrequency = 0xAC,
    /// Vertical frequency (VCP 0xAE).
    VerticalFrequency = 0xAE,
    /// Operating hours (VCP 0xC0).
    OperatingHours = 0xC0,
    /// OSD language (VCP 0xCC).
    OsdLanguage = 0xCC,
    /// Flat panel type (VCP 0xB6).
    FlatPanelType = 0xB6,
}
