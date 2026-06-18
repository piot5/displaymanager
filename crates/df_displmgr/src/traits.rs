//! Trait definitions that establish the cross-platform display management contract.
//!
//! Every platform backend (Windows CCD/GDI, Linux DRM/Wayland/KDE/udev) must
//! implement [`UniversalTopology`] and return [`OutputEditable`] trait objects.
//! This ensures the public API remains 100% identical regardless of the
//! underlying operating system.

use async_trait::async_trait;

use crate::error::DisplayResult;
use crate::types::{
    DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D,
};

/// Interface for modifying a specific display output's configuration.
///
/// All mutations are staged in-memory and flushed to hardware only when
/// [`UniversalTopology::commit`] is called. This allows batch editing of
/// multiple outputs within a single atomic transaction.
///
/// # Examples
///
/// ```rust,no_run
/// # use df_displmgr::traits::*;
/// # use df_displmgr::types::*;
/// # use df_displmgr::error::DisplayResult;
/// # fn example(topo: &mut impl UniversalTopology) -> DisplayResult<()> {
/// let did = DisplayId("HDMI-1".into());
/// let mut editor = topo.edit_output(&did)?;
/// editor
///     .set_resolution(Extent2D { width: 2560, height: 1440 })?
///     .set_refresh_rate(144_000)?
///     .set_position(Point2D { x: 0, y: 0 })?
///     .set_primary()?;
/// drop(editor);
/// # Ok(())
/// # }
/// ```
pub trait OutputEditable {
    /// Sets the hardware rotation of the display.
    ///
    /// Supported values are 0°, 90°, 180°, and 270° clockwise from normal
    /// landscape orientation.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::NotFound`] if the target output was removed
    /// between the initial lookup and this call.
    fn set_rotation(&mut self, rotation: DisplayRotation) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the active resolution in pixels.
    ///
    /// The resolution must be one of the modes listed in
    /// [`OutputState::supported_modes`]. Backend implementations may reject
    /// modes that the connected panel does not physically support.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::ConfigurationRejected`] if the hardware rejects
    /// the requested resolution.
    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable>;

    /// Positions the display within the virtual coordinate space.
    ///
    /// The origin is relative to the primary monitor's top-left corner at
    /// `(0, 0)`. Negative coordinates are permitted for monitors positioned
    /// to the left of or above the primary.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::NotFound`] if the target output was removed
    /// between the initial lookup and this call.
    fn set_position(&mut self, position: Point2D) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the refresh rate in millihertz (mHz).
    ///
    /// Divide by 1000 to obtain standard Hz. For example, 144_000 mHz
    /// represents 144 Hz. The value must correspond to a mode in
    /// [`OutputState::supported_modes`].
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::ConfigurationRejected`] if the refresh rate is
    /// not supported at the current resolution.
    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable>;

    /// Designates this output as the primary monitor.
    ///
    /// The primary monitor hosts the taskbar and receives the `(0,0)` origin
    /// in the virtual desktop coordinate system. Setting a new primary
    /// automatically clears the flag on all other outputs.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::UnsupportedFeature`] if the backend does not
    /// support primary-monitor designation.
    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable>;

    /// Configures HDR state and color-volume profile.
    ///
    /// On backends that support HDR (Windows CCD, Linux DRM with
    /// `HDR_OUTPUT_METADATA`), this toggles the HDR signaling mode and
    /// selects the appropriate tone-mapping profile.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::UnsupportedFeature`] if HDR is not available
    /// on the current backend, or [`DisplayError::UnsupportedHardware`] if the
    /// connected panel is SDR-only.
    fn set_hdr(
        &mut self,
        state: HdrState,
        mode: HdrMode,
    ) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the desktop scaling factor.
    ///
    /// A value of `1.0` represents 100% (no scaling), while `1.5` represents
    /// 150%. Fractional values are supported where the compositor or driver
    /// allows them.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::ConfigurationRejected`] if the scale factor is
    /// outside the hardware's supported range.
    fn set_scale(&mut self, scale: f64) -> DisplayResult<&mut dyn OutputEditable>;

    /// Enables or disables the signal output for this display.
    ///
    /// Disabling an output removes it from the virtual desktop without
    /// physically disconnecting it. The output remains in the topology and
    /// can be re-enabled in a subsequent transaction.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::UnsupportedFeature`] if the backend does not
    /// support dynamic output enable/disable.
    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable>;

    /// Copies the configuration from another display identified by `source_id`.
    ///
    /// This duplicates resolution, refresh rate, rotation, scale, and
    /// position from the source output to the current output. The source
    /// output must exist in the same topology.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::NotFound`] if `source_id` does not refer to a
    /// valid output in the current topology.
    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable>;

    /// Returns a snapshot of the current configuration for this output.
    ///
    /// The returned [`OutputState`] reflects the *staged* values — if
    /// mutations have been applied via this editor but not yet committed,
    /// they will be visible in the snapshot.
    fn get_state(&self) -> OutputState;
}

