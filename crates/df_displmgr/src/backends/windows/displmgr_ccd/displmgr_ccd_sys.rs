//! CCD system-level FFI types and constants.

use std::fmt;
use windows::Win32::Devices::Display as WinDisplay;

// ---------------------------------------------------------------------------
// CCD Path Configuration Flags
// ---------------------------------------------------------------------------

/// Flag indicating that a `DISPLAYCONFIG_PATH_INFO` represents an active display path.
pub const DISPLAYCONFIG_PATH_ACTIVE: u32 = 0x00000001;

/// Sentinel value indicating an invalid mode index in a display path structure.
pub const DISPLAYCONFIG_PATH_MODE_IDX_INVALID: u32 = 0xffffffff;

// ---------------------------------------------------------------------------
// DISPLAYCONFIG_MODE_INFO Type Constants
// ---------------------------------------------------------------------------

/// `DISPLAYCONFIG_MODE_INFO` carrying a `DISPLAYCONFIG_SOURCE_MODE` union arm.
///
/// This mode describes the virtual desktop position and resolution for a source.
pub const DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE: WinDisplay::DISPLAYCONFIG_MODE_INFO_TYPE =
    WinDisplay::DISPLAYCONFIG_MODE_INFO_TYPE(1);

/// `DISPLAYCONFIG_MODE_INFO` carrying a `DISPLAYCONFIG_TARGET_MODE` union arm.
///
/// This mode describes the pixel clock, sync frequencies, and signal format
/// for a target.
pub const DISPLAYCONFIG_MODE_INFO_TYPE_TARGET: WinDisplay::DISPLAYCONFIG_MODE_INFO_TYPE =
    WinDisplay::DISPLAYCONFIG_MODE_INFO_TYPE(2);

// ---------------------------------------------------------------------------
// Device-Info Type IDs for CCD Extensions
// ---------------------------------------------------------------------------

/// `DisplayConfigSetDeviceInfo` type for toggling HDR output on a target.
///
/// Maps to `DISPLAYCONFIG_DEVICE_INFO_TYPE` value 12 in `ntddvdeo.h`.
pub const DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_TYPE =
    WinDisplay::DISPLAYCONFIG_DEVICE_INFO_TYPE(12);

/// `DisplayConfigSetDeviceInfo` type for changing the DPI scaling factor.
///
/// This is an undocumented extension used by the Windows Display Settings UI.
/// The numeric value `-3` is the de-facto constant observed across Windows 10
/// and 11.
///
/// **Important:** Uses explicit `i32` literal value directly to avoid type
/// conversion issues.
pub const DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_TYPE =
    WinDisplay::DISPLAYCONFIG_DEVICE_INFO_TYPE(-3);

// ---------------------------------------------------------------------------
// Custom Structures for Undocumented Device-Info Payloads
// ---------------------------------------------------------------------------

/// Payload structure for `DisplayConfigSetDeviceInfo` HDR state changes.
///
/// The `enabled` field must be `0` (disable) or `1` (enable). Pass this
/// structure to `DisplayConfigSetDeviceInfo` with type
/// [`DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DISPLAYCONFIG_SET_HDR_STATE {
    /// Standard CCD device info header identifying the target adapter and display.
    pub header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER,
    /// HDR state: `0` = disabled, `1` = enabled.
    pub enabled: u32,
}

/// Payload structure for `DisplayConfigSetDeviceInfo` DPI scale changes.
///
/// The `scale_factor_as_percent` field represents the target DPI percentage
/// (100 = 100%, 125 = 125%, 150 = 150%, etc.). Pass this structure to
/// `DisplayConfigSetDeviceInfo` with type
/// [`DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
    /// Standard CCD device info header identifying the target adapter and display.
    pub header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER,
    /// Target DPI scale as a percentage (e.g., 100, 125, 150).
    pub scale_factor_as_percent: i32,
}

// ---------------------------------------------------------------------------
// CcdRawData — Raw CCD Path and Mode Buffers
// ---------------------------------------------------------------------------

/// Holds the raw CCD path and mode arrays returned by `QueryDisplayConfig`.
///
/// These structures are preserved for later manipulation and validation during
/// topology commit operations.
#[derive(Clone)]
pub struct CcdRawData {
    /// Array of active display paths from the CCD subsystem.
    pub paths: Vec<WinDisplay::DISPLAYCONFIG_PATH_INFO>,
    /// Array of display modes associated with the paths.
    pub modes: Vec<WinDisplay::DISPLAYCONFIG_MODE_INFO>,
}

impl fmt::Debug for CcdRawData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CcdRawData")
            .field("paths_count", &self.paths.len())
            .field("modes_count", &self.modes.len())
            .finish()
    }
}