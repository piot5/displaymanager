// test_activation_plan.rs
//
// ActivationPlan demo:
// Shows different configurations of the ActivationPlan struct.

use df_displmgr::{
    NativeTopology, UniversalTopology, DisplayId,
    Extent2D, Point2D, ActivationPlan, DisplayRotation,
    activate_with_topology_restore,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> df_displmgr::DisplayResult<()> {
    println!("================================================");
    println!("  ActivationPlan configurations");
    println!("================================================");

    // Setup: only primary is active
    println!("\n>>> Setup: only primary active:");
    let mut topo = NativeTopology::acquire()?;
    for o in topo.get_outputs() {
        if !o.is_primary && o.enabled {
            let did = DisplayId(o.identity.id.0.clone());
            if let Ok(mut editor) = topo.edit_output(&did) {
                editor.set_enabled(false)?;
            }
        }
    }
    topo.set_persistence(true);
    topo.commit().await?;
    println!("  [OK] Only primary active");

    // Test 1: auto position (default)
    println!("\n>>> Test 1: auto position (right of primary):");
    let plan = ActivationPlan::default(); // all None
    activate_with_topology_restore(4352, &plan).await?;
    println!("  [OK] DTV at auto position");

    // Test 2: explicit position
    println!("\n>>> Test 2: explicit position (1920, 0):");
    let plan = ActivationPlan {
        position: Some(Point2D { x: 1920, y: 0 }),
        resolution: None,
        rotation: None,
    };
    activate_with_topology_restore(4356, &plan).await?;
    println!("  [OK] Artisr at (1920, 0)");

    // Test 3: position + resolution
    println!("\n>>> Test 3: position (0, 1080) + resolution (1920x1080):");
    let plan = ActivationPlan {
        position: Some(Point2D { x: 0, y: 1080 }),
        resolution: Some(Extent2D { width: 1920, height: 1080 }),
        rotation: None,
    };
    // Use a different monitor if available, otherwise skip
    let topo = NativeTopology::acquire()?;
    let has_4354 = topo.get_outputs().iter().any(|o| o.identity.id.0 == "4354");
    if has_4354 {
        activate_with_topology_restore(4354, &plan).await?;
        println!("  [OK] BenQ at (0, 1080) 1920x1080");
    } else {
        println!("  [skip] Monitor 4354 not available, skipped");
    }

    // Test 4: with rotation
    println!("\n>>> Test 4: position (3840, 0) + rotation 90 deg:");
    let plan = ActivationPlan {
        position: Some(Point2D { x: 3840, y: 0 }),
        resolution: Some(Extent2D { width: 1080, height: 1920 }), // rotated
        rotation: Some(DisplayRotation::Rotate90),
    };
    activate_with_topology_restore(4356, &plan).await?;
    println!("  [OK] Artisr at (3840, 0) rotated 90 deg");

    // Final display
    println!("\n>>> Final topology:");
    let topo = NativeTopology::acquire()?;
    for (i, o) in topo.get_outputs().iter().enumerate() {
        let status = if o.enabled { "ON" } else { "off" };
        let primary = if o.is_primary { "*" } else { " " };
        println!(
            "  [{}] {} {} (id={}) pos=({},{}) size={}x{} rot={:?} {}",
            i + 1, primary, o.identity.monitor_name.trim(),
            o.identity.id.0, o.geometry.origin.x, o.geometry.origin.y,
            o.geometry.size.width, o.geometry.size.height, o.rotation, status
        );
    }

    println!("\n================================================");
    println!("  Done");
    println!("================================================");
    Ok(())
}
