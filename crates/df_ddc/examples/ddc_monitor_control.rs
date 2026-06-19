use df_ddc::list_monitors;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning for DDC-capable monitors...");
    let monitors = list_monitors();

    if monitors.is_empty() {
        println!("No DDC-capable monitors found.");
        return Ok(());
    }

    // Use the first monitor found
    let dev = &monitors[0];
    println!("Controlling: {}", dev.info);

    // Set brightness to 70%
    println!("Setting brightness to 70%...");
    dev.inner.set_brightness(70)?;

    // Read current brightness back
    let (val, max) = dev.inner.get_vcp_feature(0x10)?; // 0x10 is Brightness
    println!("Current brightness: {} / {}", val, max);

    Ok(())
}
