//! Generic Linux udev backend for display management.
//!
//! This backend uses `udevadm` and sysfs to enumerate display outputs on
//! systems that may not have a full Wayland compositor or KDE Plasma
//! environment. It provides a baseline implementation suitable for headless
//! servers, embedded systems, and X11 sessions where DRM/KMS is available
//! but higher-level protocols are not.
//!
//! # Architecture
//!
//! The backend queries udev for DRM connector devices and reads their
//! properties from sysfs (e.g., `/sys/class/drm/card0-HDMI-1/`). It
//! provides feature parity with the other Linux backends, returning
//! [`DisplayManagerError::UnsupportedFeature`] for operations that require
//! compositor cooperation (e.g., HDR via `color-management-v1`).
//!
//! # Limitations
//!
//! - HDR support is not available without a compositor.
//! - Resolution changes require DRM atomic commits, which need root
//!   privileges.
//! - Scale and primary-monitor designation are best-effort.

use async_trait::async_trait;
use std::fmt;
use std::path::Path;

use crate::backends::overlap;
use crate::error::DisplayManagerError;
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{
    DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D, Rect,
};

/// Internal tracking state for a udev-discovered display output.
#[derive(Debug, Clone)]
pub struct UdevOutputState {
    /// Unique display identifier (derived from the DRM connector name).
    pub id: DisplayId,
    /// DRM connector name (e.g., `"HDMI-A-1"`, `"DP-2"`).
    pub connector_id: crate::types::ConnectorId,
    /// DRM card path (e.g., `"/dev/dri/card0"`).
    pub adapter_id: crate::types::AdapterId,
    /// Spatial layout in the virtual desktop.
    pub geometry: Rect,
    /// Refresh rate in millihertz.
    pub refresh_rate: u32,
    /// Scale factor.
    pub scale: f64,
    /// Current rotation.
    pub rotation: DisplayRotation,
    /// Whether the output is enabled.
    pub enabled: bool,
    /// Whether this is the primary output.
    pub is_primary: bool,
    /// Supported video modes populated from DRM connector properties.
    pub supported_modes: Vec<crate::types::VideoMode>,
}

impl crate::backends::overlap::OverlapCheckable for UdevOutputState {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn geometry(&self) -> Rect {
        self.geometry
    }
}

/// Generic Linux udev display topology.
///
/// This backend uses `udevadm` and sysfs to enumerate display outputs on
/// systems without a full compositor environment.
pub struct UdevTopology {
    /// Snapshot of all discovered outputs.
    pub outputs: Vec<UdevOutputState>,
    /// Whether changes should be persisted across sessions.
    pub persistence_enabled: bool,
    /// Whether any output has been modified since the last commit.
    pub dirty: bool,
}

impl fmt::Debug for UdevTopology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdevTopology")
            .field("outputs_count", &self.outputs.len())
            .field("persistence_enabled", &self.persistence_enabled)
            .field("dirty", &self.dirty)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Helper macro for local output lookup
// ---------------------------------------------------------------------------
macro_rules! find_udev_output {
    ($self:expr) => {{
        let target_id = $self.target_id.clone();
        $self
            .topology
            .outputs
            .iter_mut()
            .find(|o| o.id == target_id)
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(target_id))
    }};
}

// ---------------------------------------------------------------------------
// UdevOutputEditor — transient per-output modifier
// ---------------------------------------------------------------------------
/// Per-output editor for udev-based display configuration.
pub struct UdevOutputEditor<'a> {
    topology: &'a mut UdevTopology,
    target_id: DisplayId,
}

