//! Constants for the DisplayManager project.
//!
//! These constants are shared across multiple modules and centralized here
//! to avoid duplication and consistency issues.

/// Hardware settle delay in milliseconds.
///
/// Used after topology changes to give the display hardware time to
/// apply new settings. In production systems, this value can be adjusted
/// based on monitor response times.
pub const HARDWARE_SETTLE_DELAY_MS: u64 = 500;

/// Tolerance for clone detection in pixels.
///
/// Used when comparing geometries to tolerate minor differences
/// between different topology sources.
pub const CLONE_TOLERANCE_PIXELS: i32 = 4;

/// Default maximum value for numeric VCP control codes (0-100 range).
pub const VCP_HIGH_RANGE_MAX: u32 = 100;

/// Default minimum value for VCP control codes (usually 0).
pub const VCP_LOW_RANGE_MIN: u32 = 0;

/// VCP code for brightness control.
pub const VCP_CODE_BRIGHTNESS: u8 = 0x10;

/// VCP code for contrast control.
pub const VCP_CODE_CONTRAST: u8 = 0x12;

/// VCP code for input source selection.
pub const VCP_CODE_INPUT_SOURCE: u8 = 0x60;

/// VCP code for power mode control.
pub const VCP_CODE_POWER_MODE: u8 = 0xD6;

/// VCP code for volume control.
pub const VCP_CODE_VOLUME: u8 = 0x62;

/// Default resolution for automatic positioning (width in pixels).
pub const DEFAULT_DISPLAY_WIDTH: i32 = 1920;

/// Number of retry attempts for DDC/I2C operations on failure.
pub const DDC_RETRY_ATTEMPTS: u32 = 3;

/// Base delay between DDC retry attempts in milliseconds.
pub const DDC_RETRY_BASE_DELAY_MS: u64 = 50;

/// Display preset constants (VCP code 0xD6 alternative meaning).
///
/// On some monitors, VCP code 0xD6 can also be used for color presets.
/// These constants represent the various preset values.
pub mod display_presets {
    /// Standard/default color preset value.
    pub const PRESET_STANDARD: u8 = 0x00;
    /// sRGB color preset value.
    pub const PRESET_SRGB: u8 = 0x01;
    /// Native (pure) color preset value.
    pub const PRESET_NATIVE: u8 = 0x02;
    /// Cinema/film color preset value.
    pub const PRESET_CINEMA: u8 = 0x03;
    /// User-defined color preset value.
    pub const PRESET_USER: u8 = 0x04;
    /// HDR color preset value.
    pub const PRESET_HDR: u8 = 0x05;
    /// Game color preset value.
    pub const PRESET_GAME: u8 = 0x06;
}

/// VCP image scaling constants (VCP code 0x6E).
pub mod image_scaling {
    /// Fill to aspect ratio (fills the screen).
    pub const FILL_TO_ASPECT_RATIO: u8 = 0x00;
    /// Maintain aspect ratio.
    pub const ASPECT_RATIO: u8 = 0x01;
    /// Full image (shows full image without cropping).
    pub const FULL_IMAGE: u8 = 0x02;
}

/// VCP screen orientation constants (VCP code 0xFE).
pub mod screen_orientation {
    /// 0 degrees / normal orientation.
    pub const ROTATE_0: u8 = 0x00;
    /// 90 degrees (portrait).
    pub const ROTATE_90: u8 = 0x01;
    /// 180 degrees (inverted).
    pub const ROTATE_180: u8 = 0x02;
    /// 270 degrees (inverted portrait).
    pub const ROTATE_270: u8 = 0x03;
}