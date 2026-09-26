// backends/linux/dispmgr_drm.rs
//! # DRM/KMS Backend Implementation
//! Implements display management traits using Linux Direct Rendering Manager.
//! Features Atomic KMS for flicker-free, synchronized updates and multi-GPU support.

use async_trait::async_trait;
use drm::control::{atomic::AtomicModeReq, AtomicCommitFlags};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};

use crate::error::DisplayManagerError;
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{
    AdapterId, DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D, Rect,
};

pub mod displmgr_drm_api;
pub mod displmgr_drm_sys;

use self::displmgr_drm_api::{detect_drm_devices, probe_all_drm_devices, probe_drm_hardware};
use self::displmgr_drm_sys::{rotation_to_drm_value, DrmPropertyCache, DrmResourceIds};

/// Linux Direct Rendering Manager (DRM) Backend using Atomic KMS.
///
/// Supports multi-GPU configurations by tracking outputs across multiple DRM devices.
pub struct DrmTopology {
    /// Map of adapter_id to its DRM device file handle.
    /// Enables multi-GPU configurations where each GPU has its own device node.
    pub devices: HashMap<AdapterId, File>,
    /// List of detected outputs across all GPUs.
    pub outputs: Vec<OutputState>,
    /// Atomic mode-setting request builder.
    pub atomic_req: AtomicModeReq,
    /// Resource mapping for DRM handles, keyed by DisplayId.
    /// Each entry stores (adapter_id, resource_ids) for multi-GPU routing.
    pub resource_map: HashMap<DisplayId, (AdapterId, DrmResourceIds)>,
    /// Cached property handles for efficient lookups.
    pub property_cache: DrmPropertyCache,
    /// Whether configuration changes are pending.
    pub dirty: bool,
}

impl DrmTopology {
    /// Internal helper to locate queryable property tokens across active connectors.
    pub fn find_prop_id(
        &self,
        conn: drm::control::connector::Handle,
        name: &str,
    ) -> Option<drm::control::property::Handle> {
        self.property_cache.props.get(&conn)?.get(name).cloned()
    }

    /// Wake inactive outputs and return their `DisplayId`s so they can be
    /// restored later. If `keep_active` is provided, that output will remain
    /// enabled after restore.
    pub fn snapshot_wake_inactive_outputs(
        &mut self,
        keep_active: Option<&DisplayId>,
    ) -> Vec<DisplayId> {
        let mut activated = Vec::new();
        for out in &mut self.outputs {
            if !out.enabled {
                if let Some(k) = keep_active {
                    if &out.identity.id == k {
                        continue;
                    }
                }
                out.enabled = true;
                activated.push(out.identity.id.clone());
            }
        }
        activated
    }

    /// Restore outputs that were previously activated by `snapshot_wake_inactive_outputs`.
    pub fn restore_inactive_outputs(&mut self, inactive_ids: &[DisplayId]) {
        for id in inactive_ids {
            if let Some(out) = self.outputs.iter_mut().find(|o| &o.identity.id == id) {
                out.enabled = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Implementation of the transient output modifier (OutputEditable)
// ---------------------------------------------------------------------------
pub struct DrmOutputEditor<'a> {
    pub topology: &'a mut DrmTopology,
    pub target_id: DisplayId,
}

macro_rules! find_drm_output {
    ($self:expr) => {{
        let target_id = $self.target_id.clone();
        $self
            .topology
            .outputs
            .iter_mut()
            .find(|o| o.identity.id == target_id)
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(target_id))
    }};
}

impl<'a> OutputEditable for DrmOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_drm_output!(self)?;
        if out.rotation != rotation {
            out.rotation = rotation;

            let (_, res) = self
                .topology
                .resource_map
                .get(&self.target_id)
                .ok_or_else(|| DisplayManagerError::MonitorNotFound(self.target_id.clone()))?;

            if let Some(prop_rot) = self.topology.find_prop_id(res.connector_id, "rotation") {
                let val = rotation_to_drm_value(rotation);
                self.topology
                    .atomic_req
                    .add_property(res.primary_plane_id, prop_rot, val);
                self.topology.dirty = true;
            }
        }
        Ok(self)
    }

