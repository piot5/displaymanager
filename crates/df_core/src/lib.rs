//! Shared core types for the DisplayManager workspace.
//!
//! This crate provides unified type definitions for monitor capabilities,
//! VCP codes, and other display-related enumerations.

#![deny(unsafe_op_in_unsafe_fn)]

use serde::{Deserialize, Serialize};

/// Central constants for the workspace (timing, VCP codes, etc.).
pub mod constants;

/// GTF / CVT timing calculation helpers.
pub mod timing;

/// Unified error type for the entire DisplayManager ecosystem.
///
/// This enum consolidates all error variants from DDC/CI operations, display
/// management, and EDID parsing into a single, exhaustive type. Callers can
/// match on precise failure modes without string inspection.
///
/// # Variant Categories
///
/// ## DDC/CI Errors
/// - [`DisplayManagerError::BackendNotAvailable`] - DDC/CI backend not present
/// - [`DisplayManagerError::CommunicationFailed`] - I²C or Win32 API failure
/// - [`DisplayManagerError::AccessDenied`] - Insufficient permissions
/// - [`DisplayManagerError::InvalidDevice`] - Invalid DDC device path/handle
///
/// ## Display Management Errors
/// - [`DisplayManagerError::ConnectionFailed`] - Graphics subsystem unreachable
/// - [`DisplayManagerError::MonitorNotFound`] - Output missing from topology
/// - [`DisplayManagerError::OutputDisabled`] - Display is currently disabled
/// - [`DisplayManagerError::HdrError`] - HDR operation failure
///
/// ## Configuration Errors
/// - [`DisplayManagerError::ConfigurationRejected`] - HW rejected settings
/// - [`DisplayManagerError::InvalidVcpValue`] - Invalid VCP code or value
///
/// ## Feature Support Errors
/// - [`DisplayManagerError::UnsupportedFeature`] - Feature not implemented in backend
/// - [`DisplayManagerError::UnsupportedHardware`] - HW lacks capability
/// - [`DisplayManagerError::UnsupportedPlatform`] - Feature impossible on OS
///
/// ## System Errors
/// - [`DisplayManagerError::PlatformError`] - Generic platform-specific error
/// - [`DisplayManagerError::BackendError`] - Raw platform error codes
/// - [`DisplayManagerError::PermissionDenied`] - Missing privileges
/// - [`DisplayManagerError::Timeout`] - Operation timed out
/// - [`DisplayManagerError::Serialization`] - Parse/encode failure
/// - [`DisplayManagerError::EdidParse`] - EDID parsing error
///
/// # Examples
///
/// ```rust
/// use df_core::DisplayManagerError;
///
/// let err = DisplayManagerError::MonitorNotFound("HDMI-1".into());
/// assert!(err.to_string().contains("not found"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum DisplayManagerError {
    // =========================================================================
    // DDC/CI Errors
    // =========================================================================
    /// The DDC/CI backend is not available on this platform or device.
    #[error("DDC/CI backend not available: {details}")]
    BackendNotAvailable {
        /// Human-readable detail about why the backend is unavailable.
        details: String,
    },

    /// Hardware communication over DDC/CI failed.
    #[error("DDC/CI hardware communication failed: {reason}")]
    CommunicationFailed {
        /// Machine-readable description of the communication failure.
        reason: String,
    },

    /// The process lacks the necessary permissions for DDC/CI access.
    #[error("access denied: check I2C group permissions or admin rights")]
    AccessDenied,

    /// The DDC/CI device at the given path or handle is invalid.
    #[error("invalid DDC/CI device at {path}")]
    InvalidDevice {
        /// The filesystem path or handle identifier of the invalid device.
        path: String,
    },

    // =========================================================================
    // Display Management Errors
    // =========================================================================
    /// The connection to the graphics subsystem (Wayland, Win32 session, or
    /// DRM device) could not be established.
    #[error("failed to connect to the graphics subsystem")]
    ConnectionFailed,

    /// Monitor not found or not responding.
    #[error("Monitor not found: {0}")]
    MonitorNotFound(String),

    /// The requested display output is currently disabled and cannot be
    /// queried or modified without first being activated.
    #[error("display '{0}' is currently disabled")]
    OutputDisabled(String),

    /// An error specific to High Dynamic Range metadata, tone-mapping, or
    /// state switching.
    #[error("HDR error: {0}")]
    HdrError(String),

    // =========================================================================
    // Configuration Errors
    // =========================================================================
    /// Invalid VCP code or value.
    #[error("Invalid VCP value: code={code:#04X}, value={value}")]
    InvalidVcpValue { code: u8, value: u32 },

    /// Display configuration was rejected by the operating system.
    #[error("display configuration rejected")]
    ConfigurationRejected,

    // =========================================================================
    // Feature Support Errors
    // =========================================================================
    /// Hardware does not support the requested feature.
    #[error("hardware does not support: {0}")]
    UnsupportedHardware(String),

    /// Operation not supported on this platform.
    #[error("operation not supported on this platform: {0}")]
    UnsupportedPlatform(String),

    /// Feature not implemented in this backend.
    #[error("feature not supported by this backend: {0}")]
    UnsupportedFeature(String),

    // =========================================================================
    // System Errors
    // =========================================================================
    /// Generic platform-specific error.
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Wrapper for raw platform error codes (Win32 HRESULT, DRM errno, etc.).
    #[error("platform-specific backend error: {0}")]
    BackendError(String),

    /// The requested operation requires elevated privileges.
    #[error("insufficient privileges for this operation")]
    PermissionDenied,

    /// Topology is stale and needs to be re-acquired.
    #[error("topology stale - re-acquire")]
    StaleTopology,

    /// Operation timed out.
    #[error("timeout after {timeout_ms} ms")]
    Timeout { timeout_ms: u64 },

    /// A serialization or deserialization error occurred.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Error parsing EDID data.
    #[error("EDID parsing error: {0}")]
    EdidParse(String),

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// =============================================================================
// Backward Compatibility Type Aliases
// =============================================================================

