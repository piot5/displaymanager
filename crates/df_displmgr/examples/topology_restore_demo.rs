// test_topology_restore.rs
//
// Topology save and restore (high-level API):
//
// Uses the `activate_with_topology_restore()` function from the crate.
// Demonstrates the full workflow: Save -> force_all -> Restore -> Place.

use df_displmgr::{
    activate_with_topology_restore, ActivationPlan, DisplayId, Extent2D, NativeTopology, Point2D,
    UniversalTopology,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> df_displmgr::DisplayResult<()> {
    println!("================================================");
    println!("  Topology Save/Restore (high-level API)");
    println!("================================================");

    // 1. Initial state: only primary is active
    println!("\n>>> 1. Leave only the primary active:");
    let mut topo = NativeTopology::acquire()?;
    for o in topo.get_outputs() {
        if !o.is_primary && o.enabled {
            let did = DisplayId(o.identity.id.0.clone());
            if let Ok(mut editor) = topo.edit_output(&did) {
                editor.set_enabled(false)?;
                println!(
                    "  [off] {} (id={})",
                    o.identity.monitor_name.trim(),
                    o.identity.id.0
                );
            }
        }
    }
    topo.set_persistence(true);
    topo.commit().await?;
    println!("  [OK] Only primary active");

    // 2. Topology-aware activation
    println!("\n>>> 2. Activate monitor 4352 (DTV) with auto-position (right of primary):");
    let plan = ActivationPlan {
        position: None, // auto: right of rightmost active monitor
        resolution: None,
        rotation: None,
    };
    activate_with_topology_restore(4352, &plan).await?;
    println!("  [OK] DTV activated and positioned");

    // 3. With explicit position
    println!("\n>>> 3. Activate monitor 4356 (Artisr) at explicit position (3840,0):");
    let plan = ActivationPlan {
        position: Some(Point2D { x: 3840, y: 0 }),
        resolution: Some(Extent2D {
            width: 1920,
            height: 1080,
        }),
        rotation: None,
    };
    activate_with_topology_restore(4356, &plan).await?;
    println!("  [OK] Artisr activated at (3840,0)");

    // 4. Final layout
    println!("\n>>> 4. Final topology:");
    let topo = NativeTopology::acquire()?;
    for (i, o) in topo.get_outputs().iter().enumerate() {
        let status = if o.enabled { "ON" } else { "off" };
        let primary = if o.is_primary { "*" } else { " " };
        println!(
            "  [{}] {} {} (id={}) pos=({},{}) size={}x{} {}",
            i + 1,
            primary,
            o.identity.monitor_name.trim(),
            o.identity.id.0,
            o.geometry.origin.x,
            o.geometry.origin.y,
            o.geometry.size.width,
            o.geometry.size.height,
            status
        );
    }

    println!("\n================================================");
    println!("  Done");
    println!("================================================");
    Ok(())
}
