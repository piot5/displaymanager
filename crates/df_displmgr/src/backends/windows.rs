use async_trait::async_trait;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tokio::task;
use windows::Win32::Devices::Display as WinDisplay;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{DisplayId, Extent2D, HdrState, OutputState, Point2D};

/// CCD (Continuous Configuration Driver) backend for advanced display topology.
pub mod displmgr_ccd;
/// GDI backend for legacy display configuration.
pub mod displmgr_gdi;

use self::displmgr_ccd::CcdTopology;
use self::displmgr_gdi::GdiTopology;

// Re-export GDI-level functions for public use
pub use self::displmgr_gdi::{force_activate_by_monitor_name, force_all};

// Re-export CCD-level functions for public use
pub use self::displmgr_ccd::{
    ccd_wake_display, find_display_target, query_all_display_targets, DisplayTargetInfo,
};

/// Holds OS-specific routing data connecting CCD target definitions to GDI device names.
#[derive(Debug, Deserialize, Clone)]
pub struct OsLayerData {
    /// The numeric target identifier used inside the Windows CCD subsystem.
    pub target_id: u32,
    /// The Win32 GDI device name path string (e.g., "\\\\.\\DISPLAY1").
    pub gdi_name: String,
}

/// Combines a human-readable monitor descriptor with its low-level OS mapping layers.
#[derive(Debug, Deserialize, Clone)]
pub struct MonitorConfig {
    /// The friendly name retrieved from the display's EDID block or descriptor.
    pub friendly_name: String,
    /// Operating system specific layer connection indices and handles.
    pub os_layer: OsLayerData,
}

/// Root structure for deserializing pre-defined hardware mappings from external storage files.
#[derive(Debug, Deserialize, Clone)]
pub struct EdidDump {
    /// List of pre-configured monitor environments and their physical routing vectors.
    pub monitors: Vec<MonitorConfig>,
}

/// Enforces per-monitor DPI awareness V2 for the current process architecture.
/// This prevents the Windows kernel from scaling coordinates or virtual rectangles falsely,
/// ensuring raw, unscaled physical pixel boundaries are returned during FFI calls.
fn ensure_dpi_aware() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Attempts to parse and load an external hardware configuration map (`hardware_map.json`).
/// Used as a fallback mechanism to supply missing hardware properties across legacy API contexts.
fn load_hardware_map() -> Option<EdidDump> {
    let path = Path::new("hardware_map.json");
    if !path.exists() {
        return None;
    }

    let mut file = File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;

    serde_json::from_str(&contents).ok()
}

/// Helper function to enrich standard GDI outputs with hardware identifiers from a loaded map.
fn enrich_gdi_topology(gdi_backend: &mut GdiTopology, hardware_map: &Option<EdidDump>) {
    if let Some(ref map) = hardware_map {
        for monitor in &map.monitors {
            if let Some(state) = gdi_backend.outputs.get_mut(&monitor.os_layer.gdi_name) {
                // Enrich legacy records with persistent tracking metadata tokens
                state.identity.hardware_uuid = Some(monitor.os_layer.target_id.to_string());
            }
        }
    }
}

/// Windows display backend: routes requests between the modern CCD API
/// and the legacy GDI fallback.
pub struct WinDisplayManager {
    /// Active instance of the modern Connecting and Configuring Displays (CCD) backend.
    ccd: Option<CcdTopology>,
    /// Active instance of the legacy Graphics Device Interface (GDI) fallback backend.
    gdi: GdiTopology,
    /// Internal flag signaling whether the current environment allows active routing via CCD.
    use_ccd: bool,
    /// Cached hardware mappings deserialized during subsystem acquisition routines.
    hardware_map: Option<EdidDump>,
}

impl WinDisplayManager {
    /// Provides public read-only access to the loaded hardware mapping configuration.
    /// This resolves the dead_code warning by exposing an explicit getter interface.
    pub fn hardware_map(&self) -> Option<&EdidDump> {
        self.hardware_map.as_ref()
    }

    /// Returns whether the CCD backend is currently active and being used for routing.
    pub fn uses_ccd(&self) -> bool {
        self.use_ccd
    }

    /// Calculates a position to the right of the rightmost active monitor.
    /// Useful for automatically placing a newly activated display.
    pub fn auto_position_right_of(&self) -> Point2D {
        let outputs = self.get_outputs();
        let rightmost_x = outputs
            .iter()
            .filter(|o| o.enabled && o.geometry.size.width > 0)
            .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
            .max()
            .unwrap_or(1920);
        Point2D {
            x: rightmost_x,
            y: 0,
        }
    }
}

