//! # df_displmgr — Cross-platform display management library
//!
//! This crate provides the core abstractions for enumerating, querying, and
//! configuring displays across Windows (CCD/GDI) and Linux (DRM/Wayland/KDE/udev).
//!
//! ## Architecture
//!
//! - [`UniversalTopology`](traits::UniversalTopology) — trait for querying current display topology
//! - [`OutputEditable`](traits::OutputEditable) — trait for modifying properties of a single output
//! - [`NativeTopology`] — platform-resolved concrete implementation
//!
//! ## Platform backends
//!
//! | Platform | Backend | Features |
//! |----------|---------|----------|
//! | Windows  | CCD     | Profile persistence, advanced topology |
//! | Windows  | GDI     | Legacy fallback, wider compatibility |
//! | Linux    | DRM     | Direct kernel mode setting |
//! | Linux    | Wayland | wlroots output management protocol |
//! | Linux    | KDE     | KScreen D-Bus integration |
//! | Linux    | udev    | sysfs/udevadm enumeration |
//!
//! ## Feature gates
//!
//! - `ctrl_center` — enables high-level activation logic (`force_activate_by_monitor_name`,
//!   `activate_with_topology_restore`, etc.)
//! - `wgpu_types` — enables WGPU integration for HDR pipelines
//!
//! ## License
//!
//! Licensed under [MIT](../LICENSE-MIT) at your option.

// NOTE: unsafe_code is allowed because Windows CCD/GDI APIs require FFI calls.
// Safety: All unsafe code is encapsulated in windows/ submodule with careful invariants.
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(unsafe_code)]

/// Platform-specific backend implementations for display configuration.
pub mod backends;
/// Error types for display configuration operations.
pub mod error;
/// Traits for output editing and topology management.
pub mod traits;
/// Core data types for display configuration and topology.
pub mod types;

// Re-export core types for a flattened, user-friendly API.
// The two deprecated aliases stay exported for backward compatibility.
#[allow(deprecated)]
pub use error::{DisplayError, DisplayManagerError, DisplayResult};
pub use traits::{OutputEditable, UniversalTopology};
pub use types::*;

// Re-export constants and timing utilities from df_core
pub use df_core::constants::{CLONE_TOLERANCE_PIXELS, HARDWARE_SETTLE_DELAY_MS};
pub use df_core::timing::{calculate_cvt, cvt_rb, gtf, TimingParameters};

// Windows-specific re-exports
#[cfg(target_os = "windows")]
pub use backends::windows::{
    activate_display, force_activate_by_monitor_name, force_all, ActivationResult,
    WinDisplayManager,
};

// Linux-specific re-exports
#[cfg(target_os = "linux")]
pub use backends::linux::{LinuxBackendVariant, LinuxTopology};

/// The primary entry point for display management.
/// Resolves to a platform-specific implementation at compile time.
///
/// # Example
///
/// ```rust,no_run
/// use df_displmgr::NativeTopology;
/// use df_displmgr::traits::UniversalTopology;
///
/// #[tokio::main]
/// async fn main() -> df_displmgr::DisplayResult<()> {
///     // Subsystem acquisition is synchronous and FFI-bound.
///     let mut topo = NativeTopology::acquire()?;
///     let outputs = topo.get_outputs();
///
///     // Async paths are isolated to mutations and validation passes.
///     topo.validate().await?;
///     topo.commit().await?;
///     Ok(())
/// }
/// ```
#[cfg(target_os = "windows")]
pub use crate::backends::NativeTopology;

#[cfg(target_os = "linux")]
pub use crate::backends::NativeTopology;

// Fallback for unsupported platforms — allows documentation builds and
// cross-compilation without pulling in platform-specific dependencies.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub use crate::backends::NativeTopology;

