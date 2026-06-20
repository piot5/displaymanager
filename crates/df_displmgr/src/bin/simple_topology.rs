use df_displmgr::{traits::UniversalTopology, NativeTopology};

fn main() -> df_displmgr::DisplayResult<()> {
    let topo = NativeTopology::acquire()?;
    for output in topo.get_outputs() {
        let geo = output.geometry;
        println!(
            "{}: {}x{} at ({}, {}), enabled={}, rotation={:?}",
            output.identity.monitor_name,
            geo.size.width,
            geo.size.height,
            geo.origin.x,
            geo.origin.y,
            output.enabled,
            output.rotation
        );
    }
    Ok(())
}