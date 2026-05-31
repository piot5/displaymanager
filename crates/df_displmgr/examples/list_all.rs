use df_displmgr::{NativeTopology, UniversalTopology, DisplayResult};

#[tokio::main]
async fn main() -> DisplayResult<()> {
    // 1. Acquire current hardware configuration
    // Internally uses the Win32 QueryDisplayConfig API
    let topology = NativeTopology::acquire()?; 
    
    // 2. Get list of all active outputs (monitors)
    let outputs = topology.get_outputs(); 
    
    println!("=== System Display Report ===");
    println!("Detected monitors: {}\n", outputs.len());

    // 3. Iterate over all monitors and print details
    for (i, output) in outputs.iter().enumerate() {
        println!("[Monitor {}]", i + 1);
        println!("  Name:         {}", output.identity.monitor_name);       // Friendly name from EDID
        println!("  ID:           {}", output.identity.id.0);         // Canonical Windows target ID
        println!("  Position:     x: {}, y: {}", output.geometry.origin.x, output.geometry.origin.y); // Desktop coordinates
        println!("  Dimensions:   {}x{} pixels", output.geometry.size.width, output.geometry.size.height);
        println!("  Refresh rate: {} Hz", output.refresh_rate / 1000); // mHz -> Hz
        println!("  Rotation:     {:?}", output.rotation); // Current rotation
        println!("  HDR status:   {:?}", output.hdr_state);
        println!("  Scale:        {:.0}%", output.scale * 100.0);
        
        if let Some(res) = output.native_resolution {
            println!("  Physical res: {}x{}", res.width, res.height);
        }
        println!("----------------------------");
    }

    Ok(())
}