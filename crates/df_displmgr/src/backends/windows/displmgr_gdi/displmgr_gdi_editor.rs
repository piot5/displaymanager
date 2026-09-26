//! GDI output editor implementing [`OutputEditable`] for the Windows GDI backend.
//!
//! Provides the [`GdiOutputEditor`] struct that stages display configuration
//! changes in-memory. All mutations are flushed to hardware during
//! [`UniversalTopology::commit`].

use crate::backends::windows::displmgr_gdi::GdiTopology;
use crate::error::DisplayManagerError;
use crate::traits::OutputEditable;
use crate::types::{DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D};

/// Per-output editor for GDI-based display configuration.
///
/// Implements [`OutputEditable`] to provide a builder-style API
/// for modifying display properties before committing. All changes
/// are staged in the parent [`GdiTopology`] state and flushed
/// atomically via [`UniversalTopology::commit`].
pub struct GdiOutputEditor<'a> {
    /// Reference to the parent GDI topology.
    topology: &'a mut GdiTopology,
    /// The display identifier for the output being edited.
    target_id: String,
}

impl<'a> GdiOutputEditor<'a> {
    /// Creates a new GDI output editor for the specified display.
    ///
    /// # Arguments
    ///
    /// * `topology` - Mutable reference to the parent GDI topology.
    /// * `target_id` - The display identifier string (GDI device name).
    pub fn new(topology: &'a mut GdiTopology, target_id: String) -> Self {
        Self {
            topology,
            target_id,
        }
    }

    /// Helper: get mutable output state or return MonitorNotFound error.
    fn get_output_mut(&mut self) -> Result<&mut OutputState, DisplayManagerError> {
        self.topology
            .outputs
            .get_mut(&self.target_id)
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(self.target_id.clone()))
    }
}