/// Legacy type alias for backward compatibility.
/// New code should use [`DisplayManagerError`] directly.
#[deprecated(since = "0.2.0", note = "Use DisplayManagerError directly")]
pub type DdcError = DisplayManagerError;

/// Legacy type alias for backward compatibility.
/// New code should use [`DisplayManagerError`] directly.
#[deprecated(since = "0.2.0", note = "Use DisplayManagerError directly")]
pub type DisplayError = DisplayManagerError;

/// Brightness and contrast capabilities reported by a monitor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorCapabilities {
    /// Current brightness value (VCP code 0x10).
    #[serde(default = "default_brightness")]
    pub brightness: u32,
    /// Minimum brightness value supported by the monitor.
    #[serde(default)]
    pub brightness_min: u32,
    /// Maximum brightness value supported by the monitor.
    #[serde(default = "default_brightness_max")]
    pub brightness_max: u32,
    /// Current contrast value (VCP code 0x12).
    #[serde(default = "default_contrast")]
    pub contrast: u32,
    /// Minimum contrast value supported by the monitor.
    #[serde(default)]
    pub contrast_min: u32,
    /// Maximum contrast value supported by the monitor.
    #[serde(default = "default_contrast_max")]
    pub contrast_max: u32,
}

/// Default brightness value (50% is typical).
fn default_brightness() -> u32 {
    50
}

/// Default brightness max value.
fn default_brightness_max() -> u32 {
    100
}

/// Default contrast value (50% is typical).
fn default_contrast() -> u32 {
    50
}

/// Default contrast max value.
fn default_contrast_max() -> u32 {
    100
}

impl Default for MonitorCapabilities {
    fn default() -> Self {
        MonitorCapabilities {
            brightness: 50,
            brightness_min: 0,
            brightness_max: 100,
            contrast: 50,
            contrast_min: 0,
            contrast_max: 100,
        }
    }
}

/// Monitor power state as defined by VCP code 0xD6 (VESA MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PowerState {
    /// Monitor is powered on and displaying an image.
    #[default]
    On = 0x01,
    /// Monitor is in standby mode.
    Standby = 0x02,
    /// Monitor is in suspend mode.
    Suspend = 0x03,
    /// Monitor is in soft-off mode (VCP code D6h).
    Off = 0x04,
    /// Power state is unknown.
    Unknown = 0x00,
}

/// Monitor input source as defined by VCP code 0x60 (VESA MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSource {
    /// Unknown input source.
    Unknown = 0x00,
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
}

