use df_displmgr::error::DisplayResult;
use df_displmgr::{NativeTopology, UniversalTopology};

#[tokio::main]
async fn main() -> DisplayResult<()> {
    // Acquire topology
    let topo = NativeTopology::acquire()?;
    let outputs = topo.get_outputs();

    println!("Detected monitors in the system:");
    println!("--------------------------------------------------");

    for (index, output) in outputs.iter().enumerate() {
        // Print details for each monitor
        println!("Index: {}", index);
        println!("Name:  {}", output.identity.monitor_name);
        println!("ID:    {:?}", output.identity.id); // This prints the canonical ID format
        println!("Enabled: {}", output.enabled);
        println!("--------------------------------------------------");
    }

    Ok(())
}
