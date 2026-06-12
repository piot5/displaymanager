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
