// linux.rs
use async_trait::async_trait;
use std::fmt;
use crate::error::{DisplayError, DisplayResult};
use crate::traits::{UniversalTopology, OutputEditable};
use crate::types::{
    OutputState, DisplayRotation, HdrState, HdrMode,
    DisplayId, Extent2D, Point2D, Rect,
};

/// Full Wayland/wlroots implementation using the zwlr-output-management-v1 protocol.
/// Contains the production-grade event-loop-based topology with color-management support.
pub mod displmgr_wlr;

/// DRM/KMS backend for X11 and bare-metal Linux environments (no compositor).
/// Uses Atomic KMS for flicker-free, synchronised updates.
pub mod displmgr_drm;

// ---------------------------------------------------------------------------
// WlrOutputState — internal Wayland protocol tracking record
// ---------------------------------------------------------------------------

/// Internal tracking state representing the low-level Wayland protocol values.
///
/// `scale` is kept as `f32` because the zwlr-output-management-v1 protocol
/// exposes fractional scale as a 32-bit float. When this state is projected
/// into the cross-platform `OutputState`, it is widened to `f64` with `as f64`.
#[derive(Debug, Clone, Default)]
pub struct WlrOutputState {
    pub id: DisplayId,
    pub connector_id: crate::types::ConnectorId,
    pub adapter_id: crate::types::AdapterId,
    pub geometry: Rect,
    pub refresh_rate: u32,
    pub scale: f32,
    pub rotation: DisplayRotation,
    pub enabled: bool,
    pub is_primary: bool,
}

#[derive(Clone)]
pub struct WlrTopology {
    /// Snapshot of all output heads as last reported by the compositor.
    pub outputs: Vec<WlrOutputState>,
    pub persistence_enabled: bool,
    pub dirty: bool,
}

impl fmt::Debug for WlrTopology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WlrTopology")
            .field("outputs_count", &self.outputs.len())
            .field("persistence_enabled", &self.persistence_enabled)
            .field("dirty", &self.dirty)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Helper macro for local output lookup (parity with Windows subsystem)
// ---------------------------------------------------------------------------
macro_rules! find_wlr_output {
    ($self:expr) => {{
        let target_id = $self.target_id.clone();
        $self
            .topology
            .outputs
            .iter_mut()
            .find(|o| o.id == target_id)
            .ok_or_else(|| DisplayError::NotFound(target_id))
    }};
}

// ---------------------------------------------------------------------------
// WlrOutputEditor — transient per-output modifier
// ---------------------------------------------------------------------------
pub struct WlrOutputEditor<'a> {
    pub topology: &'a mut WlrTopology,
    pub target_id: DisplayId,
}