/// VESA MCCS VCP (Virtual Control Panel) code constants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpCode {
    /// Active control status (VCP code 0x0E) - Read-only.
    ActiveControl = 0x0E,
    /// Active0 Focus (VCP code 0x0F) - Read-only.
    /// Indicates which input has focus for Active0 settings.
    /// Note: Value 0x0F is also used by InputSource::DisplayPort1, but context determines meaning:
    /// - For VCP code 0x0F: This is Active0Focus (focus status)
    /// - For VCP code 0x60 with value 0x0F: This indicates DisplayPort1 input
    Active0Focus = 0x0F,
    /// Brightness control (VCP code 0x10).
    Brightness = 0x10,
    /// Contrast control (VCP code 0x12).
    Contrast = 0x12,
    /// Active0 Hue (VCP code 0x14) - Color adjustment.
    Active0Hue = 0x14,
    /// Active0 Saturation (VCP code 0x15) - Color adjustment.
    Active0Saturation = 0x15,
    /// Red color gain (VCP code 0x16).
    RedGain = 0x16,
    /// Green color gain (VCP code 0x18).
    GreenGain = 0x18,
    /// Blue color gain (VCP code 0x1A).
    BlueGain = 0x1A,
    /// Clock adjustment (VCP code 0x3E).
    Clock = 0x3E,
    /// Input source selection (VCP code 0x60).
    InputSource = 0x60,
    /// Volume control (VCP code 0x62).
    Volume = 0x62,
    /// Audio mute (VCP code 0x8D).
    AudioMute = 0x8D,
    /// Sharpness control (VCP code 0xD9).
    Sharpness = 0xD9,
    /// Horizontal frequency (VCP code 0xAC).
    HorizontalFrequency = 0xAC,
    /// Vertical frequency (VCP code 0xAE).
    VerticalFrequency = 0xAE,
    /// Operating hours (VCP code 0xC0).
    OperatingHours = 0xC0,
    /// OSD language (VCP code 0xCC).
    OsdLanguage = 0xCC,
    /// Flat panel type (VCP code 0xB6).
    FlatPanelType = 0xB6,
    /// Power mode control (VCP code 0xD6).
    /// Note: ColorPreset also uses 0xD6 in some contexts - context determines meaning.
    PowerMode = 0xD6,
}

impl VcpCode {
    /// Creates a VcpCode from a raw u8 value.
    /// Returns None for undefined/reserved codes.
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0x0E => Some(VcpCode::ActiveControl),
            0x0F => Some(VcpCode::Active0Focus),
            0x10 => Some(VcpCode::Brightness),
            0x12 => Some(VcpCode::Contrast),
            0x14 => Some(VcpCode::Active0Hue),
            0x15 => Some(VcpCode::Active0Saturation),
            0x16 => Some(VcpCode::RedGain),
            0x18 => Some(VcpCode::GreenGain),
            0x1A => Some(VcpCode::BlueGain),
            0x3E => Some(VcpCode::Clock),
            0x60 => Some(VcpCode::InputSource),
            0x62 => Some(VcpCode::Volume),
            0x8D => Some(VcpCode::AudioMute),
            0xAC => Some(VcpCode::HorizontalFrequency),
            0xAE => Some(VcpCode::VerticalFrequency),
            0xB6 => Some(VcpCode::FlatPanelType),
            0xC0 => Some(VcpCode::OperatingHours),
            0xCC => Some(VcpCode::OsdLanguage),
            0xD6 => Some(VcpCode::PowerMode),
            0xD9 => Some(VcpCode::Sharpness),
            _ => None,
        }
    }

    /// Converts this VcpCode to its raw u8 representation.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Returns the maximum allowed value for this VCP code.
    /// Based on VESA MCCS Standard ranges.
    pub fn max_value(&self) -> u32 {
        match self {
            VcpCode::Brightness
            | VcpCode::Contrast
            | VcpCode::RedGain
            | VcpCode::GreenGain
            | VcpCode::BlueGain
            | VcpCode::Volume
            | VcpCode::Active0Hue
            | VcpCode::Active0Saturation
            | VcpCode::Clock
            | VcpCode::Sharpness => 100,
            VcpCode::AudioMute => 1,
            _ => 255, // Read-only codes use their native value
        }
    }

    /// Returns the minimum allowed value for this VCP code.
    pub fn min_value(&self) -> u32 {
        0 // Most controls have min=0
    }

    /// Returns true if this VCP code is read-only.
    /// Read-only codes: HorizontalFrequency, VerticalFrequency, FlatPanelType, OperatingHours, ActiveControl, Active0Focus
    pub fn is_readonly(&self) -> bool {
        matches!(
            self,
            VcpCode::HorizontalFrequency
                | VcpCode::VerticalFrequency
                | VcpCode::FlatPanelType
                | VcpCode::OperatingHours
                | VcpCode::ActiveControl
                | VcpCode::Active0Focus
        )
    }
}

