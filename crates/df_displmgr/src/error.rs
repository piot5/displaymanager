//! Error types for the `df_displmgr` crate.
//!
//! This module provides the unified [`DisplayManagerError`] re-exported from
//! [`df_core`], along with backward-compatible type aliases for legacy code.
//!
//! # Migration Guide
//!
//! | Old Type | New Type |
//! |----------|----------|
//! | `DisplayError` | `DisplayManagerError` |
//! | `DisplayErrorWithSource` | `DisplayManagerError` (context wrapper removed) |
//! | `DisplayResult<T>` | `Result<T, DisplayManagerError>` |
//!
//! New code should use [`DisplayManagerError`] directly. The type aliases
//! remain for backward compatibility but are deprecated.

pub use df_core::DisplayManagerError;

/// Legacy error type for display management operations.
///
/// This type is deprecated. Use [`DisplayManagerError`] directly for new code.
/// All display management error variants are available in the unified error type.
///
/// # Examples
///
/// ```rust
/// use df_displmgr::error::DisplayManagerError;
///
/// let err = DisplayManagerError::MonitorNotFound("HDMI-1".into());
/// assert!(err.to_string().contains("not found"));
/// ```
#[deprecated(since = "0.2.0", note = "Use df_core::DisplayManagerError directly")]
pub type DisplayError = DisplayManagerError;

/// Legacy context wrapper for display errors.
///
/// This type is deprecated. Context information should be handled differently
/// in new code (e.g., logging, error chaining).
#[deprecated(since = "0.2.0", note = "Use DisplayManagerError directly")]
pub type DisplayErrorWithSource = DisplayManagerError;

/// Legacy result type for display operations.
///
/// This type is deprecated. Use `Result<T, DisplayManagerError>` directly.
#[deprecated(since = "0.2.0", note = "Use Result<T, DisplayManagerError> directly")]
pub type DisplayResult<T> = Result<T, DisplayManagerError>;