impl<'a> OutputEditable for WlrOutputEditor<'a> {
    fn set_rotation(&mut self, rotation: DisplayRotation) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        if out.rotation != rotation {
            out.rotation = rotation;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        if out.geometry.size != extent {
            out.geometry.size = extent;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_position(&mut self, position: Point2D) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        if out.geometry.origin != position {
            out.geometry.origin = position;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        if out.refresh_rate != rate {
            out.refresh_rate = rate;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable> {
        // Clear the primary flag on every output first to uphold the
        // single-primary invariant across the virtual desktop layout.
        for out in &mut self.topology.outputs {
            out.is_primary = false;
        }
        let out = find_wlr_output!(self)?;
        out.is_primary = true;
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_hdr(&mut self, _state: HdrState, _mode: HdrMode) -> DisplayResult<&mut dyn OutputEditable> {
        // True HDR on Linux requires explicit Wayland event loops using
        // color-management-v1 protocols — not available in this simplified path.
        Err(DisplayError::UnsupportedFeature(
            "Advanced HDR via color-management-v1 requires explicit \
             dispmgr_wlr event-loop context."
                .into(),
        ))
    }

    // FIX: parameter was `f32`; the trait requires `f64`.
    // The internal WlrOutputState stores scale as `f32` (matching the
    // Wayland protocol), so we narrow with `as f32` inside the method.
    fn set_scale(&mut self, scale: f64) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        let scale_f32 = scale as f32;
        if (out.scale - scale_f32).abs() > f32::EPSILON {
            out.scale = scale_f32;
            self.topology.dirty = true;
        }
        Ok(self)
    }

    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable> {
        // Snapshot the source before borrowing the destination to satisfy
        // Rust's borrow checker.
        let source = self
            .topology
            .outputs
            .iter()
            .find(|o| o.id == *source_id)
            .ok_or_else(|| DisplayError::NotFound(source_id.clone()))?
            .clone();

        let dest = find_wlr_output!(self)?;
        dest.geometry     = source.geometry;
        dest.refresh_rate = source.refresh_rate;
        dest.scale        = source.scale;   // both f32 — no cast needed
        dest.rotation     = source.rotation;

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
                    id:           o.id.clone(),
                    connector_id: o.connector_id.clone(),
                    adapter_id:   o.adapter_id.clone(),
                    hardware_uuid: None,
                    monitor_name:  format!("Wayland Head ({})", o.connector_id.0),
                },
                geometry:          o.geometry,
                refresh_rate:      o.refresh_rate,
                rotation:          o.rotation,
                hdr_state:         HdrState::Disabled,
                hdr_mode:          HdrMode::Default,
                // FIX: widen f32 → f64; Rust has no implicit numeric coercion.
                scale:             o.scale as f64,
                native_resolution: Some(o.geometry.size),
                supported_modes:   vec![],
                // FIX: was missing — silently dropped enabled/is_primary.
                enabled:           o.enabled,
                is_primary:        o.is_primary,
            })
            .unwrap_or_default()
    }

    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable> {
        let out = find_wlr_output!(self)?;
        if out.enabled != enabled {
            out.enabled = enabled;
            self.topology.dirty = true;
        }
        Ok(self)
    }
}

// ---------------------------------------------------------------------------
// WlrTopology — UniversalTopology trait implementation
// ---------------------------------------------------------------------------
#[async_trait]
impl UniversalTopology for WlrTopology {
    fn acquire() -> DisplayResult<Self> {
        // In a live environment, Wayland registry binding happens here.
        // For trait compilation parity we initialise an empty operational state.
        Ok(Self {
            outputs: Vec::new(),
            persistence_enabled: false,
            dirty: false,
        })
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.outputs
            .iter()
            .map(|o| OutputState {
                identity: crate::types::DisplayIdentity {
                    id:           o.id.clone(),
                    connector_id: o.connector_id.clone(),
                    adapter_id:   o.adapter_id.clone(),
                    hardware_uuid: None,
                    monitor_name:  format!("Wayland Head ({})", o.connector_id.0),
                },
                geometry:          o.geometry,
                refresh_rate:      o.refresh_rate,
                rotation:          o.rotation,
                hdr_state:         HdrState::Disabled,
                hdr_mode:          HdrMode::Default,
                // FIX: widen f32 → f64.
                scale:             o.scale as f64,
                native_resolution: Some(o.geometry.size),
                supported_modes:   vec![],
                // FIX: was missing — enabled/is_primary were not propagated.
                enabled:           o.enabled,
                is_primary:        o.is_primary,
            })
            .collect()
    }

    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        if !self.outputs.iter().any(|o| o.id == *id) {
            return Err(DisplayError::NotFound(id.clone()));
        }
        Ok(Box::new(WlrOutputEditor {
            topology:  self,
            target_id: id.clone(),
        }))
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        self.persistence_enabled = enabled;
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        // Geometric overlap check within virtual desktop coordinates.
        for (i, out_a) in self.outputs.iter().enumerate() {
            for out_b in self.outputs.iter().skip(i + 1) {
                if out_a.enabled && out_b.enabled {
                    let a_x2 = out_a.geometry.origin.x + out_a.geometry.size.width  as i32;
                    let a_y2 = out_a.geometry.origin.y + out_a.geometry.size.height as i32;
                    let b_x2 = out_b.geometry.origin.x + out_b.geometry.size.width  as i32;
                    let b_y2 = out_b.geometry.origin.y + out_b.geometry.size.height as i32;

                    let overlap_x = out_a.geometry.origin.x < b_x2 && a_x2 > out_b.geometry.origin.x;
                    let overlap_y = out_a.geometry.origin.y < b_y2 && a_y2 > out_b.geometry.origin.y;

                    if overlap_x && overlap_y {
                        return Err(DisplayError::ConfigurationRejected);
                    }
                }
            }
        }
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Isolate Wayland FFI inside a blocking worker to maintain safety guarantees.
        let mut local_self = self.clone();
        let updated_topology = tokio::task::spawn_blocking(move || -> DisplayResult<WlrTopology> {
            // Low-level wlroots protocol calls go here:
            // e.g., zwlr_output_configuration_v1_apply

            local_self.dirty = false;
            Ok(local_self)
        })
        .await
        .map_err(|e| {
            DisplayError::BackendError(format!("Wayland thread pool execution panic: {}", e))
        })??;

        *self = updated_topology;
        Ok(())
    }
}