/// Interface for managing the global display arrangement.
///
/// This is the primary entry point for display management operations.
/// Implementations exist for each supported platform:
///
/// | Platform | Struct | Backend |
/// |----------|--------|---------|
/// | Windows  | [`WinDisplayManager`](crate::backends::windows::WinDisplayManager) | CCD + GDI |
/// | Linux    | [`WlrTopology`](crate::backends::linux::WlrTopology) | Wayland/wlroots |
/// | Linux    | [`DrmTopology`](crate::backends::linux::displmgr_drm::DrmTopology) | DRM/KMS |
/// | Linux    | [`KdeTopology`](crate::backends::linux::displmgr_kde::KdeTopology) | KDE Spectacle/kscreen |
/// | Linux    | [`UdevTopology`](crate::backends::linux::displmgr_udev::UdevTopology) | udev/udevadm |
///
/// # Lifecycle
///
/// 1. **Acquire** — Call [`acquire`](Self::acquire) to snapshot the current
///    hardware state. This is synchronous because it may involve FFI calls.
/// 2. **Edit** — Obtain an [`OutputEditable`] via [`edit_output`](Self::edit_output)
///    and stage mutations.
/// 3. **Validate** — Optionally call [`validate`](Self::validate) to perform
///    a dry-run test without applying changes.
/// 4. **Commit** — Call [`commit`](Self::commit) to flush all staged changes
///    to hardware atomically.
#[async_trait]
pub trait UniversalTopology: Sized + Send + Sync {
    /// Initializes the topology by querying the current system state.
    ///
    /// This method is **synchronous** because platform enumeration typically
    /// involves blocking FFI calls (e.g., `QueryDisplayConfig`, DRM ioctl).
    /// Async wrappers should be built on top of this method rather than
    /// embedding async logic inside it.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::ConnectionFailed`] if the graphics subsystem
    /// cannot be reached, or [`DisplayError::PermissionDenied`] if the process
    /// lacks the required privileges.
    fn acquire() -> DisplayResult<Self>;

    /// Returns all detected display outputs.
    ///
    /// The returned vector contains every output visible to the backend,
    /// regardless of whether it is currently enabled. Disabled outputs have
    /// [`OutputState::enabled`] set to `false`.
    fn get_outputs(&self) -> Vec<OutputState>;

    /// Returns a mutable editor for the display identified by `id`.
    ///
    /// The editor borrows the topology mutably, preventing concurrent edits
    /// to the same topology instance. Drop the editor before calling
    /// [`commit`](Self::commit).
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::NotFound`] if `id` does not correspond to any
    /// output in the current topology.
    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>>;

    /// Toggles persistence of configuration changes.
    ///
    /// When enabled, committed changes are written to persistent storage
    /// (e.g., Windows Registry, systemd/udev rules) so they survive reboots.
    /// When disabled, changes are volatile and revert on the next session.
    ///
    /// Returns `&mut Self` to allow method chaining.
    fn set_persistence(&mut self, enabled: bool) -> &mut Self;

    /// Validates the staged configuration without applying hardware changes.
    ///
    /// This performs a dry-run check using the platform's validation API
    /// (e.g., `SDC_VALIDATE` on Windows, `TEST_ONLY` atomic commit on DRM).
    /// It also performs geometric overlap detection across all enabled
    /// outputs.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::ConfigurationRejected`] if the staged
    /// configuration is invalid.
    async fn validate(&self) -> DisplayResult<()>;

    /// Flushes all staged changes to the hardware.
    ///
    /// This is the only method that actually modifies hardware state.
    /// Implementations typically spawn a blocking task for FFI-heavy
    /// backends to avoid stalling the async runtime.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayError::BackendError`] if the platform rejects the
    /// commit, or [`DisplayError::StaleTopology`] if the topology has been
    /// invalidated since the last acquire.
    async fn commit(&mut self) -> DisplayResult<()>;
}