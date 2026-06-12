//! Debug and testing utilities for display manager functionality.
//!
//! This module provides debugging tools and test functions to verify
//! the proper operation of display configuration commands and topology management.

use crate::cli::DisplayArgs;
use crate::info;
use crate::set;
use anyhow::Result;

/// Debug function to test display scanning functionality
pub fn debug_scan() -> Result<()> {
    println!("=== Display Scan Debug ===");
    info::scan()?;
    println!("Scan completed successfully\n");
    Ok(())
}

/// Debug function to test monitor resolution
pub fn debug_resolve_monitor(query: &str) -> Result<()> {
    println!("=== Monitor Resolution Debug ===");
    println!("Resolving monitor: {}", query);
    
    match info::resolve_monitor(query) {
        Ok(monitor) => {
            println!("Found monitor:");
            println!("  ID: {}", monitor.target_id);
            println!("  Name: {}", monitor.friendly_name);
            println!("  Active: {}", monitor.is_active);
            println!("  Device Path: {}", monitor.device_path);
            println!("  GDI Name: {}", monitor.gdi_name);
            println!("  Output Tech: {}", monitor.output_tech);
        }
        Err(e) => {
            eprintln!("Error resolving monitor: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Debug function to test display setting application
pub async fn debug_apply_settings(output_id: &str, args: &DisplayArgs) -> Result<()> {
    println!("=== Display Settings Application Debug ===");
    println!("Applying settings to: {}", output_id);
    println!("Mode: {:?}", args.mode);
    println!("Position: {:?}", args.pos);
    println!("Rotation: {:?}", args.rotate);
    println!("Off: {}", args.off);
    
    match set::apply_all_settings(output_id, args).await {
        Ok(()) => println!("Settings applied successfully"),
        Err(e) => eprintln!("Error applying settings: {}", e),
    }
    println!();
    Ok(())
}

/// Debug function to test DDC capabilities
#[cfg(feature = "ddc")]
pub fn debug_ddc_capabilities() -> Result<()> {
    println!("=== DDC Capabilities Debug ===");
    
    #[cfg(feature = "ddc")]
    {
        use crate::ddc;
        use df_ddc::list_monitors;
        
        let devices = list_monitors();
        if devices.is_empty() {
            println!("No DDC-capable monitors detected");
            return Ok(());
        }
        
        println!("Found {} DDC-capable monitor(s):", devices.len());
        for (i, device) in devices.iter().enumerate() {
            println!("  {}: {}", i, device.info);
        }
    }
    
    println!();
    Ok(())
}

/// Comprehensive debug function that runs all tests
pub async fn run_comprehensive_debug() -> Result<()> {
    println!("=== Comprehensive Display Manager Debug ===\n");
    
    // Test scan functionality
    debug_scan()?;
    
    // Test monitor resolution with a sample query
    debug_resolve_monitor("DisplayPort")?;
    
    // Test DDC capabilities if available
    #[cfg(feature = "ddc")]
    debug_ddc_capabilities()?;
    
    println!("=== Debug Complete ===\n");
    Ok(())
}