use df_displmgr::NativeTopology;
use df_displmgr::traits::UniversalTopology;

#[tokio::main]
async fn main() -> df_displmgr::DisplayResult<()> {
    println!("Querying current display topology...");
    
    // Acquire the platform-specific topology handle
    let topo = NativeTopology::acquire()?;
    
    let outputs = topo.get_outputs();
    println!("Found {} active output(s):", outputs.len());
    
    for out in outputs {
        println!("- ID: {} | Name: {} | Resolution: {}x{} | Position: ({}, {})", 
            out.identity.id.0, 
            out.identity.monitor_name, 
            out.geometry.size.width, 
            out.geometry.size.height, 
            out.geometry.origin.x, 
            out.geometry.origin.y
        );
    }
    
    Ok(())
}
