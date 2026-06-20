mod cli;
mod ddc;
mod info;
mod set;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // --scan: hardware inventory
    if let Some(save_path) = &cli.scan {
        info::write_edid_report("edid_dump.txt")?;
        if let Some(ref path) = save_path {
            info::write_edid_json(path)?;
        } else {
            info::scan()?;
        }
        return Ok(());
    }

    // All other operations require --id
    let id = match &cli.id {
        Some(id) => id.clone(),
        None => {
            // No --id and no --scan: just show scan
            info::scan()?;
            return Ok(());
        }
    };

    // ── DDC/CI operations (no topology change needed) ──
    if cli.brightness.is_some()
        || cli.contrast.is_some()
        || cli.volume.is_some()
        || cli.input.is_some()
        || cli.power.is_some()
    {
        ddc::apply_ddc(
            &id,
            cli.brightness,
            cli.contrast,
            cli.volume,
            cli.input.as_deref(),
            cli.power.as_deref(),
        )?;
        return Ok(());
    }

    // ── Topology operations ──
    // Determine mode from --off, --mode, or --cloned
    let mode_type = if cli.off {
        Some("off".to_string())
    } else {
        cli.mode.clone()
    };
    let clone_from = cli.cloned.clone();

    // Pass refresh rate directly (in mHz as provided by user, e.g. 144000 for 144 Hz)
    let refresh_rate = cli.freq;

    // Build SetArgs (--verify takes precedence as dry-run)
    let args = set::SetArgs {
        output: id.clone(),
        mode_type,
        clone_from,
        mode: cli.res,
        pos: cli.topo,
        rotate: cli.rotate,
        auto_pos: false,
        ccd_wake: false,
        refresh_rate,
        hdr: cli.hdr,
        scale: cli.scale,
        primary: cli.primary,
        verify_only: cli.verify,
    };

    set::apply(&id, &args).await?;

    Ok(())
}
