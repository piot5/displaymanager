//! # df_displmgr — Cross-platform display management library
//!
//! This crate provides the core abstractions for enumerating, querying, and
//! configuring displays across Windows (CCD/GDI) and Linux (DRM/Wayland).
//!
//! ## Architecture
//!
//! - [`UniversalTopology`] — trait for querying current display topology
//! - [`OutputEditable`] — trait for modifying properties of a single output
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
//!
//! ## Feature gates
//!
//! - `ctrl_center` — enables high-level activation logic (`force_activate_by_monitor_name`,
//!   `activate_with_topology_restore`, etc.)
//! - `wgpu_types` — enables WGPU integration for HDR pipelines

pub mod error;
pub mod types;
pub mod traits;
pub mod backends;

// Re-export core types for a flattened, user-friendly API.
pub use error::{DisplayError, DisplayResult};
pub use traits::{OutputEditable, UniversalTopology};
pub use types::*;

// Re-export Windows-specific functions at the top level.
// CCD-level implementation details (query_all_display_targets, find_display_target,
// ccd_wake_display, DisplayTargetInfo) are kept in the backend submodule
// to avoid leaking low-level CCD query patterns into the public API surface.
#[cfg(target_os = "windows")]
pub use backends::windows::{
    force_activate_by_monitor_name, force_all,
    // High-level activation
    activate_display, ActivationResult,
    // WinDisplayManager re-export
    WinDisplayManager,
};

/// The primary entry point for display management.
/// Resolves to a platform-specific implementation at compile time.
///
/// # Example
/// ```rust
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
/// Topology-aware activation: save current topology, force_all, restore, place target.
///
/// This is the main high-level function for activating an inactive monitor while
/// preserving the existing layout. It:
/// 1. Saves the current topology (which monitors are active + their positions/sizes)
/// 2. Calls `force_all()` to activate all monitors so the target becomes reachable
/// 3. Restores the saved topology — original active monitors get their positions back,
///    monitors that were inactive AND are not the target get turned off
/// 4. Places the target monitor according to the `ActivationPlan`
pub async fn activate_with_topology_restore(
    target_id: u32,
    plan: &ActivationPlan,
) -> DisplayResult<()> {
    use std::collections::HashMap;
    use traits::UniversalTopology;

    #[derive(Debug, Clone)]
    struct SavedOutput {
        _name: String,
        enabled: bool,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    }

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

    // ── Step 2: force_all ──
    force_all().map_err(|e| DisplayError::BackendError(e.to_string()))?;

    // Small delay for hardware to settle
    #[cfg(target_os = "windows")]
    {
        // Use a blocking sleep in a spawn_blocking context
        tokio::task::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(800));
        }).await.map_err(|e| DisplayError::BackendError(e.to_string()))?;
    }

    // ── Step 3: Restore saved topology ──
    {
        let mut topo = NativeTopology::acquire()?;
        let outputs = topo.get_outputs();
        let target_id_str = target_id.to_string();

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
        topo.commit().await
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;
    }

    // Small delay for hardware to settle
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(800));
        }).await.map_err(|e| DisplayError::BackendError(e.to_string()))?;
    }

    // ── Step 4: Place target monitor ──
    // Check if target was already active
    let was_active = saved.get(&target_id.to_string())
        .map(|s| s.enabled)
        .unwrap_or(false);

    if was_active {
        // Already positioned correctly from step 3
        return Ok(());
    }

    // Determine position
    let pos = match plan.position {
        Some(p) => p,
        None => {
            // Auto-position: right of rightmost active monitor
            let topo = NativeTopology::acquire()?;
            let right_x = topo.get_outputs()
                .iter()
                .filter(|o| o.enabled)
                .map(|o| o.geometry.origin.x + o.geometry.size.width as i32)
                .max()
                .unwrap_or(0);
            types::Point2D { x: right_x, y: 0 }
        }
    };

    {
        let mut topo = NativeTopology::acquire()?;
        let did = DisplayId(target_id.to_string());
        let mut editor = topo.edit_output(&did)
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;

        editor.set_enabled(true)
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;
        editor.set_position(pos)
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;

        if let Some(res) = plan.resolution {
            editor.set_resolution(res)
                .map_err(|e| DisplayError::BackendError(e.to_string()))?;
        }
        if let Some(rot) = plan.rotation {
            editor.set_rotation(rot)
                .map_err(|e| DisplayError::BackendError(e.to_string()))?;
        }

        drop(editor);
        topo.set_persistence(true);
        let _ = topo.validate().await;
        topo.commit().await
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub struct NativeTopology;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[async_trait::async_trait]
impl traits::UniversalTopology for NativeTopology {
    fn acquire() -> DisplayResult<Self> {
        Err(DisplayError::UnsupportedFeature("Platform not supported".into()))
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        vec![]
    }

    // FIX: parameter type was `&str`; the trait requires `&DisplayId`.
    fn edit_output(&mut self, _: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        Err(DisplayError::UnsupportedFeature("Platform not supported".into()))
    }

    fn set_persistence(&mut self, _: bool) -> &mut Self {
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        Ok(())
    }
}