impl<'a> OutputEditable for GdiOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.rotation = rotation;
        Ok(self)
    }

    fn set_resolution(
        &mut self,
        extent: Extent2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.geometry.size = extent;
        Ok(self)
    }

    fn set_position(
        &mut self,
        position: Point2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.geometry.origin = position;
        Ok(self)
    }

    fn set_refresh_rate(
        &mut self,
        rate: u32,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.refresh_rate = rate;
        Ok(self)
    }

    fn set_primary(&mut self) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        self.topology.target_primary_id = Some(self.target_id.clone());
        Ok(self)
    }

    fn set_hdr(
        &mut self,
        state_val: HdrState,
        mode_val: HdrMode,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.hdr_state = state_val;
        state.hdr_mode = mode_val;
        Ok(self)
    }

    fn set_scale(
        &mut self,
        scale_val: f64,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.scale = scale_val;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.topology
            .outputs
            .get(&self.target_id)
            .cloned()
            .unwrap_or_default()
    }

    fn set_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let state = self.get_output_mut()?;
        state.enabled = enabled;
        Ok(self)
    }

    fn clone_from(
        &mut self,
        source_id: &DisplayId,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        let source_state = self
            .topology
            .outputs
            .get(&source_id.0)
            .ok_or_else(|| DisplayManagerError::MonitorNotFound(source_id.to_string()))?
            .clone();
        let state = self.get_output_mut()?;

        state.geometry = source_state.geometry;
        state.refresh_rate = source_state.refresh_rate;
        state.rotation = source_state.rotation;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        self, DisplayIdentity, DisplayRotation, Extent2D, HdrMode, HdrState, Point2D,
    };

    #[test]
    fn test_gdi_output_editor_set_enabled() {
        let mut topo = GdiTopology {
            outputs: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "test".to_string(),
                    types::OutputState {
                        identity: DisplayIdentity {
                            id: types::DisplayId("test".to_string()),
                            connector_id: types::ConnectorId("test".to_string()),
                            adapter_id: types::AdapterId("adapter0".to_string()),
                            hardware_uuid: None,
                            monitor_name: "Test Monitor".to_string(),
                        },
                        geometry: types::Rect {
                            origin: Point2D { x: 0, y: 0 },
                            size: Extent2D {
                                width: 1920,
                                height: 1080,
                            },
                        },
                        scale: 1.0,
                        rotation: DisplayRotation::Rotate0,
                        refresh_rate: 60_000,
                        hdr_state: HdrState::Disabled,
                        hdr_mode: HdrMode::Default,
                        native_resolution: None,
                        supported_modes: vec![],
                        enabled: true,
                        is_primary: false,
                    },
                );
                map
            },
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let mut editor = GdiOutputEditor::new(&mut topo, "test".to_string());
        let result = editor.set_enabled(false);
        assert!(result.is_ok());
        assert!(!topo.outputs.get("test").unwrap().enabled);
    }

    #[test]
    fn test_gdi_output_editor_set_rotation() {
        let mut topo = GdiTopology {
            outputs: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "test".to_string(),
                    types::OutputState {
                        identity: DisplayIdentity {
                            id: types::DisplayId("test".to_string()),
                            ..Default::default()
                        },
                        geometry: types::Rect {
                            origin: Point2D { x: 0, y: 0 },
                            size: Extent2D {
                                width: 1920,
                                height: 1080,
                            },
                        },
                        rotation: DisplayRotation::Rotate0,
                        ..Default::default()
                    },
                );
                map
            },
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let mut editor = GdiOutputEditor::new(&mut topo, "test".to_string());
        let result = editor.set_rotation(DisplayRotation::Rotate90);
        assert!(result.is_ok());
        assert_eq!(
            topo.outputs.get("test").unwrap().rotation,
            DisplayRotation::Rotate90
        );
    }

    #[test]
    fn test_gdi_output_editor_set_position() {
        let mut topo = GdiTopology {
            outputs: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "test".to_string(),
                    types::OutputState {
                        identity: DisplayIdentity {
                            id: types::DisplayId("test".to_string()),
                            ..Default::default()
                        },
                        geometry: types::Rect {
                            origin: Point2D { x: 0, y: 0 },
                            size: Extent2D {
                                width: 1920,
                                height: 1080,
                            },
                        },
                        ..Default::default()
                    },
                );
                map
            },
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let mut editor = GdiOutputEditor::new(&mut topo, "test".to_string());
        let result = editor.set_position(Point2D { x: 100, y: 200 });
        assert!(result.is_ok());
        let state = topo.outputs.get("test").unwrap();
        assert_eq!(state.geometry.origin.x, 100);
        assert_eq!(state.geometry.origin.y, 200);
    }

    #[test]
    fn test_gdi_output_editor_set_resolution() {
        let mut topo = GdiTopology {
            outputs: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "test".to_string(),
                    types::OutputState {
                        identity: DisplayIdentity {
                            id: types::DisplayId("test".to_string()),
                            ..Default::default()
                        },
                        geometry: types::Rect {
                            origin: Point2D { x: 0, y: 0 },
                            size: Extent2D {
                                width: 1920,
                                height: 1080,
                            },
                        },
                        ..Default::default()
                    },
                );
                map
            },
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let mut editor = GdiOutputEditor::new(&mut topo, "test".to_string());
        let result = editor.set_resolution(Extent2D {
            width: 3840,
            height: 2160,
        });
        assert!(result.is_ok());
        assert_eq!(topo.outputs.get("test").unwrap().geometry.size.width, 3840);
        assert_eq!(topo.outputs.get("test").unwrap().geometry.size.height, 2160);
    }

    #[test]
    fn test_gdi_output_editor_get_state() {
        let mut topo = GdiTopology {
            outputs: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "test".to_string(),
                    types::OutputState {
                        identity: DisplayIdentity {
                            id: types::DisplayId("test".to_string()),
                            ..Default::default()
                        },
                        geometry: types::Rect {
                            origin: Point2D { x: 0, y: 0 },
                            size: Extent2D {
                                width: 1920,
                                height: 1080,
                            },
                        },
                        ..Default::default()
                    },
                );
                map
            },
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let editor = GdiOutputEditor::new(&mut topo, "test".to_string());
        let state = editor.get_state();
        assert_eq!(state.geometry.size.width, 1920);
    }

    #[test]
    fn test_gdi_output_editor_not_found() {
        let mut topo = GdiTopology {
            outputs: std::collections::HashMap::new(),
            persistence_enabled: true,
            saved_modes: std::collections::HashMap::new(),
            target_primary_id: None,
        };

        let mut editor = GdiOutputEditor::new(&mut topo, "nonexistent".to_string());
        let result = editor.set_enabled(true);
        assert!(result.is_err());
    }
}
