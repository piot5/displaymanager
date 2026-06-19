// displmgr_gdi.rs
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, CDS_NORESET, CDS_SET_PRIMARY, CDS_TYPE, CDS_UPDATEREGISTRY, DEVMODEW,
    DEVMODE_DISPLAY_ORIENTATION, DISP_CHANGE_SUCCESSFUL, DMDO_180, DMDO_270, DMDO_90, DMDO_DEFAULT,
    DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{DisplayId, DisplayRotation, OutputState};

/// Low-level GDI API functions for display enumeration and mode-setting.
pub mod displmgr_gdi_api;
/// GDI output editor implementing [`OutputEditable`].
pub mod displmgr_gdi_editor;
/// Win32 GDI system type helpers and flag constants.
pub mod displmgr_gdi_sys;

use self::displmgr_gdi_api::query_gdi_outputs;
pub use self::displmgr_gdi_api::{force_activate_by_monitor_name, force_all};
use self::displmgr_gdi_editor::GdiOutputEditor;
use self::displmgr_gdi_sys::to_wide;

/// Windows GDI (Graphics Device Interface) display topology backend.
///
/// This is the legacy fallback backend for Windows display management.
/// It uses `EnumDisplayDevicesW` and `ChangeDisplaySettingsExW` to
/// enumerate and configure displays. While less capable than the CCD
/// backend (no atomic multi-monitor commits, no HDR path), it provides
/// wider compatibility across all Windows versions.
///
/// # Lifecycle
///
/// 1. [`acquire`](Self::acquire) — Enumerate all GDI display devices
///    and snapshot their current `DEVMODEW` configurations.
/// 2. [`edit_output`](Self::edit_output) — Stage mutations via
///    [`GdiOutputEditor`].
/// 3. [`commit`](Self::commit) — Flush all staged changes to the
///    Windows Registry and trigger a global display-mode reset.
#[derive(Clone)]
pub struct GdiTopology {
    /// Map of GDI device names to their current output state.
    pub outputs: HashMap<String, OutputState>,
    /// When `true`, changes are written to the Registry (`CDS_UPDATEREGISTRY`).
    pub persistence_enabled: bool,
    /// Raw DEVMODEW snapshots captured at acquisition time.
    /// Used as the base for computing the next mode during commit.
    pub(crate) saved_modes: HashMap<String, DEVMODEW>,
    /// Optional target primary display identifier.
    pub(crate) target_primary_id: Option<String>,
}

/// RAII guard that restores all GDI display settings to their previous
/// state when dropped.
///
/// Captures the current topology snapshot and reapplies it on drop,
/// ensuring that temporary configuration changes are rolled back even
/// if the caller panics or drops the restorer early.
pub struct DisplayRestorer {
    /// Vector of (wide_name, was_active, DEVMODEW) tuples captured
    /// at construction time.
    // Stores: (name, was_active, DEVMODEW)
    pub snapshot: Vec<(Vec<u16>, bool, DEVMODEW)>,
}

impl Drop for DisplayRestorer {
    fn drop(&mut self) {
        for (name_u16, was_active, old_dm) in &self.snapshot {
            let pcw_name = PCWSTR(name_u16.as_ptr());
            let mut reset_dm = *old_dm;

            let flags = CDS_UPDATEREGISTRY | CDS_NORESET;

            if !*was_active {
                reset_dm.dmPelsWidth = 0;
                reset_dm.dmPelsHeight = 0;
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
            }

            unsafe {
                let _ = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, flags, None);
            }
        }
        unsafe {
            let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None);
        }
    }
}

impl fmt::Debug for GdiTopology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GdiTopology")
            .field("outputs", &self.outputs)
            .field("persistence_enabled", &self.persistence_enabled)
            .field("target_primary_id", &self.target_primary_id)
            .finish()
    }
}

