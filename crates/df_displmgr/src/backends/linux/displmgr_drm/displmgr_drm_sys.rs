// backends/linux/displmgr_drm_sys.rs
use crate::types::DisplayRotation;
use drm::control::{connector, crtc, plane, property};
use std::collections::HashMap;

/// Legacy DRM rotation bit-mask definitions matching kernel property expectations.
pub const DRM_MODE_ROTATE_0: u64 = 1 << 0;
pub const DRM_MODE_ROTATE_90: u64 = 1 << 1;
pub const DRM_MODE_ROTATE_180: u64 = 1 << 2;
pub const DRM_MODE_ROTATE_270: u64 = 1 << 3;

/// Maps core framework abstraction rotations to raw Linux DRM atomic property values.
pub fn rotation_to_drm_value(rotation: DisplayRotation) -> u64 {
    match rotation {
        DisplayRotation::Rotate0 => DRM_MODE_ROTATE_0,
        DisplayRotation::Rotate90 => DRM_MODE_ROTATE_90,
        DisplayRotation::Rotate180 => DRM_MODE_ROTATE_180,
        DisplayRotation::Rotate270 => DRM_MODE_ROTATE_270,
    }
}

/// Tracking map containing the underlying kernel object handles needed for atomic configuration injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrmResourceIds {
    pub connector_id: connector::Handle,
    pub crtc_id: crtc::Handle,
    pub primary_plane_id: plane::Handle,
}

/// Local register storage containing queryable property identifiers across active hardware resources.
#[derive(Debug, Clone, Default)]
pub struct DrmPropertyCache {
    pub props: HashMap<connector::Handle, HashMap<String, property::Handle>>,
}
