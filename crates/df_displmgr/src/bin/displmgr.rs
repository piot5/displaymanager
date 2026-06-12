//! Display Manager CLI Tool
//!
//! This tool demonstrates the functionality of the display manager crate.
//! It can list available displays, show their properties, and perform basic operations.

use df_displmgr::traits::UniversalTopology;
use df_displmgr::NativeTopology;
use df_displmgr::DisplayResult;

#[tokio::main]
async fn main() -> DisplayResult<()> {
    println!("Display Manager CLI Tool");
    println!("=========================");
    
    // Acquire the native display topology
    let topo = NativeTopology::acquire()?;
    
    // Get all available outputs (displays)
    let outputs = topo.get_outputs();
    
    println!("\nAvailable Displays:");
    println!("-------------------");
    
    if outputs.is_empty() {
        println!("No displays detected.");
        return Ok(());
    }
    
    for (index, output) in outputs.iter().enumerate() {
        println!("Display #{}:", index + 1);
        println!("  ID: {:?}", output.identity.id);
        println!("  Name: {}", output.identity.monitor_name);
        println!("  Resolution: {}x{}", output.geometry.size.width, output.geometry.size.height);
        println!("  Position: ({}, {})", output.geometry.origin.x, output.geometry.origin.y);
        println!("  Scale: {:.2}x", output.scale);
        println!("  Primary: {:?}", output.is_primary);
        println!("  Enabled: {:?}", output.enabled);
        
        println!("  HDR State: {:?}", output.hdr_state);
        
        println!("  Refresh Rate: {:.1} Hz", output.refresh_rate as f32 / 1000.0);
        
        println!();
    }
    
    // Demonstrate validation and commit functionality
    println!("Validating display configuration...");
    topo.validate().await?;
    println!("Validation completed successfully!");
    
    println!("Display manager initialized and ready for use.");
    
    Ok(())
}