#[async_trait]
impl UniversalTopology for GdiTopology {
    fn acquire() -> DisplayResult<Self> {
        let (saved_modes, outputs_vec) = query_gdi_outputs()?;
        let mut outputs = HashMap::new();
        for state in outputs_vec {
            outputs.insert(state.identity.id.0.clone(), state);
        }
        Ok(Self {
            outputs,
            persistence_enabled: true,
            saved_modes,
            target_primary_id: None,
        })
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.outputs.values().cloned().collect()
    }

    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        if !self.outputs.contains_key(&id.0) {
            return Err(DisplayError::NotFound(id.clone()));
        }
        Ok(Box::new(GdiOutputEditor::new(self, id.0.clone())))
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        self.persistence_enabled = enabled;
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        // ── Phase 1: Primary monitor ─────────────────────────────────────────
        if let Some(primary_id) = self.target_primary_id.clone() {
            if let (Some(state), Some(&saved)) = (
                self.outputs.get(&primary_id).cloned(),
                self.saved_modes.get(&primary_id),
            ) {
                let mut devmode = saved;
                devmode.dmFields |=
                    DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                devmode.dmPelsWidth = state.geometry.size.width;
                devmode.dmPelsHeight = state.geometry.size.height;

                // SAFETY: Anonymous1.Anonymous2 is the union arm valid for display-mode entries.
                // The unsafe block was removed as windows-rs now provides safe accessors.
                devmode.Anonymous1.Anonymous2.dmPosition.x = 0;
                devmode.Anonymous1.Anonymous2.dmPosition.y = 0;
                devmode.Anonymous1.Anonymous2.dmDisplayOrientation =
                    rotation_to_dmdo(state.rotation);

                let flags = if self.persistence_enabled {
                    CDS_UPDATEREGISTRY | CDS_NORESET | CDS_SET_PRIMARY
                } else {
                    CDS_NORESET | CDS_SET_PRIMARY
                };
                let wide_id = to_wide(&primary_id);
                if unsafe {
                    ChangeDisplaySettingsExW(
                        PCWSTR(wide_id.as_ptr()),
                        Some(&devmode),
                        None,
                        flags,
                        None,
                    )
                } != DISP_CHANGE_SUCCESSFUL
                {
                    return Err(DisplayError::BackendError("Primary staging failed".into()));
                }
            }
        }

        // ── Phase 2: Secondary monitors ──────────────────────────────────────
        for (id, state) in &self.outputs {
            if Some(id) == self.target_primary_id.as_ref() {
                continue;
            }
            let mut devmode = *self
                .saved_modes
                .get(id)
                .ok_or_else(|| DisplayError::NotFound(DisplayId(id.clone())))?;

            if state.enabled {
                devmode.dmFields |=
                    DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                devmode.dmPelsWidth = state.geometry.size.width;
                devmode.dmPelsHeight = state.geometry.size.height;

                // SAFETY: Removed redundant unsafe block.
                devmode.Anonymous1.Anonymous2.dmPosition.x = state.geometry.origin.x;
                devmode.Anonymous1.Anonymous2.dmPosition.y = state.geometry.origin.y;
                devmode.Anonymous1.Anonymous2.dmDisplayOrientation =
                    rotation_to_dmdo(state.rotation);
            } else {
                devmode.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT;
                devmode.dmPelsWidth = 0;
                devmode.dmPelsHeight = 0;
            }

            let flags = if self.persistence_enabled {
                CDS_UPDATEREGISTRY | CDS_NORESET
            } else {
                CDS_NORESET
            };
            let wide_id = to_wide(id);
            if unsafe {
                ChangeDisplaySettingsExW(
                    PCWSTR(wide_id.as_ptr()),
                    Some(&devmode),
                    None,
                    flags,
                    None,
                )
            } != DISP_CHANGE_SUCCESSFUL
            {
                return Err(DisplayError::BackendError(
                    "Secondary staging failed".into(),
                ));
            }
        }

        // ── Phase 3: Global flush ────────────────────────────────────────────
        if unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None) }
            != DISP_CHANGE_SUCCESSFUL
        {
            return Err(DisplayError::BackendError("Global flush failed".into()));
        }

        self.target_primary_id = None;
        Ok(())
    }
}

impl GdiTopology {
    /// Enable all detected GDI outputs temporarily while capturing a snapshot
    /// of those that were previously disabled.
    ///
    /// If a specific output is being activated as part of the wake path, pass
    /// its `DisplayId` so it is excluded from the restore list and remains
    /// active afterward.
    pub fn snapshot_wake_inactive_outputs(
        &mut self,
        keep_active: Option<&DisplayId>,
    ) -> Vec<String> {
        let previously_inactive: Vec<String> = self
            .outputs
            .iter()
            .filter_map(|(id, state)| {
                if !state.enabled && keep_active.is_none_or(|keep| &keep.0 != id) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        for state in self.outputs.values_mut() {
            if !state.enabled {
                state.enabled = true;
            }
        }

        previously_inactive
    }

    /// Restore the previously inactive outputs after a wake operation.
    ///
    /// Only the listed outputs are disabled again, so the previously selected
    /// activated target remains enabled.
    pub fn restore_inactive_outputs(&mut self, inactive_ids: &[String]) {
        for id in inactive_ids {
            if let Some(state) = self.outputs.get_mut(id) {
                state.enabled = false;
            }
        }
    }
}

/// Converts a cross-platform `DisplayRotation` value to the corresponding
/// Win32 `dmDisplayOrientation` constant.
#[inline]
fn rotation_to_dmdo(rotation: DisplayRotation) -> DEVMODE_DISPLAY_ORIENTATION {
    match rotation {
        DisplayRotation::Rotate0 => DMDO_DEFAULT,
        DisplayRotation::Rotate90 => DMDO_90,
        DisplayRotation::Rotate180 => DMDO_180,
        DisplayRotation::Rotate270 => DMDO_270,
    }
}
