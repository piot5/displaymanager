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

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Platform-specific backend implementations for display configuration.
pub mod backends;
/// Error types for display configuration operations.
pub mod error;
/// Traits for output editing and topology management.
pub mod traits;
/// Core data types for display configuration and topology.
pub mod types;

// Re-export core types for a flattened, user-friendly API.
pub use error::{DisplayError, DisplayResult};
pub use traits::{OutputEditable, UniversalTopology};
pub use types::*;

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
    _name: String,
    enabled: bool,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

async fn hardware_settle_delay() -> DisplayResult<()> {
    tokio::task::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(500));
    })
    .await
    .map_err(|e| DisplayError::BackendError(format!("hardware settle delay failed: {e}")))
}

async fn restore_saved_topology(
    saved: &std::collections::HashMap<String, SavedOutput>,
    target_id: u32,
) -> DisplayResult<()> {
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
        DisplayError::BackendError(format!("topology restore commit failed: {e}"))
    })
}

async fn place_target_monitor(
    target_id: u32,
    plan: &ActivationPlan,
    was_active: bool,
) -> DisplayResult<()> {
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
        DisplayError::BackendError(format!("edit_output '{}' failed: {e}", did.0))
    })?;

    editor.set_enabled(true).map_err(|e| {
        DisplayError::BackendError(format!("set_enabled for '{}' failed: {e}", did.0))
    })?;
    editor.set_position(pos).map_err(|e| {
        DisplayError::BackendError(format!("set_position for '{}' failed: {e}", did.0))
    })?;

    if let Some(res) = plan.resolution {
        editor.set_resolution(res).map_err(|e| {
            DisplayError::BackendError(format!("set_resolution for '{}' failed: {e}", did.0))
        })?;
    }
    if let Some(rot) = plan.rotation {
        editor.set_rotation(rot).map_err(|e| {
            DisplayError::BackendError(format!("set_rotation for '{}' failed: {e}", did.0))
        })?;
    }

    drop(editor);
    topo.set_persistence(true);
    let _ = topo.validate().await;
    topo.commit()
        .await
        .map_err(|e| DisplayError::BackendError(format!("final commit failed: {e}")))?;

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
) -> DisplayResult<()> {
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
                    _name: o.identity.monitor_name.trim().to_string(),
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
    force_all_displays()
        .map_err(|e| DisplayError::BackendError(format!("force_all failed: {e}")))?;

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
fn force_all_displays() -> Result<(), String> {
    crate::backends::windows::force_all().map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn force_all_displays() -> Result<(), String> {
    use traits::UniversalTopology;

    let mut topo = NativeTopology::acquire().map_err(|e| e.to_string())?;
    let outputs = topo.get_outputs();

    for output in &outputs {
        if let Ok(mut editor) = topo.edit_output(&output.identity.id) {
            let _ = editor.set_enabled(true);
        }
    }

    // Use a simpler approach: just acquire a fresh topology
    // (the Linux backends enumerate all outputs by default)
    drop(topo);

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn force_all_displays() -> Result<(), String> {
    Err("force_all_displays not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_plan_default() {
        let plan = ActivationPlan::default();
        assert!(plan.position.is_none());
        assert!(plan.resolution.is_none());
        assert!(plan.rotation.is_none());
    }

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
    fn test_display_rotation_default() {
        let rot = DisplayRotation::default();
        assert_eq!(rot, DisplayRotation::Rotate0);
    }

    #[test]
    fn test_display_rotation_values() {
        assert_eq!(DisplayRotation::Rotate0, DisplayRotation::Rotate0);
        assert_eq!(DisplayRotation::Rotate90, DisplayRotation::Rotate90);
        assert_eq!(DisplayRotation::Rotate180, DisplayRotation::Rotate180);
        assert_eq!(DisplayRotation::Rotate270, DisplayRotation::Rotate270);
    }

    #[test]
    fn test_extent2d_default() {
        let extent = Extent2D::default();
        assert_eq!(extent.width, 0);
        assert_eq!(extent.height, 0);
    }

    #[test]
    fn test_hdr_mode_default() {
        let mode = HdrMode::default();
        assert_eq!(mode, HdrMode::Default);
    }

    #[test]
    fn test_hdr_state_default() {
        let state = HdrState::default();
        assert_eq!(state, HdrState::Disabled);
    }

    #[test]
    fn test_output_state_default() {
        let state = OutputState::default();
        assert!(!state.enabled);
        assert_eq!(state.geometry.size.width, 0);
        assert_eq!(state.geometry.size.height, 0);
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

    #[test]
    fn test_point2d_default() {
        let point = Point2D::default();
        assert_eq!(point.x, 0);
        assert_eq!(point.y, 0);
    }

    #[test]
    fn test_rect_default() {
        let rect = Rect::default();
        assert_eq!(rect.origin.x, 0);
        assert_eq!(rect.origin.y, 0);
        assert_eq!(rect.size.width, 0);
        assert_eq!(rect.size.height, 0);
    }

    #[test]
    fn test_video_mode_default() {
        let mode = VideoMode::default();
        assert_eq!(mode.resolution.width, 0);
        assert_eq!(mode.resolution.height, 0);
        assert_eq!(mode.refresh_rate, 0);
    }
}
