use clap::Parser;
use displaymanager_cli::cli::{Cli, Commands, OutputFormat};
use displaymanager_cli::{ddc, info, set};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::List { scan, output } => run_list(*scan, output.as_deref(), cli.format.as_ref()),
        Commands::Info { id, edid_file } => run_info(id, edid_file.as_deref()),
        Commands::Set {
            id,
            mode,
            cloned,
            res,
            topo,
            freq,
            rotate,
            hdr,
            scale,
            primary,
            dry_run,
        } => run_set(
            id, mode, cloned, res, topo, freq, rotate, hdr, scale, *primary, *dry_run,
        ),
        Commands::Ddc {
            id,
            brightness,
            contrast,
            volume,
            input,
            power,
            dry_run,
        } => run_ddc(id, brightness, contrast, volume, input, power, *dry_run),
        Commands::Activate {
            id,
            resolution,
            position,
            dry_run,
        } => run_activate(id, resolution, position, *dry_run),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_list(
    scan: bool,
    output: Option<&std::path::Path>,
    format: Option<&OutputFormat>,
) -> anyhow::Result<()> {
    let fmt = match format {
        Some(OutputFormat::Json) => Some(OutputFormat::Json),
        _ => None,
    };

    info::scan(fmt)?;

    if scan || output.is_some() {
        info::write_edid_report("edid_dump.txt")?;
    }

    if let Some(path) = output {
        let path_str = path.to_str().unwrap_or("scan_output.json");
        info::write_edid_json(path_str)?;
    }

    Ok(())
}

fn run_info(id: &str, edid_file: Option<&std::path::Path>) -> anyhow::Result<()> {
    if let Some(path) = edid_file {
        let path_str = path.to_str().unwrap_or("");
        info::monitor_info_from_edid(id, path_str)?;
    } else {
        info::monitor_info(id, None)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_set(
    id: &str,
    mode: &Option<String>,
    cloned: &Option<String>,
    res: &Option<String>,
    topo: &Option<String>,
    freq: &Option<u32>,
    rotate: &Option<String>,
    hdr: &Option<String>,
    scale: &Option<f64>,
    primary: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let args = set::SetArgs {
        output: id.to_string(),
        mode_type: mode.clone(),
        clone_from: cloned.clone(),
        mode: res.clone(),
        pos: topo.clone(),
        rotate: rotate.clone(),
        auto_pos: false,
        ccd_wake: false,
        refresh_rate: *freq,
        hdr: hdr.clone(),
        scale: *scale,
        primary,
        verify_only: dry_run,
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set::apply(id, &args))?;
    Ok(())
}

fn run_ddc(
    id: &str,
    brightness: &Option<u32>,
    contrast: &Option<u32>,
    volume: &Option<u32>,
    input: &Option<String>,
    power: &Option<String>,
    dry_run: bool,
) -> anyhow::Result<()> {
    ddc::apply_ddc(
        id,
        *brightness,
        *contrast,
        *volume,
        input.as_deref(),
        power.as_deref(),
        dry_run,
    )?;
    Ok(())
}

fn run_activate(
    id: &str,
    resolution: &Option<String>,
    position: &Option<String>,
    dry_run: bool,
) -> anyhow::Result<()> {
    // Resolve the monitor first - fails with "not found" if monitor doesn't exist
    let _target = info::resolve_monitor(id)?;

    let args = set::SetArgs {
        output: id.to_string(),
        mode_type: Some("extended".to_string()),
        clone_from: None,
        mode: resolution.clone(),
        pos: position.clone(),
        rotate: None,
        auto_pos: false,
        ccd_wake: false,
        refresh_rate: None,
        hdr: None,
        scale: None,
        primary: false,
        verify_only: dry_run,
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set::apply(id, &args))?;
    Ok(())
}
