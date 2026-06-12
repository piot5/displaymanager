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
//!         println!("{} — {} {}", m.target_id, m.manufacturer, m.model);
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

pub mod edid_types;
pub mod edid_trait;
pub mod edid_backends;
pub mod edid_parser;
pub mod error;
pub mod backends;

pub use crate::edid_trait::DisplayDevice;
pub use crate::backends::MonitorDetails;
pub use crate::edid_types::{MonitorTopology, DeepDdcStats};


/// Collects all monitor data by delegating to the platform-specific backend.
///
/// On Windows the backend uses CCD + GDI + Registry; on Linux it uses sysfs +
/// `ddcutil`. The returned vector contains topology, EDID, and DDC statistics
/// for each detected monitor.
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>, crate::error::EdidError> {
    let enumerator = backends::get_platform_enumerator();
    enumerator.collect_monitors()
}
