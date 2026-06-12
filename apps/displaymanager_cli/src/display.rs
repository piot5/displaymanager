//! CLI glue for `df_displmgr`: topology acquisition, edit, commit.
//!
//! Topology acquisition, output editing, and commit through the
//! `UniversalTopology` trait. Resolution, position, rotation, and
//! enable/disable are supported.
//!
//! Note: `apply` and the parse helpers are kept for reference. The active
//! CLI path uses `set::apply_all_settings()`.

use crate::cli::DisplayArgs;
use crate::info;
use anyhow::Context;
use df_displmgr::traits::UniversalTopology;
use df_displmgr::types::{DisplayId, DisplayRotation, Extent2D, Point2D};
use df_displmgr::NativeTopology;

#[allow(dead_code)]
pub async fn apply(args: DisplayArgs) -> anyhow::Result<()> {
    let output = args
        .output
        .as_ref()
        .context("--output is required for display configuration")?;

    let target = info::resolve_monitor(output)?;
    let mut topology: NativeTopology = NativeTopology::acquire()?;

    let mut editor = topology
        .edit_output(&DisplayId(target.device_path.clone()))
        .with_context(|| {
            format!(
                "Failed to acquire editor for '{}'. Device path may be locked or invalid.",
                target.friendly_name
            )
        })?;

    if let Some(mode_str) = args.mode {
        let (width, height) = parse_resolution(&mode_str)?;
        editor.set_resolution(Extent2D { width, height })?;
        println!(
            "Resolution set to {}x{} for {}",
            width, height, target.friendly_name
        );
    }

    if let Some(pos_str) = args.pos {
        let (x, y) = parse_position(&pos_str)?;
        editor.set_position(Point2D { x, y })?;
        println!("Position set to {}x{} for {}", x, y, target.friendly_name);
    }

    if let Some(rot_str) = args.rotate {
        let rotation = parse_rotation(&rot_str);
        editor.set_rotation(rotation)?;
        println!(
            "Rotation set to {:?} for {}",
            rotation, target.friendly_name
        );
    }

    // The old --off flag is replaced by the --mode-type off system.
    // enabled is kept true here because this module is deprecated
    // in favor of set.rs.
    editor.set_enabled(true)?;
    println!("Enabled signal output for {}", target.friendly_name);

    drop(editor);
    topology
        .commit()
        .await
        .context("Topology commit rejected by OS")?;
    println!("Configuration successfully applied.");
    Ok(())
}

#[allow(dead_code)]
fn parse_resolution(mode_str: &str) -> anyhow::Result<(u32, u32)> {
    let parts: Vec<&str> = mode_str.split('x').collect();
    anyhow::ensure!(
        parts.len() == 2,
        "Invalid mode '{mode_str}', expected WIDTHxHEIGHT"
    );
    let width = parts[0].parse::<u32>().context("Invalid width")?;
    let height = parts[1].parse::<u32>().context("Invalid height")?;
    Ok((width, height))
}

#[allow(dead_code)]
fn parse_position(pos_str: &str) -> anyhow::Result<(i32, i32)> {
    let parts: Vec<&str> = pos_str.split(|c| c == 'x' || c == 'X' || c == ',').collect();
    anyhow::ensure!(
        parts.len() == 2,
        "Invalid position '{pos_str}', expected XxY or X,Y"
    );
    let x = parts[0].parse::<i32>().context("Invalid X position")?;
    let y = parts[1].parse::<i32>().context("Invalid Y position")?;
    Ok((x, y))
}

#[allow(dead_code)]
fn parse_rotation(rot_str: &str) -> DisplayRotation {
    match rot_str.trim().trim_end_matches("deg") {
        "0" | "Rotate0" | "rotate0" => DisplayRotation::Rotate0,
        "90" | "Rotate90" | "rotate90" => DisplayRotation::Rotate90,
        "180" | "Rotate180" | "rotate180" => DisplayRotation::Rotate180,
        "270" | "Rotate270" | "rotate270" => DisplayRotation::Rotate270,
        other => {
            println!("Unrecognized rotation '{other}', using 0");
            DisplayRotation::Rotate0
        }
    }
}
