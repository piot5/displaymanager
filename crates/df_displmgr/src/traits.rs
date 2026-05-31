use crate::types::{OutputState, DisplayRotation, HdrState, HdrMode, DisplayId, Extent2D, Point2D};
use crate::error::DisplayResult;
use async_trait::async_trait;

/// Interface for modifying a specific display output's configuration.
/// All mutations are staged and flushed only when `commit` is called on the topology.
pub trait OutputEditable {
    /// Sets the hardware rotation of the display.
    fn set_rotation(&mut self, rotation: DisplayRotation) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the active resolution.
    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable>;

    /// Positions the display within the virtual coordinate space.
    fn set_position(&mut self, position: Point2D) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the refresh rate in millihertz (mHz).
    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable>;

    /// Designates this output as the primary monitor.
    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable>;

    /// Configures HDR state and color-volume profile.
    fn set_hdr(&mut self, state: HdrState, mode: HdrMode) -> DisplayResult<&mut dyn OutputEditable>;

    /// Sets the desktop scaling factor (e.g., 1.5 for 150%).
    fn set_scale(&mut self, scale: f64) -> DisplayResult<&mut dyn OutputEditable>;

    /// Enables or disables the signal output for this display.
    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable>;

    /// Copies the configuration from another display identified by `source_id`.
    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable>;

    /// Returns a snapshot of the current configuration for this output.
    fn get_state(&self) -> OutputState;
}

/// Interface for managing the global display arrangement.
#[async_trait]
pub trait UniversalTopology: Sized + Send + Sync {
    /// Initializes the topology by querying the current system state.
    fn acquire() -> DisplayResult<Self>;

    /// Returns all detected display outputs.
    fn get_outputs(&self) -> Vec<OutputState>;

    /// Returns a mutable editor for the display identified by `id`.
    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>>;

    /// Toggles persistence of configuration changes (e.g., Registry updates).
    fn set_persistence(&mut self, enabled: bool) -> &mut Self;

    /// Validates the staged configuration without applying hardware changes.
    async fn validate(&self) -> DisplayResult<()>;

    /// Flushes all staged changes to the hardware.
    async fn commit(&mut self) -> DisplayResult<()>;
}