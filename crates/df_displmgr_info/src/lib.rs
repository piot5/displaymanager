//! # df_displmgr_info
//!
//! Display management and hardware telemetry framework.
//!
//! Reads raw EDID blocks, parses them, combines them with DDC statistics
//! ([`DeepDdcStats`]) and topology ([`MonitorTopology`]), and exposes a
//! single [`MonitorDetails`] per display.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use df_displmgr_info::collect_monitor_data;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let monitors = collect_monitor_data()?;
//!     for m in &monitors {
//!         println!("{} — {} (active: {})", m.target_id, m.friendly_name, m.is_active);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |---|---|
//! | `edid_parser` | EDID parser (base block + extensions) with checksum validation |
//! | `edid_trait` | [`DisplayDevice`] trait with `fetch_edid()` |
//! | `edid_backends` | OS-specific sources (Windows: SetupAPI/CCD; Linux: sysfs + ddcutil) |
//! | `backends` | Platform enumerator and [`MonitorDetails`] |
//! | `edid_types` | [`EdidData`], [`MonitorCapabilities`], [`MonitorTopology`], [`DeepDdcStats`] |
//! | `error` | [`EdidError`] enum |
//!
//! ## License
//!
//! Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
//! at your option.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unsafe_code)]

pub mod backends;
/// Platform-specific EDID backend implementations.
pub mod edid_backends;
/// EDID binary data parser.
pub mod edid_parser;
/// Trait definitions for EDID data retrieval.
pub mod edid_trait;
/// EDID and DDC/CI type definitions for display hardware.
pub mod edid_types;
/// Error types for EDID operations.
pub mod error;

pub use crate::backends::MonitorDetails;
pub use crate::edid_trait::DisplayDevice;
pub use crate::edid_types::{DeepDdcStats, EdidData, MonitorTopology};

/// Collects all monitor data by delegating to the platform-specific backend.
///
/// On Windows the backend uses CCD + GDI + Registry; on Linux it uses sysfs +
/// `ddcutil`. The returned vector contains topology, EDID, and DDC statistics
/// for each detected monitor.
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>, crate::error::EdidError> {
    let enumerator = backends::get_platform_enumerator();
    enumerator.collect_monitors()
}

/// Parses an EDID file from the filesystem into [`EdidData`].
///
/// This function reads a binary EDID file (typically from `/sys/class/drm/*/edid`
/// on Linux or from registry dumps on Windows) and parses it into a structured
/// [`EdidData`] instance.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if the EDID data is invalid.
pub fn parse_edid_file(path: &str) -> Result<EdidData, crate::error::EdidError> {
    use std::fs;
    
    // Read EDID binary data from file (uses transparent From impl)
    let raw = fs::read(path).map_err(crate::error::EdidError::Io)?;
    
    // Use EdidParser to parse the raw data
    edid_parser::EdidParser::parse(&raw)
}