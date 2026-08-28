use df_ddc::list_monitors;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitors = list_monitors();
    if monitors.is_empty() {
        println!("No DDC/CI-capable monitors found.");
        return Ok(());
    }
    for (idx, dev) in monitors.iter().enumerate() {
        println!("[{}] {}", idx, dev.info);
    }
    Ok(())
}