impl InputSource {
    /// Creates an InputSource from a raw u8 VCP value (0x60 code).
    pub fn from_vcp_value(value: u8) -> Self {
        match value {
            0x01 => InputSource::AnalogVga,
            0x03 => InputSource::Dvi,
            0x05 => InputSource::Composite,
            0x06 => InputSource::SVideo,
            0x0F => InputSource::DisplayPort1,
            0x10 => InputSource::DisplayPort2,
            0x11 => InputSource::Hdmi1,
            0x12 => InputSource::Hdmi2,
            0x13 => InputSource::UsbC,
            _ => InputSource::Unknown,
        }
    }

    /// Converts this InputSource to its VCP value for code 0x60.
    pub fn to_vcp_value(&self) -> u8 {
        match self {
            InputSource::Unknown => 0x00,
            InputSource::AnalogVga => 0x01,
            InputSource::Dvi => 0x03,
            InputSource::Composite => 0x05,
            InputSource::SVideo => 0x06,
            InputSource::DisplayPort1 => 0x0F,
            InputSource::DisplayPort2 => 0x10,
            InputSource::Hdmi1 => 0x11,
            InputSource::Hdmi2 => 0x12,
            InputSource::UsbC => 0x13,
        }
    }
}

impl PowerState {
    /// Creates a PowerState from a raw u8 VCP value (0xD6 code).
    pub fn from_vcp_value(value: u8) -> Self {
        match value {
            0x01 => PowerState::On,
            0x02 => PowerState::Standby,
            0x03 => PowerState::Suspend,
            0x04 => PowerState::Off,
            _ => PowerState::Unknown,
        }
    }

