//! Display settings management — integrated from CLI examples.
//!
//! Provides topology edit/commit, CCD wake, clone, extended, off,
//! automatic positioning, GDI activation, and verify-only analysis.
//!
//! Solutions drawn from:
//! - test_extend.rs    -> --mode-type extended|cloned|off, clone_from, verify-only, post-commit verification
//! - test_activate_ccd.rs -> CCD Wake via SetDisplayConfig, auto-position
//! - test.rs           -> batch horizontal layout, set_refresh_rate, set_persistence
//! - test_on.rs        -> GDI activation (via force_all / force_activate_by_monitor_name)

use anyhow::{bail, Context, Result};
use crate::cli::SetArgs;
use crate::info;
use df_displmgr::types::{DisplayId, Extent2D, Point2D, DisplayRotation};
use df_displmgr::{NativeTopology, UniversalTopology};

/// Maximum allowed geometry difference for clone detection (pixels).
const CLONE_TOLERANCE: i32 = 4;

// Public API

/// Main topology apply function — dispatches based on --mode-type.
pub async fn apply(output_id_str: &str, args: &SetArgs) -> Result<()> {
    let target = info::resolve_monitor(output_id_str)?;
    let friendly = target.friendly_name.trim().to_string();

    // --verify-only: read-only check (from test_extend.rs)
    if args.verify_only {
        return verify_topology(&target);
    }

    // --ccd-wake: wake inactive display (from test_activate_ccd.rs)
    if args.ccd_wake || (!target.is_active && is_mode_active(args)) {
        ccd_wake_display(target.target_id)?;
    }

    let display_id = DisplayId(target.target_id.to_string());
    let mode = resolve_mode_type(args);

    // Pre-calculate auto-position BEFORE acquiring the editor (borrow avoidance)
    let auto_pos = calc_auto_position(args, &mode);

    let mut topology = <NativeTopology as UniversalTopology>::acquire()
        .context("Failed to acquire topology")?;

    // Apply editor changes (editor dropped inside function — topology released on return)
    apply_editor_inline(&mut topology, &display_id, args, &auto_pos, &mode)?;

    // Validate + commit
    println!("  Validating...");
    match topology.validate().await {
        Ok(_) => {
            println!("  Validation OK, committing...");
            topology.set_persistence(true);
            topology.commit().await
                .context("Commit rejected by OS")?;
            println!("  [OK] Commit OK");
        }
        Err(e) => {
            eprintln!("  [WARN] Validation warning: {e}");
            println!("  Attempting commit anyway...");
            topology.commit().await
                .context("Commit rejected by OS")?;
            println!("  [OK] Commit OK (despite validation warning)");
        }
    }

    // Post-commit verification (from test_extend.rs verify_mode)
    verify_post_commit_extended(&friendly, &mode)?;
    Ok(())
}

/// Inline editor operations — avoids passing Box<dyn OutputEditable + '_> between functions.
fn apply_editor_inline(
    topology: &mut NativeTopology,
    display_id: &DisplayId,
    args: &SetArgs,
    auto_pos: &Option<(i32, i32)>,
    mode: &str,
) -> Result<()> {
    let mut editor = topology.edit_output(display_id)
        .with_context(|| "Cannot edit output")?;

    match mode {
        "cloned" => {
            let src_id = args.clone_from.as_ref()
                .ok_or_else(|| anyhow::anyhow!("--mode-type cloned requires --clone-from <source>"))?;
            println!("  Mode: CLONED — copying from '{}'", src_id);
            editor.clone_from(&DisplayId(src_id.clone()))
                .context("clone_from failed")?;
            println!("  Clone source: {}", src_id);
        }
        "off" => {
            println!("  Mode: OFF — disabling display");
            editor.set_enabled(false)?;
            println!("  Display disabled");
            return Ok(());
        }
        _ => {
            println!("  Mode: EXTENDED");
            if let Some(ref mode_str) = args.mode {
                let (w, h) = parse_resolution(mode_str)?;
                editor.set_resolution(Extent2D { width: w, height: h })
                    .context("set_resolution failed")?;
                println!("  Resolution: {}x{}", w, h);
            }
            let final_pos = if let Some(ref pos) = args.pos {
                Some(parse_position(pos)?)
            } else {
                *auto_pos
            };
            if let Some((x, y)) = final_pos {
                editor.set_position(Point2D { x, y })
                    .context("set_position failed")?;
                println!("  Position: ({}, {})", x, y);
            }
            if let Some(ref rot) = args.rotate {
                let rotation = parse_rotation(rot)?;
                editor.set_rotation(rotation)
                    .context("set_rotation failed")?;
                println!("  Rotation: {:?}", rotation);
            }
            if let Some(rr) = args.refresh_rate {
                editor.set_refresh_rate(rr)
                    .context("set_refresh_rate failed")?;
                println!("  Refresh rate: {} mHz ({} Hz)", rr, rr as f64 / 1000.0);
            }
            if let Some(ref hdr_val) = args.hdr {
                let (state, mode) = match hdr_val.to_lowercase().as_str() {
                    "on" | "enable" => (df_displmgr::types::HdrState::Enabled, df_displmgr::types::HdrMode::Default),
                    "off" | "disable" => (df_displmgr::types::HdrState::Disabled, df_displmgr::types::HdrMode::Default),
                    _ => bail!("Invalid --hdr value '{hdr_val}', use 'on' or 'off'"),
                };
                editor.set_hdr(state, mode)
                    .context("set_hdr failed")?;
                println!("  HDR: {:?}", state);
            }
            if let Some(scale_val) = args.scale {
                if !(0.25..=5.0).contains(&scale_val) {
                    bail!("--scale must be between 0.25 and 5.0, got {scale_val}");
                }
                editor.set_scale(scale_val)
                    .context("set_scale failed")?;
                println!("  Scale: {:.0}%", scale_val * 100.0);
            }
            if args.primary {
                editor.set_primary()
                    .context("set_primary failed")?;
                println!("  Primary: true");
            }
            editor.set_enabled(true)?;
            println!("  Enabled: true");
        }
    }
    // Editor dropped here — topology borrow released
    Ok(())
}

