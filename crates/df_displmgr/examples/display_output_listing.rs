//! Example: List all display outputs and their current state.

use df_displmgr::traits::UniversalTopology;
use df_displmgr::NativeTopology;

#[tokio::main]
async fn main() -> df_displmgr::DisplayResult<()> {
    let topo = NativeTopology::acquire()?;
    let outputs = topo.get_outputs();

    if outputs.is_empty() {
        println!("No display outputs found.");
        return Ok(());
    }

    println!("Found {} output(s):", outputs.len());
    for out in &outputs {
        println!("\n  ID: {}", out.identity.id.0);
        println!("  Name: {}", out.identity.monitor_name);
        println!(
            "  Resolution: {}x{} @ {} Hz",
            out.geometry.size.width,
            out.geometry.size.height,
            out.refresh_rate_hz()
        );
        println!(
            "  Position: ({}, {})",
            out.geometry.origin.x, out.geometry.origin.y
        );
        println!("  Enabled: {}", out.enabled);
        println!("  Rotation: {:?}", out.rotation);
    }

    Ok(())
}