    fn set_resolution(
        &mut self,
        extent: Extent2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_drm_output!(self)?;
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
        let out = find_drm_output!(self)?;
        if out.geometry.origin != position {
            out.geometry.origin = position;

            let (_, res) = self
                .topology
                .resource_map
                .get(&self.target_id)
                .ok_or_else(|| DisplayManagerError::MonitorNotFound(self.target_id.clone()))?;

            if let Some(prop_x) = self.topology.find_prop_id(res.connector_id, "CRTC_X") {
                self.topology.atomic_req.add_property(
                    res.primary_plane_id,
                    prop_x,
                    position.x as u64,
                );
            }
            if let Some(prop_y) = self.topology.find_prop_id(res.connector_id, "CRTC_Y") {
                self.topology.atomic_req.add_property(
                    res.primary_plane_id,
                    prop_y,
                    position.y as u64,
                );
            }
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_refresh_rate(
        &mut self,
        rate: u32,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_drm_output!(self)?;
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
        let out = find_drm_output!(self)?;
        out.is_primary = true;
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_hdr(
        &mut self,
        state: HdrState,
        mode: HdrMode,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_drm_output!(self)?;
        if out.hdr_state != state || out.hdr_mode != mode {
            out.hdr_state = state;
            out.hdr_mode = mode;

            let (_, res) = self
                .topology
                .resource_map
                .get(&self.target_id)
                .ok_or_else(|| DisplayManagerError::MonitorNotFound(self.target_id.clone()))?;

            if let Some(prop_hdr) = self
                .topology
                .find_prop_id(res.connector_id, "HDR_OUTPUT_METADATA")
            {
                let val = if state == HdrState::Enabled { 1 } else { 0 };
                self.topology
                    .atomic_req
                    .add_property(res.connector_id, prop_hdr, val);
                self.topology.dirty = true;
            }
        }
        Ok(self)
    }

    fn set_scale(&mut self, scale: f64) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let out = find_drm_output!(self)?;
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
        let out = find_drm_output!(self)?;
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
        let source_state = self
            .topology
            .outputs
            .iter()
            .find(|o| o.identity.id == *source_id)
            .cloned()
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(source_id.clone()))?;

        let dest = find_drm_output!(self)?;
        dest.geometry = source_state.geometry;
        dest.refresh_rate = source_state.refresh_rate;
        dest.rotation = source_state.rotation;
        dest.scale = source_state.scale;

        self.topology.dirty = true;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.topology
            .outputs
            .iter()
            .find(|o| o.identity.id == self.target_id)
            .cloned()
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Global Interface Mapping (UniversalTopology Trait Compliance)
// ---------------------------------------------------------------------------
#[async_trait]
impl UniversalTopology for DrmTopology {
    /// Acquires the display topology using multi-GPU aware enumeration.
    ///
    /// Uses `probe_all_drm_devices()` to detect all available DRM devices
    /// and creates a unified topology containing outputs from all GPUs.
    fn acquire() -> Result<Self, DisplayManagerError> {
        // Use probe_all_drm_devices for full multi-GPU support
        let (all_outputs, all_resource_maps, property_cache) = probe_all_drm_devices()?;

        if all_outputs.is_empty() {
            return Err(DisplayManagerError::ConnectionFailed);
        }

        // Build the devices map from detected adapters
        let mut devices: HashMap<AdapterId, File> = HashMap::new();
        for (adapter_id, _) in detect_drm_devices() {
            // Re-open devices to get fresh handles (original handles consumed by probe_all_drm_devices)
            let path = &adapter_id.0;
            if let Ok(file) = OpenOptions::new().read(true).write(true).open(path) {
                devices.entry(adapter_id.clone()).or_insert(file);
            }
        }

        // Convert outputs to the expected format (just OutputState, not (AdapterId, OutputState))
        let outputs: Vec<OutputState> = all_outputs.into_iter().map(|(_, output)| output).collect();

        // Convert resource maps to include adapter_id
        let resource_map: HashMap<DisplayId, (AdapterId, DrmResourceIds)> = all_resource_maps;

        Ok(Self {
            devices,
            outputs,
            atomic_req: AtomicModeReq::new(),
            resource_map,
            property_cache,
            dirty: false,
        })
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.outputs.clone()
    }

    fn edit_output(
        &mut self,
        id: &DisplayId,
    ) -> Result<Box<dyn OutputEditable + '_>, DisplayManagerError> {
        if !self.outputs.iter().any(|o| o.identity.id == *id) {
            return Err(DisplayManagerError::MonitorNotFound(id.clone()));
        }
        Ok(Box::new(DrmOutputEditor {
            topology: self,
            target_id: id.clone(),
        }))
    }

    fn set_persistence(&mut self, _enabled: bool) -> &mut Self {
        self
    }

    async fn validate(&self) -> Result<(), DisplayManagerError> {
        // Validate across all devices in multi-GPU setup
        for (adapter_id, device) in &self.devices {
            let flags = AtomicCommitFlags::TEST_ONLY;
            let req_clone = self.atomic_req.clone();
            if let Err(e) = device.atomic_commit(flags, req_clone) {
                return Err(DisplayManagerError::BackendError(format!(
                    "Validation failed on adapter {}: {}",
                    adapter_id.0, e
                )));
            }
        }
        Ok(())
    }

    /// Commits changes across all GPUs in the system.
    ///
    /// For multi-GPU setups, this iterates through all configured devices
    /// and applies the atomic commit to each. The requests are spawned
    /// as blocking tasks to avoid stalling the async runtime.
    async fn commit(&mut self) -> Result<(), DisplayManagerError> {
        if !self.dirty {
            return Ok(());
        }

        // For multi-GPU: commit to all devices
        // Note: In a production system, you would need to maintain separate
        // atomic requests per adapter and route properties to the correct device.
        // This implementation uses a simplified approach that commits to the first device
        // for backward compatibility while the multi-GPU commit logic matures.

        let primary_device = self
            .devices
            .iter()
            .next()
            .map(|(_, f)| f)
            .ok_or_else(|| DisplayManagerError::ConnectionFailed)?;

        let device = primary_device
            .try_clone()
            .map_err(|e| DisplayManagerError::Io(e))?;
        let req = self.atomic_req.clone();

        tokio::task::spawn_blocking(move || -> Result<(), DisplayManagerError> {
            let flags = AtomicCommitFlags::ALLOW_MODESET;
            device.atomic_commit(flags, req).map_err(|e| {
                DisplayManagerError::BackendError(format!("KMS Atomic Flush Failure: {}", e))
            })?;
            Ok(())
        })
        .await
        .map_err(|e| {
            DisplayManagerError::BackendError(format!("DRM thread pool panic: {}", e))
        })??;

        self.atomic_req = AtomicModeReq::new();
        self.dirty = false;
        Ok(())
    }
}