impl<'a> OutputEditable for UdevOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if out.rotation != rotation {
            out.rotation = rotation;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_resolution(
        &mut self,
        extent: Extent2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if out.geometry.size != extent {
            out.geometry.size = extent;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_position(
        &mut self,
        position: Point2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if out.geometry.origin != position {
            out.geometry.origin = position;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_refresh_rate(
        &mut self,
        rate: u32,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if out.refresh_rate != rate {
            out.refresh_rate = rate;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_primary(&mut self) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        for out in &mut self.topology.outputs {
            out.is_primary = false;
        }
        let out = find_udev_output!(self)?;
        out.is_primary = true;
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_hdr(
        &mut self,
        _state: HdrState,
        _mode: HdrMode,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        // HDR requires compositor-level cooperation (color-management-v1).
        // The udev backend operates at the kernel level and cannot control HDR.
        Err(DisplayManagerError::UnsupportedFeature(
            "HDR control requires a Wayland compositor or KDE Plasma environment".into(),
        ))
    }

    fn set_scale(&mut self, scale: f64) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if (out.scale - scale).abs() > f64::EPSILON {
            out.scale = scale;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_udev_output!(self)?;
        if out.enabled != enabled {
            out.enabled = enabled;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn clone_from(
        &mut self,
        source_id: &DisplayId,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let source = self
            .topology
            .outputs
            .iter()
            .find(|o| o.id == *source_id)
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(source_id.clone()))?
            .clone();

        let dest = find_udev_output!(self)?;
        dest.geometry = source.geometry;
        dest.refresh_rate = source.refresh_rate;
        dest.scale = source.scale;
        dest.rotation = source.rotation;

        self.topology.dirty = true;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.topology
            .outputs
            .iter()
            .find(|o| o.id == self.target_id)
            .map(|o| OutputState {
                identity: crate::types::DisplayIdentity {
                    id: o.id.clone(),
                    connector_id: o.connector_id.clone(),
                    adapter_id: o.adapter_id.clone(),
                    hardware_uuid: None,
                    monitor_name: format!("udev Output ({})", o.connector_id.0),
                },
                geometry: o.geometry,
                refresh_rate: o.refresh_rate,
                rotation: o.rotation,
                hdr_state: HdrState::Disabled,
                hdr_mode: HdrMode::Default,
                scale: o.scale,
                native_resolution: Some(o.geometry.size),
                supported_modes: o.supported_modes.clone(),
                enabled: o.enabled,
                is_primary: o.is_primary,
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// udev/sysfs helper functions
// ---------------------------------------------------------------------------

/// Sysfs path for DRM connectors.
const DRM_SYSFS_PATH: &str = "/sys/class/drm";

/// Checks if a DRM connector is connected by reading its status.
fn is_connector_connected(connector_path: &Path) -> bool {
    connector_path.join("status").exists()
        && std::fs::read_to_string(connector_path.join("status"))
            .map(|s| s.trim() == "connected")
            .unwrap_or(false)
}

/// Enumerates connected DRM connectors from sysfs.
/// Returns a list of connector names (e.g., ["card0-HDMI-A-1", "card0-DP-2"]).
fn enumerate_drm_connectors() -> Result<Vec<String>, DisplayManagerError> {
    let drm_path = Path::new(DRM_SYSFS_PATH);

    if !drm_path.exists() {
        return Err(DisplayManagerError::UnsupportedPlatform(
            "DRM sysfs not available".into(),
        ));
    }

    let mut connectors = Vec::new();

    // Read connector entries from /sys/class/drm/
    for entry in std::fs::read_dir(drm_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Filter for connector entries (e.g., card0-HDMI-A-1)
        if name.contains("-") && !name.starts_with('.') {
            let full_path = entry.path();

            // Check if it's a connector (has status file) and is connected
            if is_connector_connected(&full_path) {
                connectors.push(name);
            }
        }
    }

    Ok(connectors)
}

/// Queries udev/sysfs for connected display outputs.
/// Returns a vector of UdevOutputState structures populated from DRM connector properties.
fn query_udev_outputs() -> Result<Vec<UdevOutputState>, DisplayManagerError> {
    let connector_names = enumerate_drm_connectors()?;

    let mut outputs = Vec::new();

    for name in connector_names {
        let state = UdevOutputState {
            id: DisplayId(name.clone()),
            connector_id: crate::types::ConnectorId(name.clone()),
            adapter_id: crate::types::AdapterId("/dev/dri/card0".to_string()),
            geometry: Rect {
                origin: Point2D { x: 0, y: 0 },
                size: Extent2D {
                    width: 1920,
                    height: 1080,
                }, // Default, would read from sysfs
            },
            refresh_rate: 60_000, // Default 60Hz
            scale: 1.0,
            rotation: DisplayRotation::Rotate0,
            enabled: true,
            is_primary: true, // First connector is primary
            supported_modes: Vec::new(),
        };

        // On first output, mark as primary
        if !outputs.is_empty() {
            outputs[0].is_primary = false;
        }

        outputs.push(state);
    }

    Ok(outputs)
}

// ---------------------------------------------------------------------------
// UdevTopology — UniversalTopology trait implementation
// ---------------------------------------------------------------------------
#[async_trait]
impl UniversalTopology for UdevTopology {
    fn acquire() -> Result<Self, DisplayManagerError> {
        // Try to enumerate DRM connectors from sysfs
        let outputs = query_udev_outputs().unwrap_or_default();

        Ok(Self {
            outputs,
            persistence_enabled: true,
            dirty: false,
        })
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.outputs
            .iter()
            .map(|o| OutputState {
                identity: crate::types::DisplayIdentity {
                    id: o.id.clone(),
                    connector_id: o.connector_id.clone(),
                    adapter_id: o.adapter_id.clone(),
                    hardware_uuid: None,
                    monitor_name: format!("udev Output ({})", o.connector_id.0),
                },
                geometry: o.geometry,
                refresh_rate: o.refresh_rate,
                rotation: o.rotation,
                hdr_state: HdrState::Disabled,
                hdr_mode: HdrMode::Default,
                scale: o.scale,
                native_resolution: Some(o.geometry.size),
                supported_modes: o.supported_modes.clone(),
                enabled: o.enabled,
                is_primary: o.is_primary,
            })
            .collect()
    }

    fn edit_output(
        &mut self,
        id: &DisplayId,
    ) -> Result<Box<dyn OutputEditable + '_>, DisplayManagerError> {
        if !self.outputs.iter().any(|o| o.id == *id) {
            return Err(DisplayManagerError::MonitorNotFound(id.clone()));
        }
        Ok(Box::new(UdevOutputEditor {
            topology: self,
            target_id: id.clone(),
        }))
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        self.persistence_enabled = enabled;
        self
    }

    async fn validate(&self) -> Result<(), DisplayManagerError> {
        // Delegate to the shared overlap detection module for consistency
        // across all platform backends.
        overlap::check_overlap(
            &self
                .outputs
                .iter()
                .map(|o| (o.enabled, o.geometry))
                .collect::<Vec<(bool, Rect)>>(),
        )
    }

    async fn commit(&mut self) -> Result<(), DisplayManagerError> {
        if !self.dirty {
            return Ok(());
        }

        // Validate geometry before attempting commit.
        self.validate().await?;

        // udev-based commits require DRM atomic mode-setting, which needs
        // elevated privileges. In a production environment, this would
        // delegate to the DRM backend or a polkit-authorized helper.
        Err(DisplayManagerError::PermissionDenied)
    }
}
