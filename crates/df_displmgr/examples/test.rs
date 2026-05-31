use df_displmgr::traits::{UniversalTopology,};
use df_displmgr::NativeTopology;
use std::error::Error;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading current display configuration...");
    let mut topology = NativeTopology::acquire()?;

    let mut seen_ids = HashSet::new();
    // Take the first 3 valid monitors and snapshot their current state
    let valid_outputs: Vec<_> = topology
        .get_outputs()
        .into_iter()
        .filter(|o| {
            let is_named = o.identity.monitor_name != "Unknown" && o.identity.monitor_name != "Unknown Display";
            is_named && seen_ids.insert(o.identity.id.clone())
        })
        .take(3)
        .collect();

    println!("Detected monitors: {}", valid_outputs.len());

    let mut current_x_offset = 0;

    for output in &valid_outputs {
        // Wir nutzen die Werte, die der Monitor bereits hat, um Error 87 zu vermeiden
        let w = if output.geometry.size.width > 0 { output.geometry.size.width } else { 1920 };
        let h = if output.geometry.size.height > 0 { output.geometry.size.height } else { 1080 };
        let rr = if output.refresh_rate > 0 { output.refresh_rate } else { 60000 };

        println!("Positioning {} ({}) at {}x0 ({}x{} @ {}mHz)", 
            output.identity.monitor_name, output.identity.id.0, current_x_offset, w, h, rr);

        let mut editor = topology.edit_output(&output.identity.id)?;
        editor
            .set_enabled(true)?
            .set_resolution(df_displmgr::types::Extent2D { width: w, height: h })?
            .set_position(df_displmgr::types::Point2D { x: current_x_offset, y: 0 })?
            .set_refresh_rate(rr)?;

        current_x_offset += w as i32;
    }

    println!("\nStarting validation...");
    match topology.validate().await {
        Ok(_) => {
            println!("Validation successful! Applying layout...");
            topology.set_persistence(true);
            topology.commit().await?;
            println!("Setup applied successfully.");
        }
        Err(e) => {
            eprintln!("Validation failed: {}", e);
            eprintln!("\nLast attempt: trying layout without set_resolution/refresh_rate.");
        }
    }

    Ok(())
}