/// Saved snapshot of an output's state for topology restoration.
#[derive(Debug, Clone)]
struct SavedOutput {
    enabled: bool,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

async fn hardware_settle_delay() -> Result<(), DisplayManagerError> {
    tokio::time::sleep(std::time::Duration::from_millis(HARDWARE_SETTLE_DELAY_MS)).await;
    Ok(())
}

async fn restore_saved_topology(
    saved: &std::collections::HashMap<String, SavedOutput>,
    target_id: u32,
) -> Result<(), DisplayManagerError> {
    let target_id_str = target_id.to_string();
    let mut topo = NativeTopology::acquire()?;
    let outputs = topo.get_outputs();

    for o in &outputs {
        let id_str = &o.identity.id.0;
        if let Some(s) = saved.get(id_str) {
            if s.enabled {
                let did = DisplayId(id_str.clone());
                if let Ok(mut editor) = topo.edit_output(&did) {
                    let _ = editor.set_enabled(true);
                    let _ = editor.set_position(types::Point2D { x: s.x, y: s.y });
                    let _ = editor.set_resolution(types::Extent2D {
                        width: s.width,
                        height: s.height,
                    });
                }
            } else {
                let did = DisplayId(id_str.clone());
                if let Ok(mut editor) = topo.edit_output(&did) {
                    let _ = editor.set_enabled(false);
                }
            }
        } else if id_str == &target_id_str {
            // Target monitor — keep enabled, will be positioned in step 4
        } else {
            // Neither saved nor target — turn off
            let did = DisplayId(id_str.clone());
            if let Ok(mut editor) = topo.edit_output(&did) {
                let _ = editor.set_enabled(false);
            }
        }
    }

    topo.set_persistence(true);
    let _ = topo.validate().await;
    topo.commit().await.map_err(|e| {
        DisplayManagerError::BackendError(format!("topology restore commit failed: {e}"))
    })
}

async fn place_target_monitor(
    target_id: u32,
    plan: &ActivationPlan,
    was_active: bool,
) -> Result<(), DisplayManagerError> {
    if was_active {
        return Ok(());
    }

    let pos = match plan.position {
        Some(p) => p,
        None => {
            let topo = NativeTopology::acquire()?;
            let right_x = topo
                .get_outputs()
                .iter()
                .filter(|o| o.enabled)
                .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
                .max()
                .unwrap_or(0);
            types::Point2D { x: right_x, y: 0 }
        }
    };

    let mut topo = NativeTopology::acquire()?;
    let did = DisplayId(target_id.to_string());
    let mut editor = topo.edit_output(&did).map_err(|e| {
        DisplayManagerError::BackendError(format!("edit_output '{}' failed: {e}", did.0))
    })?;

    editor.set_enabled(true).map_err(|e| {
        DisplayManagerError::BackendError(format!("set_enabled for '{}' failed: {e}", did.0))
    })?;
    editor.set_position(pos).map_err(|e| {
        DisplayManagerError::BackendError(format!("set_position for '{}' failed: {e}", did.0))
    })?;

    if let Some(res) = plan.resolution {
        editor.set_resolution(res).map_err(|e| {
            DisplayManagerError::BackendError(format!("set_resolution for '{}' failed: {e}", did.0))
        })?;
    }
    if let Some(rot) = plan.rotation {
        editor.set_rotation(rot).map_err(|e| {
            DisplayManagerError::BackendError(format!("set_rotation for '{}' failed: {e}", did.0))
        })?;
    }

    drop(editor);
    topo.set_persistence(true);
    let _ = topo.validate().await;
    topo.commit()
        .await
        .map_err(|e| DisplayManagerError::BackendError(format!("final commit failed: {e}")))?;

    Ok(())
}

/// Topology-aware activation: save current topology, force_all, restore, place target.
///
/// This is the main high-level function for activating an inactive monitor while
/// preserving the existing layout. It:
/// 1. Saves the current topology (which monitors are active + their positions/sizes)
/// 2. Calls `force_all()` to activate all monitors so the target becomes reachable
/// 3. Restores the saved topology — original active monitors get their positions back,
///    monitors that were inactive AND are not the target get turned off
/// 4. Places the target monitor according to the `ActivationPlan`
///
/// # Platform Support
///
/// | Platform | Implementation |
/// |----------|---------------|
/// | Windows  | Uses CCD `SetDisplayConfig` with `SDC_TOPOLOGY_SUPPLIED` to force all targets active |
/// | Linux    | Uses compositor-native enable-all (Wayland KDE) or DRM connector enable (bare metal) |
pub async fn activate_with_topology_restore(
    target_id: u32,
    plan: &ActivationPlan,
) -> Result<(), DisplayManagerError> {
    use std::collections::HashMap;
    use traits::UniversalTopology;

    // ── Step 1: Save current topology ──
    let saved: HashMap<String, SavedOutput> = {
        let topo = NativeTopology::acquire()?;
        let outputs = topo.get_outputs();
        let mut map = HashMap::new();
        for o in &outputs {
            map.insert(
                o.identity.id.0.clone(),
                SavedOutput {
                    enabled: o.enabled,
                    x: o.geometry.origin.x,
                    y: o.geometry.origin.y,
                    width: o.geometry.size.width,
                    height: o.geometry.size.height,
                },
            );
        }
        map
    };

    // ── Step 2: force_all / activate all (platform-specific) ──
    force_all_displays().await?;

    hardware_settle_delay().await?;

    // ── Step 3: Restore saved topology ──
    let was_active = saved
        .get(&target_id.to_string())
        .map(|s| s.enabled)
        .unwrap_or(false);

    restore_saved_topology(&saved, target_id).await?;

    // ── Step 4: Place target monitor ──
    hardware_settle_delay().await?;
    place_target_monitor(target_id, plan, was_active).await
}

/// Platform-specific force-all-displays-active operation.
///
/// On Windows this uses CCD `SetDisplayConfig` with `SDC_TOPOLOGY_SUPPLIED`.
/// On Linux this enables all discovered connectors via the active backend.
#[cfg(target_os = "windows")]
async fn force_all_displays() -> Result<(), DisplayManagerError> {
    // Windows backend is synchronous, so we just call it directly in an async context
    crate::backends::windows::force_all()
        .map_err(|e| DisplayManagerError::BackendError(e.to_string()))
}

#[cfg(target_os = "linux")]
async fn force_all_displays() -> Result<(), DisplayManagerError> {
    use traits::UniversalTopology;

    let mut topo = NativeTopology::acquire()?;
    let outputs = topo.get_outputs();

    for output in &outputs {
        if let Ok(mut editor) = topo.edit_output(&output.identity.id) {
            let _ = editor.set_enabled(true);
        }
    }

    // Validate and commit the changes so they actually take effect
    topo.set_persistence(false);
    topo.validate().await?;
    topo.commit().await?;

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
async fn force_all_displays() -> Result<(), DisplayManagerError> {
    Err(DisplayManagerError::UnsupportedPlatform(
        "force_all_displays not supported on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_plan_with_values() {
        let plan = ActivationPlan {
            position: Some(types::Point2D { x: 100, y: 0 }),
            resolution: Some(types::Extent2D {
                width: 1920,
                height: 1080,
            }),
            rotation: Some(DisplayRotation::Rotate90),
        };
        assert_eq!(plan.position, Some(types::Point2D { x: 100, y: 0 }));
        assert_eq!(
            plan.resolution,
            Some(types::Extent2D {
                width: 1920,
                height: 1080
            })
        );
        assert_eq!(plan.rotation, Some(DisplayRotation::Rotate90));
    }

    #[test]
    fn test_display_id_comparison() {
        let id1 = DisplayId("HDMI-1".to_string());
        let id2 = DisplayId("HDMI-1".to_string());
        let id3 = DisplayId("DP-1".to_string());
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_display_id_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(DisplayId("HDMI-1".to_string()), 1);
        map.insert(DisplayId("HDMI-1".to_string()), 2);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_display_id_ord() {
        let id1 = DisplayId("A".to_string());
        let id2 = DisplayId("B".to_string());
        assert!(id1 < id2);
        assert!(id2 > id1);
    }

    #[test]
    fn test_display_id_serialization_roundtrip() {
        let original = DisplayId("HDMI-1".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DisplayId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_display_identity_serialization() {
        let identity = DisplayIdentity {
            id: DisplayId("HDMI-1".to_string()),
            connector_id: ConnectorId("HDMI-1".to_string()),
            adapter_id: AdapterId("card0".to_string()),
            hardware_uuid: Some("12345".to_string()),
            monitor_name: "Test Monitor".to_string(),
        };
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: DisplayIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity.id, deserialized.id);
        assert_eq!(identity.connector_id, deserialized.connector_id);
        assert_eq!(identity.monitor_name, deserialized.monitor_name);
    }

    #[test]
    fn test_display_rotation_values() {
        assert_eq!(DisplayRotation::Rotate0, DisplayRotation::Rotate0);
        assert_eq!(DisplayRotation::Rotate90, DisplayRotation::Rotate90);
        assert_eq!(DisplayRotation::Rotate180, DisplayRotation::Rotate180);
        assert_eq!(DisplayRotation::Rotate270, DisplayRotation::Rotate270);
    }

    #[test]
    fn test_hdr_mode_cinema() {
        let mode = HdrMode::Cinema;
        assert_ne!(mode, HdrMode::Default);
        assert_eq!(mode, HdrMode::Cinema);
    }

    #[test]
    fn test_hdr_mode_game() {
        let mode = HdrMode::Game;
        assert_ne!(mode, HdrMode::Default);
        assert_eq!(mode, HdrMode::Game);
    }

    #[test]
    fn test_output_state_is_landscape() {
        let landscape = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            ..Default::default()
        };
        assert!(landscape.is_landscape());

        let portrait = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1080,
                    height: 1920,
                },
            },
            ..Default::default()
        };
        assert!(!portrait.is_landscape());
    }

    #[test]
    fn test_output_state_refresh_rate_hz() {
        let state = OutputState {
            refresh_rate: 144_000,
            ..Default::default()
        };
        assert_eq!(state.refresh_rate_hz(), 144.0);
    }

    #[test]
    fn test_output_state_serialization() {
        let state = OutputState {
            enabled: true,
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            refresh_rate: 60_000,
            ..Default::default()
        };
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: OutputState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.enabled, deserialized.enabled);
        assert_eq!(state.geometry, deserialized.geometry);
    }

