//! Stub backend for unsupported platforms.
//!
//! This module provides a no-op implementation of [`UniversalTopology`] and
//! [`OutputEditable`] so the crate compiles on platforms other than Windows
//! and Linux (e.g., macOS, WASM targets, or docs.rs documentation builds).
//!
//! All methods return [`DisplayManagerError::UnsupportedFeature`] to ensure callers
//! receive a clear, idiomatic error at runtime.

use async_trait::async_trait;

use crate::error::DisplayManagerError;
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D};

/// Stub topology for unsupported platforms.
///
/// Every method returns an [`DisplayManagerError::UnsupportedFeature`] error.
/// This type exists solely to satisfy the [`UniversalTopology`] trait
/// bound on [`NativeTopology`](super::NativeTopology) during documentation
/// builds and cross-platform compilation.
pub struct StubTopology;

/// Stub output editor for unsupported platforms.
///
/// All mutation methods return
/// [`DisplayManagerError::UnsupportedFeature`].
pub struct StubOutputEditor;

impl OutputEditable for StubOutputEditor {
    fn set_rotation(
        &mut self,
        _rotation: DisplayRotation,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_resolution(
        &mut self,
        _extent: Extent2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_position(
        &mut self,
        _position: Point2D,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_refresh_rate(
        &mut self,
        _rate: u32,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_primary(&mut self) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_hdr(
        &mut self,
        _state: HdrState,
        _mode: HdrMode,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_scale(&mut self, _scale: f64) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_enabled(
        &mut self,
        _enabled: bool,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn clone_from(
        &mut self,
        _source_id: &DisplayId,
    ) -> Result<&mut dyn OutputEditable, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn get_state(&self) -> OutputState {
        OutputState::default()
    }
}

#[async_trait]
impl UniversalTopology for StubTopology {
    fn acquire() -> Result<Self, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        Vec::new()
    }

    fn edit_output(
        &mut self,
        _id: &DisplayId,
    ) -> Result<Box<dyn OutputEditable + '_>, DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_persistence(&mut self, _enabled: bool) -> &mut Self {
        self
    }

    async fn validate(&self) -> Result<(), DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    async fn commit(&mut self) -> Result<(), DisplayManagerError> {
        Err(DisplayManagerError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }
}
