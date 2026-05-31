// apps/displaymanager_cli/src/synth.rs

use df_displmgr_info::collect_monitor_data;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Context};

/// Represents a stable, persistent identity for a display,
/// decoupling the hardware from volatile OS-assigned IDs.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistentMonitorIdentity {
    pub friendly_name: String,
    pub target_id: u32,
    pub device_path: String,
    /// Added to support GDI backend resolution (e.g., "\\\\.\\DISPLAY1")
    pub gdi_name: String,
}

pub struct MonitorSynthesis;

impl MonitorSynthesis {
    /// Builds a persistent registry of all connected displays by scanning the hardware.
    pub fn build_registry() -> anyhow::Result<Vec<PersistentMonitorIdentity>> {
        let monitors = collect_monitor_data().context("Hardware scan failed via info-crate")?;
        
        Ok(monitors.into_iter().map(|m| {
            PersistentMonitorIdentity {
                friendly_name: m.friendly_name,
                target_id: m.target_id,
                device_path: m.device_path,
                // Assuming GDI name is derived from device path or provided by info-crate
                gdi_name: m.gdi_name, 
            }
        }).collect())
    }

    /// Resolves a user-provided string (name, target_id, or device path) to a stable identity.
    /// Uses .iter() to borrow the registry, ensuring the list remains accessible for error reporting.
    pub fn resolve(query: &str) -> anyhow::Result<PersistentMonitorIdentity> {
        let registry = Self::build_registry()?;
        
        registry.iter()
            .find(|m| {
                m.friendly_name.to_lowercase().contains(&query.to_lowercase()) || 
                m.target_id.to_string() == query ||
                m.device_path.to_lowercase().contains(&query.to_lowercase()) ||
                m.gdi_name.to_lowercase().contains(&query.to_lowercase())
            })
            .cloned() // Clone the found reference to return an owned PersistentMonitorIdentity
            .ok_or_else(|| {
                // Now registry is still available here to construct a helpful error message
                let available: Vec<String> = registry.iter()
                    .map(|m| m.friendly_name.clone())
                    .collect();
                anyhow!("Monitor '{}' not found. Available displays: {:?}", query, available)
            })
    }
}