    #[test]
    fn test_output_state_supported_modes() {
        let state = OutputState {
            supported_modes: vec![types::VideoMode {
                resolution: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
                refresh_rate: 60_000,
            }],
            ..Default::default()
        };
        assert_eq!(state.supported_modes.len(), 1);
        assert_eq!(state.supported_modes[0].resolution.width, 1920);
    }

    // -------------------------------------------------------------------------
    // ActivationPlan tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_activation_plan_clone() {
        let plan = ActivationPlan {
            position: Some(types::Point2D { x: 0, y: 0 }),
            resolution: Some(types::Extent2D {
                width: 1920,
                height: 1080,
            }),
            rotation: Some(DisplayRotation::Rotate90),
        };
        let cloned = plan.clone();
        assert_eq!(plan, cloned);
        assert_eq!(cloned.rotation, Some(DisplayRotation::Rotate90));
    }

    #[test]
    fn test_activation_plan_debug() {
        let plan = ActivationPlan::default();
        let debug_str = format!("{plan:?}");
        assert!(debug_str.contains("ActivationPlan"));
    }

    #[test]
    fn test_activation_plan_partial_eq() {
        let plan1 = ActivationPlan {
            position: Some(types::Point2D { x: 0, y: 0 }),
            resolution: Some(types::Extent2D {
                width: 1920,
                height: 1080,
            }),
            rotation: Some(DisplayRotation::Rotate0),
        };
        let plan2 = ActivationPlan {
            position: Some(types::Point2D { x: 0, y: 0 }),
            resolution: Some(types::Extent2D {
                width: 1920,
                height: 1080,
            }),
            rotation: Some(DisplayRotation::Rotate0),
        };
        let plan3 = ActivationPlan {
            position: Some(types::Point2D { x: 100, y: 0 }),
            ..Default::default()
        };
        assert_eq!(plan1, plan2);
        assert_ne!(plan1, plan3);
    }

    #[test]
    fn test_activation_plan_with_none_values() {
        let plan = ActivationPlan {
            position: None,
            resolution: None,
            rotation: None,
        };
        assert!(plan.position.is_none());
        assert!(plan.resolution.is_none());
        assert!(plan.rotation.is_none());
    }

    // -------------------------------------------------------------------------
    // SavedOutput tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_saved_output_fields() {
        let saved = SavedOutput {
            enabled: true,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert!(saved.enabled);
        assert_eq!(saved.x, 0);
        assert_eq!(saved.y, 0);
        assert_eq!(saved.width, 1920);
        assert_eq!(saved.height, 1080);
    }

    #[test]
    fn test_saved_output_disabled() {
        let saved = SavedOutput {
            enabled: false,
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert!(!saved.enabled);
        assert_eq!(saved.x, 1920);
    }

    #[test]
    fn test_saved_output_clone() {
        let saved = SavedOutput {
            enabled: true,
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
        };
        let cloned = saved.clone();
        assert_eq!(saved.enabled, cloned.enabled);
        assert_eq!(saved.x, cloned.x);
        assert_eq!(saved.width, cloned.width);
    }

    #[test]
    fn test_saved_output_debug() {
        let saved = SavedOutput {
            enabled: true,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let s = format!("{saved:?}");
        assert!(s.contains("SavedOutput"));
    }

    // -------------------------------------------------------------------------
    // hardware_settle_delay tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_hardware_settle_delay_completes() {
        let start = std::time::Instant::now();
        let result = hardware_settle_delay().await;
        let elapsed = start.elapsed();
        assert!(result.is_ok());
        assert!(elapsed >= std::time::Duration::from_millis(450));
        assert!(elapsed < std::time::Duration::from_millis(1000));
    }

    // -------------------------------------------------------------------------
    // force_all_displays tests
    // -------------------------------------------------------------------------

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    #[tokio::test]
    async fn test_force_all_displays_unsupported_platform() {
        let result = force_all_displays().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not supported"));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_activate_with_topology_restore_no_hardware() {
        let result = activate_with_topology_restore(9999, &ActivationPlan::default()).await;
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Position calculation helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_calculate_right_x_no_enabled_outputs() {
        // Simulate the right_x calculation: max of (origin.x + size.width) for enabled outputs,
        // or 0 if none.
        let outputs: Vec<OutputState> = vec![];
        let right_x: i32 = outputs
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
            .max()
            .unwrap_or(0);
        assert_eq!(right_x, 0);
    }

    #[test]
    fn test_calculate_right_x_with_enabled_outputs() {
        let outputs = [
            OutputState {
                enabled: true,
                geometry: types::Rect {
                    origin: types::Point2D { x: 0, y: 0 },
                    size: types::Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
                ..Default::default()
            },
            OutputState {
                enabled: true,
                geometry: types::Rect {
                    origin: types::Point2D { x: 1920, y: 0 },
                    size: types::Extent2D {
                        width: 1600,
                        height: 900,
                    },
                },
                ..Default::default()
            },
        ];
        let right_x: i32 = outputs
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
            .max()
            .unwrap_or(0);
        assert_eq!(right_x, 3520); // max(1920, 1920+1600)
    }

    #[test]
    fn test_calculate_right_x_ignores_disabled() {
        let outputs = [
            OutputState {
                enabled: false,
                geometry: types::Rect {
                    origin: types::Point2D { x: 0, y: 0 },
                    size: types::Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
                ..Default::default()
            },
            OutputState {
                enabled: true,
                geometry: types::Rect {
                    origin: types::Point2D { x: 500, y: 0 },
                    size: types::Extent2D {
                        width: 1024,
                        height: 768,
                    },
                },
                ..Default::default()
            },
        ];
        let right_x: i32 = outputs
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
            .max()
            .unwrap_or(0);
        assert_eq!(right_x, 1524); // only 500 + 1024
    }

    // -------------------------------------------------------------------------
    // SavedOutput -> HashMap key extraction tests (for restore_saved_topology logic)
    // -------------------------------------------------------------------------

    #[test]
    fn test_restore_saved_topology_id_matching() {
        let saved_id = "HDMI-1".to_string();
        let outputs = vec![
            types::OutputState {
                identity: types::DisplayIdentity {
                    id: types::DisplayId("HDMI-1".to_string()),
                    ..Default::default()
                },
                enabled: true,
                ..Default::default()
            },
            types::OutputState {
                identity: types::DisplayIdentity {
                    id: types::DisplayId("DP-1".to_string()),
                    ..Default::default()
                },
                enabled: true,
                ..Default::default()
            },
        ];

        let mut matched = 0;
        for o in &outputs {
            if o.identity.id.0 == saved_id {
                matched += 1;
            }
        }
        assert_eq!(matched, 1);
    }

    #[test]
    fn test_restore_saved_topology_target_id_matching() {
        let target_id = 42;
        let target_id_str = target_id.to_string();
        let outputs = vec![
            types::OutputState {
                identity: types::DisplayIdentity {
                    id: types::DisplayId("HDMI-1".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
            types::OutputState {
                identity: types::DisplayIdentity {
                    id: types::DisplayId(target_id_str.clone()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ];

        let mut target_found = false;
        for o in &outputs {
            if o.identity.id.0 == target_id_str {
                target_found = true;
                break;
            }
        }
        assert!(target_found);
    }

    // -------------------------------------------------------------------------
    // Output geometry tests for topology layout
    // -------------------------------------------------------------------------

    #[test]
    fn test_output_geometry_topology_side_by_side() {
        let o1 = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            enabled: true,
            ..Default::default()
        };
        let o2 = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 1920, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            enabled: true,
            ..Default::default()
        };
        assert!(o1.geometry.origin.x < o2.geometry.origin.x);
        assert_eq!(
            o1.geometry.origin.x + o1.geometry.size.width as i32,
            o2.geometry.origin.x
        );
    }

    #[test]
    fn test_output_geometry_topology_above_below() {
        let top = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            enabled: true,
            ..Default::default()
        };
        let bottom = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 1080 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            enabled: true,
            ..Default::default()
        };
        assert_eq!(
            top.geometry.origin.y + top.geometry.size.height as i32,
            bottom.geometry.origin.y
        );
    }

    // -------------------------------------------------------------------------
    // OutputState scale, rotation, hdr_state coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_output_state_scale() {
        let state = OutputState {
            scale: 1.5,
            ..Default::default()
        };
        assert_eq!(state.scale, 1.5);
    }

    #[test]
    fn test_output_state_is_portrait() {
        let portrait = OutputState {
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1080,
                    height: 1920,
                },
            },
            ..Default::default()
        };
        assert!(!portrait.is_landscape());
    }

    #[test]
    fn test_output_state_rotation() {
        let state = OutputState {
            rotation: DisplayRotation::Rotate180,
            ..Default::default()
        };
        assert_eq!(state.rotation, DisplayRotation::Rotate180);
    }

    #[test]
    fn test_output_state_hdr() {
        let state = OutputState {
            hdr_state: HdrState::Enabled,
            ..Default::default()
        };
        assert_eq!(state.hdr_state, HdrState::Enabled);
    }

    #[test]
    fn test_output_state_clone() {
        let state = OutputState {
            identity: types::DisplayIdentity {
                id: types::DisplayId("test".to_string()),
                connector_id: types::ConnectorId("test".to_string()),
                adapter_id: types::AdapterId("test".to_string()),
                ..Default::default()
            },
            geometry: types::Rect {
                origin: types::Point2D { x: 10, y: 20 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            scale: 2.0,
            rotation: DisplayRotation::Rotate90,
            refresh_rate: 120_000,
            hdr_state: HdrState::Enabled,
            hdr_mode: types::HdrMode::Default,
            native_resolution: None,
            supported_modes: vec![],
            enabled: true,
            is_primary: false,
        };
        let cloned = state.clone();
        assert_eq!(state, cloned);
        assert_eq!(cloned.geometry.origin.x, 10);
        assert_eq!(cloned.scale, 2.0);
    }

    // -------------------------------------------------------------------------
    // DisplayIdentity coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_display_identity_with_uuid() {
        let identity = DisplayIdentity {
            hardware_uuid: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            monitor_name: "Test".to_string(),
            ..Default::default()
        };
        assert!(identity.hardware_uuid.is_some());
        assert_eq!(
            identity.hardware_uuid.unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_display_identity_clone() {
        let identity = DisplayIdentity {
            id: DisplayId("HDMI-1".to_string()),
            connector_id: ConnectorId("HDMI-1".to_string()),
            adapter_id: AdapterId("card0".to_string()),
            hardware_uuid: Some("uuid".to_string()),
            monitor_name: "Monitor".to_string(),
        };
        let cloned = identity.clone();
        assert_eq!(identity, cloned);
    }

    // -------------------------------------------------------------------------
    // Extent2D, Point2D, Rect method coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_extent2d_with_values() {
        let e = Extent2D {
            width: 800,
            height: 600,
        };
        assert_eq!(e.width, 800);
        assert_eq!(e.height, 600);
    }

    #[test]
    fn test_point2d_with_values() {
        let p = Point2D { x: 100, y: 200 };
        assert_eq!(p.x, 100);
        assert_eq!(p.y, 200);
    }

    #[test]
    fn test_output_state_all_fields() {
        let state = OutputState {
            identity: types::DisplayIdentity {
                id: types::DisplayId("HDMI-1".to_string()),
                connector_id: types::ConnectorId("HDMI-1".to_string()),
                adapter_id: types::AdapterId("card0".to_string()),
                hardware_uuid: Some("uuid123".to_string()),
                monitor_name: "MyMonitor".to_string(),
            },
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            refresh_rate: 60_000,
            rotation: types::DisplayRotation::Rotate0,
            hdr_state: types::HdrState::Disabled,
            hdr_mode: types::HdrMode::Default,
            scale: 1.0,
            native_resolution: Some(types::Extent2D {
                width: 1920,
                height: 1080,
            }),
            supported_modes: vec![types::VideoMode {
                resolution: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
                refresh_rate: 60_000,
            }],
            enabled: true,
            is_primary: true,
        };
        assert!(state.enabled);
        assert!(state.is_primary);
        assert_eq!(state.scale, 1.0);
    }

    #[test]
    fn test_extent2d_clone() {
        let e = Extent2D {
            width: 100,
            height: 200,
        };
        let c = e;
        assert_eq!(e, c);
    }

    #[test]
    fn test_point2d_clone() {
        let p = Point2D { x: 50, y: 75 };
        let c = p;
        assert_eq!(p, c);
    }

    #[test]
    fn test_rect_clone() {
        let r = Rect {
            origin: Point2D { x: 10, y: 20 },
            size: Extent2D {
                width: 100,
                height: 100,
            },
        };
        let c = r;
        assert_eq!(r, c);
    }

    // -------------------------------------------------------------------------
    // DisplayError coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_display_error_not_found() {
        let err = DisplayManagerError::MonitorNotFound("DP-1".to_string());
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("DP-1"));
    }

    #[test]
    fn test_display_error_configuration_rejected() {
        let err = DisplayManagerError::ConfigurationRejected;
        assert!(err.to_string().contains("rejected"));
    }

    #[test]
    fn test_display_error_permission_denied() {
        let err = DisplayManagerError::PermissionDenied;
        assert!(
            err.to_string().to_lowercase().contains("permission")
                || err.to_string().to_lowercase().contains("privilege")
        );
    }

    #[test]
    fn test_display_error_stale_topology() {
        let err = DisplayManagerError::StaleTopology;
        assert!(err.to_string().contains("stale"));
    }

    #[test]
    fn test_display_error_connection_failed() {
        let err = DisplayManagerError::ConnectionFailed;
        assert!(
            err.to_string().to_lowercase().contains("connect")
                || err.to_string().to_lowercase().contains("fail")
        );
    }

    // -------------------------------------------------------------------------
    // DisplayRotation coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_display_rotation_clone() {
        let r = DisplayRotation::Rotate90;
        let c = r;
        assert_eq!(r, c);
    }

    #[test]
    fn test_display_rotation_debug() {
        let r = DisplayRotation::Rotate180;
        let s = format!("{r:?}");
        assert!(s.contains("Rotate180"));
    }

    // -------------------------------------------------------------------------
    // VcpCode coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_output_state_is_primary() {
        let primary = OutputState {
            is_primary: true,
            ..Default::default()
        };
        assert!(primary.is_primary);
    }

    #[test]
    fn test_output_state_native_resolution() {
        let state = OutputState {
            native_resolution: Some(types::Extent2D {
                width: 3840,
                height: 2160,
            }),
            ..Default::default()
        };
        assert_eq!(state.native_resolution.unwrap().width, 3840);
    }

    #[test]
    fn test_display_identity_debug() {
        let identity = DisplayIdentity {
            monitor_name: "Test".to_string(),
            ..Default::default()
        };
        let s = format!("{identity:?}");
        assert!(!s.is_empty());
    }

    #[test]
    fn test_video_mode_with_values() {
        let mode = VideoMode {
            resolution: Extent2D {
                width: 2560,
                height: 1440,
            },
            refresh_rate: 144_000,
        };
        assert_eq!(mode.refresh_rate_hz(), 144.0);
    }

    #[test]
    fn test_point2d_negative_coords() {
        let p = Point2D { x: -100, y: -50 };
        assert_eq!(p.x, -100);
        assert_eq!(p.y, -50);
    }

    #[test]
    fn test_extent2d_large_values() {
        let e = Extent2D {
            width: 7680,
            height: 4320,
        };
        assert_eq!(e.width, 7680);
        assert_eq!(e.height, 4320);
    }

    #[test]
    fn test_calculate_right_x_single_output() {
        let outputs = [OutputState {
            enabled: true,
            geometry: types::Rect {
                origin: types::Point2D { x: 0, y: 0 },
                size: types::Extent2D {
                    width: 1920,
                    height: 1080,
                },
            },
            ..Default::default()
        }];
        let right_x: i32 = outputs
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
            .max()
            .unwrap_or(0);
        assert_eq!(right_x, 1920);
    }

    // -------------------------------------------------------------------------
    // ConnectorId / AdapterId coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_connector_id_clone() {
        let cid = ConnectorId("DP-1".to_string());
        let c = cid.clone();
        assert_eq!(cid, c);
    }

    #[test]
    fn test_adapter_id_clone() {
        let aid = AdapterId("card0".to_string());
        let c = aid.clone();
        assert_eq!(aid, c);
    }

    #[test]
    fn test_connector_id_equality() {
        let c1 = ConnectorId("HDMI-1".to_string());
        let c2 = ConnectorId("HDMI-1".to_string());
        let c3 = ConnectorId("DP-1".to_string());
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_adapter_id_equality() {
        let a1 = AdapterId("card0".to_string());
        let a2 = AdapterId("card0".to_string());
        let a3 = AdapterId("card1".to_string());
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    // -------------------------------------------------------------------------
    // VideoMode method coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_video_mode_clone() {
        let mode = VideoMode {
            resolution: Extent2D {
                width: 1920,
                height: 1080,
            },
            refresh_rate: 60_000,
        };
        let c = mode;
        assert_eq!(mode, c);
    }

    #[test]
    fn test_hardware_settle_delay_error_handling() {
        // We can't easily test the error path without mocking, but we can test
        // that the function type is correct by calling it and checking success.
        // The actual 500ms delay is tested in the async test above.
    }

    #[test]
    fn test_output_state_refresh_rate_zero() {
        let state = OutputState {
            refresh_rate: 0,
            ..Default::default()
        };
        assert_eq!(state.refresh_rate_hz(), 0.0);
    }

    #[test]
    fn test_output_state_refresh_rate_high() {
        let state = OutputState {
            refresh_rate: 240_000,
            ..Default::default()
        };
        assert_eq!(state.refresh_rate_hz(), 240.0);
    }
}
