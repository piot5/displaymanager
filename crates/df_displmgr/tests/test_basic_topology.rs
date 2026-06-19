// test_basic_topology.rs
//
// Basic topology operations:
// - Scan and list all monitors
// - Activate/deactivate a single monitor
// - Set position and resolution

use df_displmgr::{
    force_activate_by_monitor_name, force_all, DisplayId, Extent2D, NativeTopology, Point2D,
    UniversalTopology,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> df_displmgr::DisplayResult<()> {
    println!("================================================");
    println!("  Basic Topology Operations");
    println!("================================================");

    // 1. Scan
    println!("\n>>> 1. Scanning all monitors:");
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

    // 2. force_all
    println!("\n>>> 2. force_all() -- turn everything on:");
    force_all()?;
    println!("  [OK] All monitors activated");

    // 3. Deactivate a single monitor
    println!("\n>>> 3. Deactivate monitor 'Artisr15.6Pro':");
    force_activate_by_monitor_name("Artisr15.6Pro")?; // make sure it is present first
    {
        let mut topo = NativeTopology::acquire()?;
        if let Ok(mut editor) = topo.edit_output(&DisplayId("4356".into())) {
            editor.set_enabled(false)?;
        }
        topo.set_persistence(true);
        topo.commit().await?;
        println!("  [OK] Artisr15.6Pro deactivated");
    }

    // 4. Activate with position and resolution
    println!("\n>>> 4. Activate 'Artisr15.6Pro' at (1920,0) 1920x1080:");
    {
        let mut topo = NativeTopology::acquire()?;
        if let Ok(mut editor) = topo.edit_output(&DisplayId("4356".into())) {
            editor.set_enabled(true)?;
            editor.set_position(Point2D { x: 1920, y: 0 })?;
            editor.set_resolution(Extent2D {
                width: 1920,
                height: 1080,
            })?;
        }
        topo.set_persistence(true);
        topo.commit().await?;
        println!("  [OK] Artisr15.6Pro activated and positioned");
    }

    println!("\n================================================");
    println!("  Done");
    println!("================================================");
    Ok(())
}
