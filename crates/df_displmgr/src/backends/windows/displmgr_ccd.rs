use async_trait::async_trait;
use std::collections::HashMap;
use windows::Win32::Devices::Display as WinDisplay;
use windows::Win32::Devices::Display::{
    SetDisplayConfig, SDC_APPLY, SDC_SAVE_TO_DATABASE, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SDC_VALIDATE,
};

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{DisplayId, HdrMode, HdrState, OutputState};

/// CCD API functions for querying and configuring display targets.
pub mod displmgr_ccd_api;

/// CCD system-level FFI types and constants.
/// Low-level Win32 FFI types for CCD display configuration.
pub mod displmgr_ccd_sys;

/// CCD output editor for staging display configuration changes.
pub mod displmgr_ccd_editor;

use self::displmgr_ccd_api::query_config_sync;
use self::displmgr_ccd_editor::CcdOutputEditor;
pub use self::displmgr_ccd_sys::*;

// Re-export CCD-level display management functions from the API module
pub use self::displmgr_ccd_api::{
    ccd_wake_display, find_display_target, query_all_display_targets, DisplayTargetInfo,
};

/// CCD (Connecting and Configuring Displays) topology implementation.
///
/// This is the modern Windows display management backend that uses the
/// `SetDisplayConfig` / `QueryDisplayConfig` API for flicker-free,
/// atomic display configuration changes.
#[derive(Debug, Clone)]
pub struct CcdTopology {
    /// Raw CCD data including display paths and modes.
    pub data: CcdRawData,
    /// Snapshot of all display outputs as reported by the CCD subsystem.
    pub outputs: Vec<OutputState>,
    /// Whether changes should be persisted to the Windows Registry.
    pub persist: bool,
    /// Staged DPI scale factors keyed by target ID.
    pub(crate) staged_scales: HashMap<u32, i32>,
    /// Staged HDR state and mode keyed by target ID.
    pub(crate) staged_hdr: HashMap<u32, (HdrState, HdrMode)>,
    /// Whether any output has been modified since the last commit.
    pub(crate) dirty: bool,
}

#[async_trait]
impl UniversalTopology for CcdTopology {
    fn acquire() -> DisplayResult<Self> {
        let (raw_data, outputs) = query_config_sync().map_err(|e| {
            DisplayError::BackendError(format!("Initial QueryDisplayConfig failed: {:?}", e))
        })?;

        Ok(CcdTopology {
            data: raw_data,
            outputs,
            persist: true,
            staged_scales: HashMap::new(),
            staged_hdr: HashMap::new(),
            dirty: false,
        })
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.outputs.clone()
    }

    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        let target_id =
            id.0.parse::<u32>()
                .map_err(|_| DisplayError::NotFound(id.clone()))?;
        if !self.data.paths.iter().any(|p| p.targetInfo.id == target_id) {
            return Err(DisplayError::NotFound(id.clone()));
        }

        Ok(Box::new(CcdOutputEditor::new(self, target_id)))
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        self.persist = enabled;
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        let has_work = self.dirty || !self.staged_scales.is_empty() || !self.staged_hdr.is_empty();
        if !has_work || self.data.paths.is_empty() {
            return Ok(());
        }

        // Phase 1: Geometric collision detection
        let active_outputs: Vec<&OutputState> = self
            .outputs
            .iter()
            .filter(|o| o.enabled && o.geometry.size.width > 0 && o.geometry.size.height > 0)
            .collect();

        for i in 0..active_outputs.len() {
            for j in (i + 1)..active_outputs.len() {
                let (o1, o2) = (active_outputs[i], active_outputs[j]);
                if o1.geometry.origin.x < o2.geometry.origin.x + o2.geometry.size.width as i32
                    && o1.geometry.origin.x + o1.geometry.size.width as i32 > o2.geometry.origin.x
                    && o1.geometry.origin.y < o2.geometry.origin.y + o2.geometry.size.height as i32
                    && o1.geometry.origin.y + o1.geometry.size.height as i32 > o2.geometry.origin.y
                {
                    return Err(DisplayError::ConfigurationRejected);
                }
            }
        }

        // Phase 2: Kernel validation
        let status = unsafe {
            SetDisplayConfig(
                Some(&self.data.paths),
                Some(&self.data.modes),
                SDC_VALIDATE | SDC_USE_SUPPLIED_DISPLAY_CONFIG,
            )
        };
        if status != 0 {
            return Err(DisplayError::ConfigurationRejected);
        }
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        if !self.dirty && self.staged_scales.is_empty() && self.staged_hdr.is_empty() {
            return Ok(());
        }
        if self.data.paths.is_empty() {
            return Ok(());
        }

        self.validate().await?;

        let mut flags = SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG;
        if self.persist {
            flags |= SDC_SAVE_TO_DATABASE;
        }

        if self.dirty {
            let status =
                unsafe { SetDisplayConfig(Some(&self.data.paths), Some(&self.data.modes), flags) };
            if status != 0 {
                self.staged_scales.clear();
                self.staged_hdr.clear();
                self.dirty = false;
                return Err(DisplayError::BackendError(format!(
                    "SetDisplayConfig failed: {}",
                    status
                )));
            }
        }

        // Out-of-band: DPI Scaling & HDR
        for (&target_id, &scale) in &self.staged_scales {
            if let Some(path) = self
                .data
                .paths
                .iter()
                .find(|p| p.targetInfo.id == target_id)
            {
                let mut payload = DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
                    header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE,
                        size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>() as u32,
                        adapterId: path.targetInfo.adapterId,
                        id: target_id,
                    },
                    scale_factor_as_percent: scale,
                };
                unsafe {
                    let _ = WinDisplay::DisplayConfigSetDeviceInfo(
                        &mut payload.header as *mut _ as *mut _,
                    );
                }
            }
        }

        for (&target_id, &(hdr_state, _)) in &self.staged_hdr {
            if let Some(path) = self
                .data
                .paths
                .iter()
                .find(|p| p.targetInfo.id == target_id)
            {
                let mut payload = DISPLAYCONFIG_SET_HDR_STATE {
                    header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE,
                        size: std::mem::size_of::<DISPLAYCONFIG_SET_HDR_STATE>() as u32,
                        adapterId: path.targetInfo.adapterId,
                        id: target_id,
                    },
                    enabled: if hdr_state == HdrState::Enabled { 1 } else { 0 },
                };
                unsafe {
                    let _ = WinDisplay::DisplayConfigSetDeviceInfo(
                        &mut payload.header as *mut _ as *mut _,
                    );
                }
            }
        }

        self.staged_scales.clear();
        self.staged_hdr.clear();
        self.dirty = false;
        if let Ok((raw_data, outputs)) = query_config_sync() {
            self.data = raw_data;
            self.outputs = outputs;
        }
        Ok(())
    }
}
