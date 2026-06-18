use std::fmt;
use serde::{Deserialize, Serialize};

/// Strongly-typed wrapper for a unique display output identifier.
///
/// Maps directly to operating system handle designations like GDI DeviceName,
/// CCD Target ID, or Wayland Output Name.
///
/// `Default` is derived so that structs containing `DisplayId` (e.g.
/// `WlrOutputState`) can in turn derive `Default`.
///
/// # Examples
///
/// ```rust
/// use df_displmgr::types::DisplayId;
///
/// let id = DisplayId("HDMI-1".to_string());
/// assert_eq!(id.to_string(), "HDMI-1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize, Default)]
pub struct DisplayId(pub String);

impl fmt::Display for DisplayId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly-typed wrapper for physical connector port designations
/// (e.g., `"DP-1"`, `"HDMI-A-1"`).
///
/// Connector IDs are assigned by the kernel's DRM subsystem or the Wayland
/// compositor and remain stable across reboots for a given physical port.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize, Default)]
pub struct ConnectorId(pub String);

impl fmt::Display for ConnectorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly-typed wrapper for the underlying GPU / adapter handle.
///
/// On Windows this maps to the adapter LUID; on Linux it represents the
/// DRM card path (e.g., `"/dev/dri/card0"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize, Default)]
pub struct AdapterId(pub String);

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hardware identity descriptor containing immutable hardware properties
/// and persistent session tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayIdentity {
    /// Token representing the dynamic target handle assigned by the active OS session.
    pub id: DisplayId,
    /// System designation string for the underlying physical connection port.
    pub connector_id: ConnectorId,
    /// Identifier for the specific graphics card powering this display output.
    pub adapter_id: AdapterId,
    /// Persistent EDID serial number or hardware UUID if queryable.
    pub hardware_uuid: Option<String>,
    /// Human-readable model or brand string supplied by the hardware EDID block.
    pub monitor_name: String,
}

/// Represents the physical rotation of a screen.
/// These values map directly to standard OS rotation flags.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DisplayRotation {
    /// Normal landscape orientation.
    #[default]
    Rotate0,
    /// Portrait orientation (rotated 90 degrees clockwise).
    Rotate90,
    /// Inverted landscape (rotated 180 degrees).
    Rotate180,
    /// Inverted portrait (rotated 270 degrees clockwise).
    Rotate270,
}

/// Operational status of High Dynamic Range (HDR) functionality.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HdrState {
    /// HDR is active and the monitor is receiving an HDR signal.
    Enabled,
    /// HDR is inactive; the monitor is in Standard Dynamic Range (SDR).
    #[default]
    Disabled,
}

/// Defines specific HDR metadata profiles or usage modes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HdrMode {
    /// Default system-defined HDR profile.
    #[default]
    Default,
    /// Optimized for high-contrast cinematic content.
    Cinema,
    /// Optimized for low-latency, high-brightness gaming content.
    Game,
}

/// Represents a 2D vector for display positioning within the virtual desktop
/// coordinate system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Point2D {
    /// Horizontal coordinate. Positive values extend to the right.
    pub x: i32,
    /// Vertical coordinate. Positive values extend downwards.
    pub y: i32,
}

/// Represents the physical or virtual spatial dimensions of a display target.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Extent2D {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// A combined spatial structure representing the boundary layout of an output
/// in virtual space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Rect {
    /// Top-left starting coordinate of the display.
    pub origin: Point2D,
    /// Spatial sizing dimensions of the display.
    pub size: Extent2D,
}

/// Combines physical layout properties into an explicit, hardware-supported
/// configuration mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VideoMode {
    /// Resolution dimensions of this specific mode.
    pub resolution: Extent2D,
    /// Refresh rate in millihertz (mHz). Divide by 1000 for Hz.
    pub refresh_rate: u32,
}

/// A comprehensive snapshot of a display output's current configuration.
/// Used both to report state and to stage changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputState {
    /// Complete structural identity profile for hardware mapping.
    pub identity: DisplayIdentity,
    /// Explicit spatial positioning and layout details within the virtual desktop.
    pub geometry: Rect,
    /// Refresh rate in millihertz (mHz). Divide by 1000 for Hz.
    pub refresh_rate: u32,
    /// Current screen orientation.
    pub rotation: DisplayRotation,
    /// Whether HDR is currently active.
    pub hdr_state: HdrState,
    /// The active HDR profile (if applicable).
    pub hdr_mode: HdrMode,
    /// Desktop scaling factor (e.g., 1.5 for 150%).
    pub scale: f64,
    /// Native resolution of the panel if detectable, otherwise None.
    pub native_resolution: Option<Extent2D>,
    /// List of all distinct valid hardware modes supported by this panel.
    pub supported_modes: Vec<VideoMode>,
    /// Indicates if the output is active and receiving a signal.
    pub enabled: bool,
    /// Indicates if this display is the primary desktop baseline (0,0 origin).
    pub is_primary: bool,
}

impl OutputState {
    /// Returns true if the active pixel area is wider than it is tall,
    /// accounting for rotation.
    pub fn is_landscape(&self) -> bool {
        match self.rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => {
                self.geometry.size.width >= self.geometry.size.height
            }
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => {
                self.geometry.size.height >= self.geometry.size.width
            }
        }
    }

    /// Converts the stored millihertz value to a standard Hz float.
    pub fn refresh_rate_hz(&self) -> f32 {
        self.refresh_rate as f32 / 1000.0
    }
}

/// Plan for activating a previously inactive monitor.
/// Used with `activate_with_topology_restore()`.
#[derive(Debug, Clone, Default)]
pub struct ActivationPlan {
    /// Target position (x, y). If None, auto-positions right of rightmost active monitor.
    pub position: Option<Point2D>,
    /// Target resolution (width, height). If None, keeps hardware default.
    pub resolution: Option<Extent2D>,
    /// Target rotation. If None, keeps current rotation.
    pub rotation: Option<DisplayRotation>,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            identity: DisplayIdentity {
                id: DisplayId(String::new()),
                connector_id: ConnectorId(String::new()),
                adapter_id: AdapterId(String::new()),
                hardware_uuid: None,
                monitor_name: "Unknown Display".to_string(),
            },
            geometry: Rect::default(),
            refresh_rate: 0,
            rotation: DisplayRotation::Rotate0,
            hdr_state: HdrState::Disabled,
            hdr_mode: HdrMode::Default,
            scale: 1.0,
            native_resolution: None,
            supported_modes: Vec::new(),
            enabled: false,
            is_primary: false,
        }
    }
}

