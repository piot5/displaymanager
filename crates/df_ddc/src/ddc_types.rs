//! DDC/CI VCP (Virtual Control Panel) type re-exports.
//!
//! This module re-exports unified types from df_core crate.
//! All type definitions have been moved to `crates/df_core/src/lib.rs`.

// Re-export unified types from df_core
pub use df_core::{MonitorCapabilities, PowerState, InputSource, VcpCode};

// Re-export error type for backward compatibility
pub use df_core::DisplayManagerError as DdcError;

/// DDC/CI specific result type.
pub type DdcResult<T> = Result<T, DdcError>;
