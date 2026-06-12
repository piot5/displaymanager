//! Example: List all DDC/CI-capable monitors and their capabilities.

use df_ddc::list_monitors;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitors = list_monitors();

    if monitors.is_empty() {
        println!("No DDC/CI-capable monitors found.");
        return Ok(());
    }

    println!("Found {} monitor(s):", monitors.len());
    for (i, dev) in monitors.iter().enumerate() {
        println!("\n--- Monitor {} ---", i);
        println!("  Info: {}", dev.info);

        match dev.inner.get_capabilities() {
            Ok(caps) => {
                println!("  Brightness: {}/{}", caps.brightness, caps.brightness_max);
                println!("  Contrast:   {}/{}", caps.contrast, caps.contrast_max);
            }
            Err(e) => {
                println!("  Capabilities error: {}", e);
            }
        }
    }

    Ok(())
}