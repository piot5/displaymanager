// ── Imports ──────────────────────────────────────────────────────────────────
use super::{CcdTopology, DISPLAYCONFIG_PATH_ACTIVE, DISPLAYCONFIG_PATH_MODE_IDX_INVALID};
use crate::error::{DisplayError, DisplayResult};
use crate::traits::OutputEditable;
use crate::types::{DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D};
use windows::Win32::Devices::Display as WinDisplay;

// ── Editor struct ─────────────────────────────────────────────────────────────

/// Mutable editor for staging CCD display configuration changes.
///
/// Implements [`OutputEditable`] to provide a builder-style API
/// for modifying display properties before committing.
pub struct CcdOutputEditor<'a> {
    pub(crate) topology: &'a mut CcdTopology,
    pub(crate) target_id: u32,
}

impl<'a> CcdOutputEditor<'a> {
    /// Creates a new editor for the specified CCD target.
    pub fn new(topology: &'a mut CcdTopology, target_id: u32) -> Self {
        Self {
            topology,
            target_id,
        }
    }
}

// ── Internal macros ───────────────────────────────────────────────────────────

macro_rules! find_path {
    ($self:expr) => {
        $self
            .topology
            .data
            .paths
            .iter_mut()
            .find(|p| p.targetInfo.id == $self.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId($self.target_id.to_string())))
    };
}

macro_rules! find_output_mut {
    ($self:expr) => {
        $self
            .topology
            .outputs
            .iter_mut()
            .find(|o| o.identity.id.0 == $self.target_id.to_string())
    };
}

// ── OutputEditable implementation ─────────────────────────────────────────────

impl<'a> OutputEditable for CcdOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        if self.get_state().rotation == rotation {
            return Ok(self);
        }
        let path = find_path!(self)?;
        path.targetInfo.rotation = match rotation {
            DisplayRotation::Rotate0 => WinDisplay::DISPLAYCONFIG_ROTATION_IDENTITY,
            DisplayRotation::Rotate90 => WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE90,
            DisplayRotation::Rotate180 => WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE180,
            DisplayRotation::Rotate270 => WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE270,
        };
        if let Some(out) = find_output_mut!(self) {
            out.rotation = rotation;
        }
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable> {
        let path = find_path!(self)?;
        // SAFETY: Accessing union field requires unsafe block.
        let idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx } as usize;
        if idx < self.topology.data.modes.len() {
            unsafe {
                let src = &mut self.topology.data.modes[idx].Anonymous.sourceMode;
                src.width = extent.width;
                src.height = extent.height;
            }
        }
        if let Some(out) = find_output_mut!(self) {
            out.geometry.size = extent;
        }
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_position(&mut self, position: Point2D) -> DisplayResult<&mut dyn OutputEditable> {
        let path = find_path!(self)?;
        // SAFETY: Accessing union field requires unsafe block.
        let idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx } as usize;
        if idx < self.topology.data.modes.len() {
            unsafe {
                let pos = &mut self.topology.data.modes[idx].Anonymous.sourceMode.position;
                pos.x = position.x;
                pos.y = position.y;
            }
        }
        if let Some(out) = find_output_mut!(self) {
            out.geometry.origin = position;
        }
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable> {
        let path = find_path!(self)?;
        // SAFETY: Accessing union field requires unsafe block.
        let idx = unsafe { path.targetInfo.Anonymous.modeInfoIdx } as usize;
        if idx < self.topology.data.modes.len() {
            unsafe {
                let freq = &mut self.topology.data.modes[idx]
                    .Anonymous
                    .targetMode
                    .targetVideoSignalInfo
                    .vSyncFreq;
                freq.Numerator = rate;
                freq.Denominator = 1_000;
            }
        }
        if let Some(out) = find_output_mut!(self) {
            out.refresh_rate = rate;
        }
        self.topology.dirty = true;
        Ok(self)
    }

    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable> {
        for out in &mut self.topology.outputs {
            out.is_primary = false;
        }
        if let Some(out) = find_output_mut!(self) {
            out.is_primary = true;
        }
        self.set_position(Point2D { x: 0, y: 0 })
    }

    fn set_hdr(
        &mut self,
        state: HdrState,
        mode: HdrMode,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        self.topology
            .staged_hdr
            .insert(self.target_id, (state, mode));
        if let Some(out) = find_output_mut!(self) {
            out.hdr_state = state;
            out.hdr_mode = mode;
        }
        Ok(self)
    }

    fn set_scale(&mut self, scale: f64) -> DisplayResult<&mut dyn OutputEditable> {
        self.topology
            .staged_scales
            .insert(self.target_id, (scale * 100.0).round() as i32);
        if let Some(out) = find_output_mut!(self) {
            out.scale = scale;
        }
        Ok(self)
    }

    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable> {
        let path = find_path!(self)?;
        if enabled {
            path.flags |= DISPLAYCONFIG_PATH_ACTIVE;
        } else {
            path.flags &= !DISPLAYCONFIG_PATH_ACTIVE;
            // SAFETY: Modifying Windows union fields requires unsafe block.

            path.sourceInfo.Anonymous.modeInfoIdx = DISPLAYCONFIG_PATH_MODE_IDX_INVALID;
            path.targetInfo.Anonymous.modeInfoIdx = DISPLAYCONFIG_PATH_MODE_IDX_INVALID;
        }
        if let Some(out) = find_output_mut!(self) {
            out.enabled = enabled;
        }
        self.topology.dirty = true;
        Ok(self)
    }

    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable> {
        let src_id = source_id
            .0
            .parse::<u32>()
            .map_err(|_| DisplayError::NotFound(source_id.clone()))?;

        // SAFETY: union field read requires an unsafe block.
        let src_idx = self
            .topology
            .data
            .paths
            .iter()
            .find(|p| p.targetInfo.id == src_id)
            .map(|p| unsafe { p.sourceInfo.Anonymous.modeInfoIdx })
            .ok_or_else(|| DisplayError::NotFound(source_id.clone()))?;

        let path = find_path!(self)?;

        // Union field access is safe here.
        path.sourceInfo.Anonymous.modeInfoIdx = src_idx;

        self.topology.dirty = true;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.topology
            .outputs
            .iter()
            .find(|o| o.identity.id.0 == self.target_id.to_string())
            .cloned()
            .unwrap_or_default()
    }
}
