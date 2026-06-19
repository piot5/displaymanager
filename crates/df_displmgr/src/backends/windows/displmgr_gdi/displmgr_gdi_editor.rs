//! GDI output editor implementing [`OutputEditable`] for the Windows GDI backend.
//!
//! Provides the [`GdiOutputEditor`] struct that stages display configuration
//! changes in-memory. All mutations are flushed to hardware during
//! [`UniversalTopology::commit`].

use crate::backends::windows::displmgr_gdi::GdiTopology;
use crate::error::{DisplayError, DisplayResult};
use crate::traits::OutputEditable;
use crate::types::{DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D};

/// Per-output editor for GDI-based display configuration.
///
/// Implements [`OutputEditable`] to provide a builder-style API
/// for modifying display properties before committing. All changes
/// are staged in the parent [`GdiTopology`] state and flushed
/// atomically via [`UniversalTopology::commit`].
pub struct GdiOutputEditor<'a> {
    /// Reference to the parent GDI topology.
    topology: &'a mut GdiTopology,
    /// The display identifier for the output being edited.
    target_id: String,
}

impl<'a> GdiOutputEditor<'a> {
    /// Creates a new GDI output editor for the specified display.
    ///
    /// # Arguments
    ///
    /// * `topology` - Mutable reference to the parent GDI topology.
    /// * `target_id` - The display identifier string (GDI device name).
    pub fn new(topology: &'a mut GdiTopology, target_id: String) -> Self {
        Self {
            topology,
            target_id,
        }
    }
}

impl<'a> OutputEditable for GdiOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.rotation = rotation;
        Ok(self)
    }

    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.geometry.size = extent;
        Ok(self)
    }

    fn set_position(&mut self, position: Point2D) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.geometry.origin = position;
        Ok(self)
    }

    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.refresh_rate = rate;
        Ok(self)
    }

    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable> {
        self.topology.target_primary_id = Some(self.target_id.clone());
        Ok(self)
    }

    fn set_hdr(
        &mut self,
        state_val: HdrState,
        mode_val: HdrMode,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.hdr_state = state_val;
        state.hdr_mode = mode_val;
        Ok(self)
    }

    fn set_scale(&mut self, scale_val: f64) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.scale = scale_val;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.topology
            .outputs
            .get(&self.target_id)
            .cloned()
            .unwrap_or_default()
    }

    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable> {
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;
        state.enabled = enabled;
        Ok(self)
    }

    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable> {
        let source_state = self
            .topology
            .outputs
            .get(&source_id.0)
            .ok_or_else(|| DisplayError::NotFound(source_id.clone()))?
            .clone();
        let state = self
            .topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(self.target_id.clone())))?;

        state.geometry = source_state.geometry;
        state.refresh_rate = source_state.refresh_rate;
        state.rotation = source_state.rotation;
        Ok(self)
    }
}
