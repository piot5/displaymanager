//! Stub backend for unsupported platforms.
//!
//! This module provides a no-op implementation of [`UniversalTopology`] and
//! [`OutputEditable`] so the crate compiles on platforms other than Windows
//! and Linux (e.g., macOS, WASM targets, or docs.rs documentation builds).
//!
//! All methods return [`DisplayError::UnsupportedFeature`] to ensure callers
//! receive a clear, idiomatic error at runtime.

use async_trait::async_trait;

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, Point2D};

/// Stub topology for unsupported platforms.
///
/// Every method returns an [`DisplayError::UnsupportedFeature`] error.
/// This type exists solely to satisfy the [`UniversalTopology`] trait
/// bound on [`NativeTopology`](super::NativeTopology) during documentation
/// builds and cross-platform compilation.
pub struct StubTopology;

/// Stub output editor for unsupported platforms.
///
/// All mutation methods return
/// [`DisplayError::UnsupportedFeature`].
pub struct StubOutputEditor;

impl OutputEditable for StubOutputEditor {
    fn set_rotation(
        &mut self,
        _rotation: DisplayRotation,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_resolution(&mut self, _extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_position(&mut self, _position: Point2D) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_refresh_rate(&mut self, _rate: u32) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_hdr(
        &mut self,
        _state: HdrState,
        _mode: HdrMode,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_scale(&mut self, _scale: f64) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_enabled(&mut self, _enabled: bool) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn clone_from(&mut self, _source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn get_state(&self) -> OutputState {
        OutputState::default()
    }
}

#[async_trait]
impl UniversalTopology for StubTopology {
    fn acquire() -> DisplayResult<Self> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        Vec::new()
    }

    fn edit_output(&mut self, _id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    fn set_persistence(&mut self, _enabled: bool) -> &mut Self {
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        Err(DisplayError::UnsupportedFeature(
            "platform not supported".into(),
        ))
    }
}