/// High-level activation result containing details about what was done.
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// The CCD target identifier for the activated display.
    pub target_id: u32,
    /// The friendly monitor name.
    pub monitor_name: String,
    /// Whether the display was already active before the operation.
    pub was_already_active: bool,
    /// Whether the CCD wake step was performed.
    pub ccd_wake_performed: bool,
}

/// Display activation sequence:
/// 1. Search for the display by name across CCD (including inactive) and GDI
/// 2. Perform CCD wake if the display is inactive
/// 3. Optionally configure resolution and position via the topology editor
/// 4. Commit the changes
///
/// This is the high-level equivalent of what `test_activate_ccd.rs` does,
/// but integrated into the library API.
pub async fn activate_display(
    manager: &mut WinDisplayManager,
    monitor_name: &str,
    resolution: Option<Extent2D>,
    position: Option<Point2D>,
) -> DisplayResult<ActivationResult> {
    // Step 1: Search for the display target
    let target = find_display_target(monitor_name)
        .ok_or_else(|| DisplayError::NotFound(DisplayId(monitor_name.to_string())))?;

    let target_id = target.target_id;
    let was_already_active = target.is_active;
    let mut ccd_wake_performed = false;

    // Step 2: CCD wake if inactive
    if !target.is_active {
        ccd_wake_performed = ccd_wake_display(target_id)?;
    }

    // Step 3: Configure via topology editor
    // Pre-compute auto position before mutable borrow of manager
    let final_pos = position.unwrap_or_else(|| manager.auto_position_right_of());
    let did = DisplayId(target_id.to_string());
    {
        let mut editor = manager.edit_output(&did)?;

        if let Some(res) = resolution {
            let _ = editor.set_resolution(res);
        }

        let _ = editor.set_position(final_pos);

        // Ensure the display is enabled
        let _ = editor.set_enabled(true);
    }

    // Step 4: Commit changes
    manager.commit().await?;

    Ok(ActivationResult {
        target_id,
        monitor_name: target.friendly_name,
        was_already_active,
        ccd_wake_performed,
    })
}

/// Applies GDI-staged HDR and scale values into the CCD device-info interface.
/// Called after a GDI commit to synchronize state back to the modern CCD layer.
fn apply_gdi_state_to_ccd(gdi: &GdiTopology, fresh_ccd: &CcdTopology) {
    use crate::backends::windows::displmgr_ccd::displmgr_ccd_sys as ccd_sys;

    for gdi_state in gdi.outputs.values() {
        let Some(ref hid) = gdi_state.identity.hardware_uuid else {
            continue;
        };
        let Ok(target_id) = hid.parse::<u32>() else {
            continue;
        };

        for path in &fresh_ccd.data.paths {
            if path.targetInfo.id != target_id {
                continue;
            }

            // Apply DPI scale if different from default
            if (gdi_state.scale - 1.0).abs() > f64::EPSILON {
                let scale_percent = (gdi_state.scale * 100.0).round() as i32;
                let mut payload = ccd_sys::DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
                    header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: ccd_sys::DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE,
                        size: std::mem::size_of::<ccd_sys::DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>()
                            as u32,
                        adapterId: path.targetInfo.adapterId,
                        id: target_id,
                    },
                    scale_factor_as_percent: scale_percent,
                };
                let res = unsafe {
                    WinDisplay::DisplayConfigSetDeviceInfo(&mut payload.header as *mut _ as *mut _)
                };
                if res != 0 {
                    eprintln!(
                        "Warning: DPI scale set failed for target {}: 0x{:08X}",
                        target_id, res
                    );
                }
            }

            // Apply HDR state if requested
            if gdi_state.hdr_state != HdrState::Disabled {
                let enabled = if gdi_state.hdr_state == HdrState::Enabled {
                    1
                } else {
                    0
                };
                let mut payload = ccd_sys::DISPLAYCONFIG_SET_HDR_STATE {
                    header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: ccd_sys::DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE,
                        size: std::mem::size_of::<ccd_sys::DISPLAYCONFIG_SET_HDR_STATE>() as u32,
                        adapterId: path.targetInfo.adapterId,
                        id: target_id,
                    },
                    enabled,
                };
                let res = unsafe {
                    WinDisplay::DisplayConfigSetDeviceInfo(&mut payload.header as *mut _ as *mut _)
                };
                if res != 0 {
                    eprintln!(
                        "Warning: HDR set failed for target {}: 0x{:08X}",
                        target_id, res
                    );
                }
            }

            break; // Found the matching path, no need to check further
        }
    }
}

