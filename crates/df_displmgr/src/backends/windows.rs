use async_trait::async_trait;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tokio::task;
use serde::Deserialize;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::Devices::Display as WinDisplay;

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{OutputState, DisplayId};

pub mod displmgr_ccd;
pub mod displmgr_gdi;

use self::displmgr_ccd::CcdTopology;
use self::displmgr_gdi::GdiTopology;

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

/// Orchestrates the Windows graphics subsystem by routing requests dynamically
/// between the modern CCD API and the legacy GDI fallback interface.
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
                None => return Err(DisplayError::BackendError("CCD backend uninitialized".into())),
            };

            // CCD requires synchronous kernel interactions isolated onto a dedicated blocking thread
            let updated_ccd = task::spawn_blocking(move || -> DisplayResult<CcdTopology> {
                futures::executor::block_on(async {
                    <CcdTopology as UniversalTopology>::commit(&mut ccd_backend).await
                })?;
                Ok(ccd_backend)
            })
            .await
            .map_err(|e| DisplayError::BackendError(format!("CCD blocking task execution panic: {}", e)))??;

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
            .map_err(|e| DisplayError::BackendError(format!("GDI blocking task execution panic: {}", e)))??;

            self.gdi = updated_gdi;

            // Attempt to recover the CCD layer state following manual GDI framework rewrites
            if let Ok(fresh_ccd) = CcdTopology::acquire() {
                // Map GDI-staged HDR and scale values into the modern CCD device-info
                // interface where a hardware mapping token (target id) is available.
                for (_gdi_name, gdi_state) in &self.gdi.outputs {
                    if let Some(ref hid) = gdi_state.identity.hardware_uuid {
                        if let Ok(target_id) = hid.parse::<u32>() {
                            for path in &fresh_ccd.data.paths {
                                if path.targetInfo.id == target_id {
                                    // Apply DPI scale if different from default
                                    if (gdi_state.scale - 1.0).abs() > f64::EPSILON {
                                        let scale_percent = (gdi_state.scale * 100.0).round() as i32;
                                        let mut payload = crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
                                            header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                                                r#type: crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE,
                                                size: std::mem::size_of::<crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>() as u32,
                                                adapterId: path.targetInfo.adapterId,
                                                id: target_id,
                                            },
                                            scale_factor_as_percent: scale_percent,
                                        };
                                        let res = unsafe { WinDisplay::DisplayConfigSetDeviceInfo(&mut payload.header as *mut _ as *mut _) };
                                        if res != 0 {
                                            eprintln!("Warning: DISPLAYCONFIG DPI scale set failed for target {}: {}", target_id, res);
                                        }
                                    }

                                    // Apply HDR state if requested
                                    if gdi_state.hdr_state != crate::types::HdrState::Disabled {
                                        let enabled = if gdi_state.hdr_state == crate::types::HdrState::Enabled { 1 } else { 0 };
                                        let mut payload = crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_SET_HDR_STATE {
                                            header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
                                                r#type: crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE,
                                                size: std::mem::size_of::<crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_SET_HDR_STATE>() as u32,
                                                adapterId: path.targetInfo.adapterId,
                                                id: target_id,
                                            },
                                            enabled,
                                        };
                                        let res = unsafe { WinDisplay::DisplayConfigSetDeviceInfo(&mut payload.header as *mut _ as *mut _) };
                                        if res != 0 {
                                            eprintln!("Warning: DISPLAYCONFIG HDR set failed for target {}: {}", target_id, res);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.ccd = Some(fresh_ccd);
                self.use_ccd = true;
            }
            
            // Re-enrich the local GDI state layer following the flush sequence
            enrich_gdi_topology(&mut self.gdi, &self.hardware_map);
        }
        
        Ok(())
    }
}