/// Resolve the effective mode type from args.
fn resolve_mode_type(args: &SetArgs) -> String {
    if let Some(ref mt) = args.mode_type {
        let lower = mt.to_lowercase();
        if lower == "off" || lower == "cloned" || lower == "extended" {
            return lower;
        }
    }
    if args.clone_from.is_some() {
        return "cloned".to_string();
    }
    "extended".to_string()
}

/// Returns true if the mode is active (not off).
pub fn is_mode_active(args: &SetArgs) -> bool {
    resolve_mode_type(args) != "off"
}

/// Calculate auto-position using df_displmgr_info (no borrow conflict).
fn calc_auto_position(args: &SetArgs, mode: &str) -> Option<(i32, i32)> {
    if mode != "extended" {
        return None;
    }
    if args.pos.is_some() {
        return None; // explicit --pos overrides
    }
    if !args.auto_pos && args.mode.is_none() {
        return None; // no auto-position needed
    }
    // Find rightmost active X from df_displmgr_info (from test_activate_ccd.rs)
    let rightmost_x = df_displmgr_info::collect_monitor_data()
        .ok()
        .map(|monitors| {
            monitors.iter()
                .filter(|m| m.is_active)
                .filter_map(|m| m.topology.as_ref())
                .map(|t| t.x.saturating_add(t.width as i32))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    println!("  Auto-position: x={} (right of rightmost monitor)", rightmost_x);
    Some((rightmost_x, 0i32))
}

// Verify-only (from test_extend.rs)

fn verify_topology(target: &df_displmgr_info::MonitorDetails) -> Result<()> {
    println!("--- Verify-Only: Topology Analysis ---");
    println!("  Target: '{}' (id={}) active={}",
        target.friendly_name.trim(), target.target_id, target.is_active);
    if let Some(ref t) = target.topology {
        println!("  Position: ({}, {})  Size: {}x{}  Rotation: {}",
            t.x, t.y, t.width, t.height, t.rotation);
    }
    info::scan()?;
    Ok(())
}

// Post-commit verification (from test_extend.rs verify_mode)

fn verify_post_commit_extended(friendly_name: &str, mode: &str) -> Result<()> {
    let monitors = match df_displmgr_info::collect_monitor_data() {
        Ok(m) => m,
        Err(e) => { eprintln!("  Verification: {e}"); return Ok(()); }
    };
    let qq = friendly_name.to_lowercase();
    let target = match monitors.iter().find(|m| m.friendly_name.trim().to_lowercase().contains(&qq)) {
        Some(t) => t,
        None => { println!("  [WARN] Target '{friendly_name}' not found"); return Ok(()); }
    };
    println!("\n--- Post-Commit Verification ---");
    if mode == "off" {
        println!("  {} -> {}", if !target.is_active { "[OK] OFF" } else { "[WARN] STILL ACTIVE" }, friendly_name);
        return Ok(());
    }
    if !target.is_active {
        println!("  [WARN] '{friendly_name}' is inactive");
        return Ok(());
    }
    let topo = match &target.topology { Some(t) => t, None => { println!("  [WARN] No topology"); return Ok(()); } };
    println!("  Position: ({}, {})  Size: {}x{}  Rotation: {}", topo.x, topo.y, topo.width, topo.height, topo.rotation);
    // Check overlaps
    let overlaps: Vec<String> = monitors.iter()
        .filter(|m| m.is_active && m.target_id != target.target_id)
        .filter_map(|m| m.topology.as_ref().map(|t| (m, t)))
        .filter(|(_, ot)| rects_overlap(topo.x, topo.y, topo.width, topo.height, ot.x, ot.y, ot.width, ot.height))
        .map(|(m, _)| format!("'{}'", m.friendly_name.trim()))
        .collect();
    if mode == "cloned" {
        if !overlaps.is_empty() { println!("  [OK] CLONED — shares geometry: {}", overlaps.join(", ")); }
        else { println!("  [WARN] CLONED requested but no overlap"); }
    } else if overlaps.is_empty() { println!("  [OK] EXTENDED — unique geometry at ({}, {})", topo.x, topo.y); }
    else { println!("  [WARN] EXTENDED but overlaps: {}", overlaps.join(", ")); }
    Ok(())
}

// CCD Wake (from test_activate_ccd.rs)

#[cfg(target_os = "windows")]
pub fn ccd_wake_display(target_id: u32) -> Result<()> {
    println!("  CCD Wake: activating display {} via SetDisplayConfig...", target_id);
    unsafe {
        use windows::Win32::Devices::Display::{
            DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_MODE_INFO,
            GetDisplayConfigBufferSizes, QueryDisplayConfig, SetDisplayConfig,
            QDC_ALL_PATHS, SDC_APPLY, SDC_SAVE_TO_DATABASE, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
        };
        use df_displmgr::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_PATH_ACTIVE;

        let mut pc = 0u32; let mut mc = 0u32;
        GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut pc, &mut mc)
            .context("GetDisplayConfigBufferSizes")?;
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); pc as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mc as usize];
        QueryDisplayConfig(QDC_ALL_PATHS, &mut pc, paths.as_mut_ptr(), &mut mc, modes.as_mut_ptr(), None)
            .context("QueryDisplayConfig")?;
        paths.truncate(pc as usize);
        let zi = paths.iter().position(|p| p.targetInfo.id == target_id)
            .ok_or_else(|| anyhow::anyhow!("Target path {} not found", target_id))?;
        if (paths[zi].flags & DISPLAYCONFIG_PATH_ACTIVE) != 0 {
            println!("    Already active, skipping CCD wake.");
            return Ok(());
        }
        if paths[zi].sourceInfo.Anonymous.modeInfoIdx == 0xFFFF_FFFF {
            bail!("  No valid source mode index");
        }
        paths[zi].flags |= DISPLAYCONFIG_PATH_ACTIVE;
        let st = SetDisplayConfig(Some(&paths), Some(&modes),
            SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_SAVE_TO_DATABASE);
        if st != 0 { bail!("CCD Wake failed with status 0x{:08X}", st as u32); }
        println!("    Wake OK — target_id={} now active", target_id);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn ccd_wake_display(_target_id: u32) -> Result<()> {
    Err(anyhow::anyhow!("CCD Wake is only supported on Windows"))
}

// Parser helpers

pub fn parse_resolution(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 { bail!("Invalid resolution: {s} (expected WxH)"); }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

pub fn parse_position(s: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = if s.contains('x') { s.split('x').collect() }
    else if s.contains(',') { s.split(',').collect() }
    else { bail!("Invalid position: {s} (expected XxY or X,Y)"); };
    if parts.len() != 2 { bail!("Invalid position: {s}"); }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

pub fn parse_rotation(s: &str) -> Result<DisplayRotation> {
    match s {
        "0" => Ok(DisplayRotation::Rotate0),
        "90" => Ok(DisplayRotation::Rotate90),
        "180" => Ok(DisplayRotation::Rotate180),
        "270" => Ok(DisplayRotation::Rotate270),
        _ => bail!("Invalid rotation: {s} (expected 0, 90, 180, or 270)"),
    }
}

#[allow(clippy::too_many_arguments)]
fn rects_overlap(ax: i32, ay: i32, aw: u32, ah: u32, bx: i32, by: i32, bw: u32, bh: u32) -> bool {
    let a_x2 = ax.saturating_add(aw as i32);
    let a_y2 = ay.saturating_add(ah as i32);
    let b_x2 = bx.saturating_add(bw as i32);
    let b_y2 = by.saturating_add(bh as i32);
    (ax < b_x2 - CLONE_TOLERANCE) && (a_x2 > bx + CLONE_TOLERANCE)
        && (ay < b_y2 - CLONE_TOLERANCE) && (a_y2 > by + CLONE_TOLERANCE)
}