#[async_trait]
impl UniversalTopology for WinDisplayManager {
    /// Synchronously initializes the Windows graphics topology layer by polling available APIs.
    fn acquire() -> DisplayResult<Self> {
        // Force raw physical pixel grids across all connected FFI communication boundaries
        ensure_dpi_aware();

        let hardware_map = load_hardware_map();
        let mut use_ccd = true;

        // Attempt to establish a valid session connection using the modern CCD subsystem
        let ccd_backend = CcdTopology::acquire().map(Some).unwrap_or_else(|_| {
            // Degrade gracefully to legacy tracking if CCD fails (e.g., inside an RDP session)
            use_ccd = false;
            None
        });

        // GDI is initialized unconditionally to guarantee a safe, reliable execution fallback
        let mut gdi_backend = GdiTopology::acquire()?;

        // Apply hardware map definitions to the initial GDI dataset
        enrich_gdi_topology(&mut gdi_backend, &hardware_map);

        Ok(WinDisplayManager {
            ccd: ccd_backend,
            gdi: gdi_backend,
            use_ccd,
            hardware_map,
        })
    }

    /// Exposes the active display states collected by the currently selected routing backend.
    fn get_outputs(&self) -> Vec<OutputState> {
        if self.use_ccd {
            if let Some(ref ccd_backend) = self.ccd {
                return ccd_backend.get_outputs();
            }
        }

        // Fallback to evaluating the current GDI topology map structures
        self.gdi.get_outputs()
    }

    /// Generates a mutable, chainable graphics parameter staging editor for a targeted display ID.
    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        if self.use_ccd {
            if let Some(ref mut ccd_backend) = self.ccd {
                return ccd_backend.edit_output(id);
            }
        }

        self.gdi.edit_output(id)
    }

    /// Flags whether configuration mutations must be written permanently to the system registry.
    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        if let Some(ref mut ccd_backend) = self.ccd {
            ccd_backend.set_persistence(enabled);
        }
        self.gdi.set_persistence(enabled);
        self
    }

    /// Tests the validity of staged display alterations without altering active hardware states.
    async fn validate(&self) -> DisplayResult<()> {
        if self.use_ccd {
            if let Some(ref ccd_backend) = self.ccd {
                // Delegate validation cycles directly to the modern kernel driver pipeline
                return ccd_backend.validate().await;
            }
        }

        // Legacy GDI does not natively support isolated dry-run validation steps
        Ok(())
    }

    /// Flushes all staged layout configurations down to the native Windows kernel layer.
    async fn commit(&mut self) -> DisplayResult<()> {
        if self.use_ccd {
            let mut ccd_backend = match self.ccd.take() {
                Some(backend) => backend,
                None => {
                    return Err(DisplayError::BackendError(
                        "CCD backend uninitialized".into(),
                    ))
                }
            };

            // CCD requires synchronous kernel interactions isolated onto a dedicated blocking thread
            let updated_ccd = task::spawn_blocking(move || -> DisplayResult<CcdTopology> {
                futures::executor::block_on(async {
                    <CcdTopology as UniversalTopology>::commit(&mut ccd_backend).await
                })?;
                Ok(ccd_backend)
            })
            .await
            .map_err(|e| {
                DisplayError::BackendError(format!("CCD blocking task execution panic: {}", e))
            })??;

            self.ccd = Some(updated_ccd);

            // Re-synchronize the GDI cache immediately following successful CCD mutations
            if let Ok(mut fresh_gdi) = GdiTopology::acquire() {
                // Use the hardware map within commit() to re-inject IDs into the newly generated cache
                enrich_gdi_topology(&mut fresh_gdi, &self.hardware_map);
                self.gdi = fresh_gdi;
            }
        } else {
            let mut gdi_backend = self.gdi.clone();

            // Execute legacy GDI subsystem resets safely within isolated blocking workers
            let updated_gdi = task::spawn_blocking(move || -> DisplayResult<GdiTopology> {
                futures::executor::block_on(async {
                    <GdiTopology as UniversalTopology>::commit(&mut gdi_backend).await
                })?;
                Ok(gdi_backend)
            })
            .await
            .map_err(|e| {
                DisplayError::BackendError(format!("GDI blocking task execution panic: {}", e))
            })??;

            self.gdi = updated_gdi;

            // Attempt to recover the CCD layer state following manual GDI framework rewrites
            if let Ok(fresh_ccd) = CcdTopology::acquire() {
                apply_gdi_state_to_ccd(&self.gdi, &fresh_ccd);
                self.ccd = Some(fresh_ccd);
                self.use_ccd = true;
            }

            // Re-enrich the local GDI state layer following the flush sequence
            enrich_gdi_topology(&mut self.gdi, &self.hardware_map);
        }

        Ok(())
    }
}
