use wayland_client::{protocol::wl_output, Proxy};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_manager_v1, zwlr_output_head_v1, zwlr_output_mode_v1
};

/// Represents a raw hardware head (monitor) as seen by the Wayland compositor.
pub struct WlrHeadRaw {
    pub head: zwlr_output_head_v1::ZwlrOutputHeadV1,
    pub name: String,
    pub description: String,
    pub physical_width: i32,
    pub physical_height: i32,
    pub enabled: bool,
    pub modes: Vec<WlrModeRaw>,
    pub current_mode: Option<usize>,
    pub position: (i32, i32),
    pub transform: wl_output::Transform,
    pub scale: f64,
}

/// Represents a supported hardware mode (resolution/refresh rate) for a Linux output.
pub struct WlrModeRaw {
    pub mode: zwlr_output_mode_v1::ZwlrOutputModeV1,
    pub width: i32,
    pub height: i32,
    pub refresh: i32, // in mHz
    pub preferred: bool,
}

/// Internal state for the global Wayland connection.
pub struct WlrGlobalState {
    pub manager: Option<zwlr_output_manager_v1::ZwlrOutputManagerV1>,
    pub heads: Vec<WlrHeadRaw>,
}