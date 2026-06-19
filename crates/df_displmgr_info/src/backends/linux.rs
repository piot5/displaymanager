//! Linux-specific monitor enumeration using sysfs and ddcutil backends.

use std::fs;

use super::{MonitorDetails, MonitorEnumerator};
use crate::edid_backends::edid_linux_ddc::LinuxDdcBackend;
use crate::edid_backends::edid_linux_sys::LinuxSysfsBackend;
use crate::edid_parser::EdidParser;
use crate::edid_trait::EdidControl;
use crate::error::EdidError;

/// Linux monitor enumerator using sysfs and ddcutil.
pub struct LinuxMonitorEnumerator;

impl MonitorEnumerator for LinuxMonitorEnumerator {
    fn collect_monitors(&self) -> Result<Vec<MonitorDetails>, EdidError> {
        enumerate_linux_monitors()
    }
}

/// Collects monitor data on Linux by reading sysfs and querying ddcutil.
fn enumerate_linux_monitors() -> Result<Vec<MonitorDetails>, EdidError> {
    let mut results = Vec::new();
    let drm_path = std::path::Path::new("/sys/class/drm/");

    // Build a bus-ID map from `ddcutil detect` once upfront so each connector
    // is queried against its own I2C bus. Without this map, all connectors
    // would probe bus 0 and return stats for the wrong monitor.
    let bus_map = LinuxDdcBackend::detect_connector_bus_map();

    if let Ok(entries) = fs::read_dir(drm_path) {
        let mut target_id = 0u32;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();

            // Filter out render/control virtual nodes.
            if !name.contains('-') || name.contains("render") || name.contains("control") {
                continue;
            }

            let status_path = entry.path().join("status");
            let is_active = fs::read_to_string(&status_path)
                .map(|s| s.trim() == "connected")
                .unwrap_or(false);

            let sysfs_backend = LinuxSysfsBackend {
                connector_hint: Some(name.clone()),
            };

            let edid_data = sysfs_backend
                .get_edid_raw()
                .ok()
                .and_then(|raw| EdidParser::parse(&raw).ok());

            // Look up the bus ID for this connector and query real VCP stats
            // via `ddcutil getvcp`. Falls back to None on any error.
            let ddc_stats = if is_active {
                let bus_id = bus_map.get(&name).copied();
                LinuxDdcBackend { bus_id }.query_hardware_stats().ok()
            } else {
                None
            };

            results.push(MonitorDetails {
                target_id,
                friendly_name: edid_data
                    .as_ref()
                    .map(|e| e.model_name.clone())
                    .unwrap_or_else(|| "Generic Display".into()),
                is_active,
                output_tech: name.split('-').nth(1).unwrap_or("Unknown").to_string(),
                gdi_name: name.clone(),
                device_path: entry.path().to_string_lossy().into_owned(),
                topology: None,
                edid: edid_data,
                ddc_stats,
            });
            target_id += 1;
        }
    }

    if results.is_empty() {
        Err(EdidError::NotFound)
    } else {
        Ok(results)
    }
}