    /// Converts this PowerState to its VCP value for code 0xD6.
    pub fn to_vcp_value(&self) -> u8 {
        match self {
            PowerState::On => 0x01,
            PowerState::Standby => 0x02,
            PowerState::Suspend => 0x03,
            PowerState::Off => 0x04,
            PowerState::Unknown => 0x00,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_manager_error_variants() {
        // DDC/CI errors
        let backend_err = DisplayManagerError::BackendNotAvailable {
            details: "no I2C".into(),
        };
        assert!(backend_err.to_string().contains("backend not available"));

        let comm_err = DisplayManagerError::CommunicationFailed {
            reason: "timeout".into(),
        };
        assert!(comm_err.to_string().contains("communication failed"));

        let access_err = DisplayManagerError::AccessDenied;
        assert!(access_err.to_string().contains("access denied"));

        let invalid_dev = DisplayManagerError::InvalidDevice {
            path: "/dev/i2c-3".into(),
        };
        assert!(invalid_dev.to_string().contains("invalid DDC/CI device"));

        // Display management errors
        let conn_err = DisplayManagerError::ConnectionFailed;
        assert!(conn_err.to_string().contains("connect"));

        let not_found = DisplayManagerError::MonitorNotFound("HDMI-1".into());
        assert!(not_found.to_string().contains("not found"));

        let disabled = DisplayManagerError::OutputDisabled("DP-1".into());
        assert!(disabled.to_string().contains("disabled"));

        let hdr_err = DisplayManagerError::HdrError("invalid metadata".into());
        assert!(hdr_err.to_string().contains("HDR error"));

        // Configuration errors
        let vcp_err = DisplayManagerError::InvalidVcpValue {
            code: 0x10,
            value: 999,
        };
        assert!(vcp_err.to_string().contains("0x10"));

        let rejected = DisplayManagerError::ConfigurationRejected;
        assert!(rejected.to_string().contains("rejected"));

        // Feature support errors
        let hw_err = DisplayManagerError::UnsupportedHardware("SDR only".into());
        assert!(hw_err.to_string().contains("hardware does not support"));

        let os_err = DisplayManagerError::UnsupportedPlatform("Linux".into());
        assert!(os_err.to_string().contains("operation not supported"));

        let feature_err = DisplayManagerError::UnsupportedFeature("HDR".into());
        assert!(feature_err.to_string().contains("feature not supported"));

        // System errors
        let platform_err = DisplayManagerError::PlatformError("Win32 error".into());
        assert!(platform_err.to_string().contains("Platform error"));

        let backend_err = DisplayManagerError::BackendError("0x80070005".into());
        assert!(backend_err.to_string().contains("backend error"));

        let perm_err = DisplayManagerError::PermissionDenied;
        assert!(perm_err.to_string().contains("privileges"));

        let stale = DisplayManagerError::StaleTopology;
        assert!(stale.to_string().contains("stale"));

        let timeout = DisplayManagerError::Timeout { timeout_ms: 5000 };
        assert!(timeout.to_string().contains("5000"));

        let serial_err = DisplayManagerError::Serialization("JSON failed".into());
        assert!(serial_err.to_string().contains("serialization"));

        let edid_err = DisplayManagerError::EdidParse("checksum failed".into());
        assert!(edid_err.to_string().contains("EDID parsing"));
    }

    #[test]
    fn test_display_manager_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let dm_err: DisplayManagerError = io_err.into();
        assert!(dm_err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_ddc_error_conversion() {
        use super::*;

        // Test conversion from DdcError variants
        let backend_err = DisplayManagerError::BackendNotAvailable {
            details: "test".into(),
        };
        let dm_err: DisplayManagerError = backend_err;
        assert!(matches!(
            dm_err,
            DisplayManagerError::BackendNotAvailable { .. }
        ));

        let comm_err = DisplayManagerError::CommunicationFailed {
            reason: "timeout".into(),
        };
        let dm_err: DisplayManagerError = comm_err;
        assert!(matches!(
            dm_err,
            DisplayManagerError::CommunicationFailed { .. }
        ));

        let access_err = DisplayManagerError::AccessDenied;
        let dm_err: DisplayManagerError = access_err;
        assert!(matches!(dm_err, DisplayManagerError::AccessDenied));
    }

    #[test]
    fn test_display_error_conversion() {
        use super::*;

        // Test conversion from DisplayError variants
        let conn_err = DisplayManagerError::ConnectionFailed;
        let dm_err: DisplayManagerError = conn_err;
        assert!(matches!(dm_err, DisplayManagerError::ConnectionFailed));

        let not_found = DisplayManagerError::MonitorNotFound("test".to_string());
        let dm_err: DisplayManagerError = not_found;
        assert!(matches!(dm_err, DisplayManagerError::MonitorNotFound(_)));

        let timeout = DisplayManagerError::Timeout { timeout_ms: 1000 };
        let dm_err: DisplayManagerError = timeout;
        assert!(matches!(dm_err, DisplayManagerError::Timeout { .. }));
    }

    #[test]
    fn test_vcp_code_values() {
        assert_eq!(VcpCode::Brightness.to_u8(), 0x10);
        assert_eq!(VcpCode::Contrast.to_u8(), 0x12);
        assert_eq!(VcpCode::Volume.to_u8(), 0x62);
        assert_eq!(VcpCode::PowerMode.to_u8(), 0xD6);
        assert_eq!(VcpCode::InputSource.to_u8(), 0x60);
    }

    #[test]
    fn test_vcp_code_from_u8() {
        assert_eq!(VcpCode::from_u8(0x10), Some(VcpCode::Brightness));
        assert_eq!(VcpCode::from_u8(0x12), Some(VcpCode::Contrast));
        assert_eq!(VcpCode::from_u8(0xFF), None);
    }

    #[test]
    fn test_vcp_code_max_value() {
        assert_eq!(VcpCode::Brightness.max_value(), 100);
        assert_eq!(VcpCode::Contrast.max_value(), 100);
        assert_eq!(VcpCode::Volume.max_value(), 100);
        assert_eq!(VcpCode::AudioMute.max_value(), 1);
        assert_eq!(VcpCode::HorizontalFrequency.max_value(), 255);
    }

    #[test]
    fn test_vcp_code_min_value() {
        assert_eq!(VcpCode::Brightness.min_value(), 0);
        assert_eq!(VcpCode::Contrast.min_value(), 0);
    }

    #[test]
    fn test_vcp_code_readonly() {
        assert!(VcpCode::HorizontalFrequency.is_readonly());
        assert!(VcpCode::VerticalFrequency.is_readonly());
        assert!(VcpCode::FlatPanelType.is_readonly());
        assert!(VcpCode::OperatingHours.is_readonly());
        assert!(!VcpCode::Brightness.is_readonly());
        assert!(!VcpCode::Contrast.is_readonly());
    }

    #[test]
    fn test_input_source_variants() {
        assert_eq!(InputSource::AnalogVga as u8, 0x01);
        assert_eq!(InputSource::Dvi as u8, 0x03);
        assert_eq!(InputSource::Hdmi1 as u8, 0x11);
        assert_eq!(InputSource::DisplayPort1 as u8, 0x0F);
        assert_eq!(InputSource::UsbC as u8, 0x13);
    }

    #[test]
    fn test_input_source_conversion() {
        assert_eq!(InputSource::from_vcp_value(0x11), InputSource::Hdmi1);
        assert_eq!(InputSource::from_vcp_value(0x0F), InputSource::DisplayPort1);
        assert_eq!(InputSource::from_vcp_value(0x13), InputSource::UsbC);
        assert_eq!(InputSource::from_vcp_value(0xFF), InputSource::Unknown);
        assert_eq!(InputSource::Unknown.to_vcp_value(), 0x00);
        assert_eq!(InputSource::Hdmi1.to_vcp_value(), 0x11);
        assert_eq!(InputSource::UsbC.to_vcp_value(), 0x13);
    }

    #[test]
    fn test_power_state_variants() {
        assert_eq!(PowerState::On as u8, 0x01);
        assert_eq!(PowerState::Standby as u8, 0x02);
        assert_eq!(PowerState::Suspend as u8, 0x03);
        assert_eq!(PowerState::Off as u8, 0x04);
        assert_eq!(PowerState::Unknown as u8, 0x00);
    }

    #[test]
    fn test_power_state_conversion() {
        assert_eq!(PowerState::from_vcp_value(0x01), PowerState::On);
        assert_eq!(PowerState::from_vcp_value(0x04), PowerState::Off);
        assert_eq!(PowerState::from_vcp_value(0xFF), PowerState::Unknown);
        assert_eq!(PowerState::On.to_vcp_value(), 0x01);
        assert_eq!(PowerState::Off.to_vcp_value(), 0x04);
        assert_eq!(PowerState::Unknown.to_vcp_value(), 0x00);
    }

    // -------------------------------------------------------------------------
    // Extended VCP Code tests (ARCH-008)
    // -------------------------------------------------------------------------

    // Test disambiguation: 0x0F is Active0Focus for VCP code 0x0F,
    // but also represents DisplayPort1 for InputSource value 0x0F
    // Both have the same numeric value (0x0F), but context determines meaning!
    #[test]
    fn test_vcp_code_0x0f_disambiguation() {
        // VCP code 0x0F maps to Active0Focus (read-only focus status)
        assert_eq!(VcpCode::from_u8(0x0F), Some(VcpCode::Active0Focus));
        // InputSource value 0x0F maps to DisplayPort1 (input selection)
        assert_eq!(InputSource::from_vcp_value(0x0F), InputSource::DisplayPort1);
        // Both have the same numeric value - this is the ambiguity point
        // Context determines meaning:
        // - VCP code 0x0F with value = Active0Focus (focus status)
        // - VCP code 0x60 with value 0x0F = DisplayPort1 input
        assert_eq!(VcpCode::Active0Focus.to_u8(), 0x0F);
        assert_eq!(InputSource::DisplayPort1 as u8, 0x0F);
        // Verify Active0Focus is read-only (focus status)
        assert!(VcpCode::Active0Focus.is_readonly());
    }

    #[test]
    fn test_vcp_code_all_values_0_to_255() {
        // Test boundary values for various VCP codes
        assert_eq!(VcpCode::Brightness.min_value(), 0);
        assert_eq!(VcpCode::Brightness.max_value(), 100);

        assert_eq!(VcpCode::Contrast.min_value(), 0);
        assert_eq!(VcpCode::Contrast.max_value(), 100);

        assert_eq!(VcpCode::AudioMute.min_value(), 0);
        assert_eq!(VcpCode::AudioMute.max_value(), 1);
    }
}
