//! Error types for DDC/CI monitor control operations.
//!
//! This module provides backward-compatible wrappers around the unified
//! [`df_core::DisplayManagerError`] type. New code should use
//! `DisplayManagerError` directly.
//!
//! # Migration Guide
//!
//! | Old Type | New Type |
//! |----------|----------|
//! | `DdcError` | `DisplayManagerError` |
//! | `DdcErrorWithContext` | `DisplayManagerError` (with context fields) |
//!
//! The [`DdcError`] type alias is deprecated but still functional.

pub use df_core::DisplayManagerError;

/// Legacy error type for DDC/CI operations.
///
/// This type is deprecated. Use [`DisplayManagerError`] directly for new code.
/// All DDC/CI error variants are available in the unified error type.
///
/// # Examples
///
/// ```rust
/// use df_ddc::error::DisplayManagerError;
///
/// let err = DisplayManagerError::AccessDenied;
/// assert!(err.to_string().contains("access denied"));
/// ```
#[deprecated(since = "0.2.0", note = "Use df_core::DisplayManagerError directly")]
pub type DdcError = DisplayManagerError;

/// Legacy context wrapper for DDC errors.
///
/// This type is deprecated. Use [`DisplayManagerError`] directly.
#[deprecated(since = "0.2.0", note = "Use DisplayManagerError directly")]
pub type DdcErrorWithContext = DisplayManagerError;

// Note: Context methods (context(), with_vcp_code(), with_bus_path()) are
// no longer available on the unified error type. Use structured error
// variants or logging for error context instead.
