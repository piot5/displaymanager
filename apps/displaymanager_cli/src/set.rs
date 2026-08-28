//! Display settings management — integrated from CLI examples.
//!
//! Provides topology edit/commit, CCD wake, clone, extended, off,
//! automatic positioning, GDI activation, and verify-only analysis.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_args_default() {
        let args = SetArgs::default();
        assert!(args.mode_type.is_none());
        assert!(args.clone_from.is_none());
        assert!(!args.auto_pos);
        assert!(!args.ccd_wake);
        assert!(!args.primary);
        assert!(!args.verify_only);
    }

    #[test]
    fn test_set_args_with_values() {
        let args = SetArgs {
            output: "HDMI-1".to_string(),
            mode_type: Some("cloned".to_string()),
            clone_from: Some("DP-1".to_string()),
            mode: Some("1920x1080".to_string()),
            pos: Some("0,0".to_string()),
            rotate: Some("90".to_string()),
            auto_pos: true,
            ccd_wake: true,
            refresh_rate: Some(144),
            hdr: Some("enabled".to_string()),
            scale: Some(1.5),
            primary: true,
            verify_only: false,
        };
        assert_eq!(args.output, "HDMI-1");
        assert_eq!(args.mode_type, Some("cloned".to_string()));
        assert!(args.auto_pos);
        assert!(args.ccd_wake);
        assert!(args.primary);
    }

    #[test]
    fn test_clone_tolerance_constant() {
        assert_eq!(CLONE_TOLERANCE, 4);
    }

    #[test]
    fn test_parse_resolution_valid() {
        let (w, h) = parse_resolution("1920x1080").unwrap();
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
    }

    #[test]
    fn test_parse_resolution_invalid() {
        assert!(parse_resolution("1920-1080").is_err());
        assert!(parse_resolution("1920").is_err());
    }

    #[test]
    fn test_parse_position_comma() {
        let (x, y) = parse_position("100,200").unwrap();
        assert_eq!(x, 100);
        assert_eq!(y, 200);
    }

    #[test]
    fn test_parse_position_x() {
        let (x, y) = parse_position("100x200").unwrap();
        assert_eq!(x, 100);
        assert_eq!(y, 200);
    }

    #[test]
    fn test_parse_position_invalid() {
        assert!(parse_position("abc").is_err());
    }

    #[test]
    fn test_parse_rotation_valid() {
        assert_eq!(parse_rotation("0").unwrap(), DisplayRotation::Rotate0);
        assert_eq!(parse_rotation("90").unwrap(), DisplayRotation::Rotate90);
        assert_eq!(parse_rotation("180").unwrap(), DisplayRotation::Rotate180);
        assert_eq!(parse_rotation("270").unwrap(), DisplayRotation::Rotate270);
    }

    #[test]
    fn test_parse_rotation_invalid() {
        assert!(parse_rotation("45").is_err());
        assert!(parse_rotation("abc").is_err());
    }

    #[test]
    fn test_rects_overlap_true() {
        assert!(rects_overlap(0, 0, 100, 100, 50, 50, 100, 100));
    }

    #[test]
    fn test_rects_overlap_false() {
        assert!(!rects_overlap(0, 0, 100, 100, 200, 200, 100, 100));
    }

    #[test]
    fn test_is_mode_active_true() {
        let args = SetArgs {
            mode_type: Some("extended".to_string()),
            ..Default::default()
        };
        assert!(is_mode_active(&args));
    }

    #[test]
    fn test_is_mode_active_false() {
        let args = SetArgs {
            mode_type: Some("off".to_string()),
            ..Default::default()
        };
        assert!(!is_mode_active(&args));
    }

    #[test]
    fn test_resolve_mode_type_off() {
        let args = SetArgs {
            mode_type: Some("off".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_mode_type(&args), "off");
    }

    #[test]
    fn test_resolve_mode_type_cloned() {
        let args = SetArgs {
            clone_from: Some("DP-1".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_mode_type(&args), "cloned");
    }

    #[test]
    fn test_resolve_mode_type_extended_default() {
        let args = SetArgs::default();
        assert_eq!(resolve_mode_type(&args), "extended");
    }
}

use crate::info;
use anyhow::{bail, Context, Result};
use df_displmgr::types::{DisplayId, DisplayRotation, Extent2D, Point2D};
use df_displmgr::{NativeTopology, UniversalTopology};

/// Maximum allowed geometry difference for clone detection (pixels).
const CLONE_TOLERANCE: i32 = 4;

#[derive(Debug, Clone, Default)]
pub struct SetArgs {
    #[allow(dead_code)]
    pub output: String,
    pub mode_type: Option<String>,
    pub clone_from: Option<String>,
    pub mode: Option<String>,
    pub pos: Option<String>,
    pub rotate: Option<String>,
    pub auto_pos: bool,
    pub ccd_wake: bool,
    pub refresh_rate: Option<u32>,
    pub hdr: Option<String>,
    pub scale: Option<f64>,
    pub primary: bool,
    pub verify_only: bool,
}

// Public API

/// Main topology apply function — dispatches based on mode_type.
pub async fn apply(output_id_str: &str, args: &SetArgs) -> Result<()> {
    let target = info::resolve_monitor(output_id_str)?;
    let friendly = target.friendly_name.trim().to_string();

    // --verify: read-only check
    if args.verify_only {
        return verify_topology(&target);
    }

    // ccd_wake: wake inactive display
    if args.ccd_wake || (!target.is_active && is_mode_active(args)) {
        ccd_wake_display(target.target_id)?;
    }

    let display_id = DisplayId(target.target_id.to_string());
    let mode = resolve_mode_type(args);

    // Pre-calculate auto-position BEFORE acquiring the editor (borrow avoidance)
    let auto_pos = calc_auto_position(args, &mode);

    let mut topology =
        <NativeTopology as UniversalTopology>::acquire().context("Failed to acquire topology")?;

    // Apply editor changes
    apply_editor_inline(&mut topology, &display_id, args, &auto_pos, &mode)?;

    // Validate + commit
    println!("  Validating...");
    match topology.validate().await {
        Ok(_) => {
            println!("  Validation OK, committing...");
            topology.set_persistence(true);
            topology.commit().await.context("Commit rejected by OS")?;
            println!("  [OK] Commit OK");
        }
        Err(e) => {
            eprintln!("  [WARN] Validation warning: {e}");
            println!("  Attempting commit anyway...");
            topology.commit().await.context("Commit rejected by OS")?;
            println!("  [OK] Commit OK (despite validation warning)");
        }
    }

    // Post-commit verification
    verify_post_commit_extended(&friendly, &mode)?;
    Ok(())
}

/// Inline editor operations
fn apply_editor_inline(
    topology: &mut NativeTopology,
    display_id: &DisplayId,
    args: &SetArgs,
    auto_pos: &Option<(i32, i32)>,
    mode: &str,
) -> Result<()> {
    let mut editor = topology
        .edit_output(display_id)
        .with_context(|| "Cannot edit output")?;

    match mode {
        "cloned" => {
            let src_id = args
                .clone_from
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("--mode cloned requires --cloned <source>"))?;
            println!("  Mode: CLONED — copying from '{}'", src_id);
            editor
                .clone_from(&DisplayId(src_id.clone()))
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
                editor
                    .set_resolution(Extent2D {
                        width: w,
                        height: h,
                    })
                    .context("set_resolution failed")?;
                println!("  Resolution: {}x{}", w, h);
            }
            let final_pos = if let Some(ref pos) = args.pos {
                Some(parse_position(pos)?)
            } else {
                *auto_pos
            };
            if let Some((x, y)) = final_pos {
                editor
                    .set_position(Point2D { x, y })
                    .context("set_position failed")?;
                println!("  Position: ({}, {})", x, y);
            }
            if let Some(ref rot) = args.rotate {
                let rotation = parse_rotation(rot)?;
                editor
                    .set_rotation(rotation)
                    .context("set_rotation failed")?;
                println!("  Rotation: {:?}", rotation);
            }
            if let Some(rr) = args.refresh_rate {
                editor
                    .set_refresh_rate(rr)
                    .context("set_refresh_rate failed")?;
                println!("  Refresh rate: {} mHz ({} Hz)", rr, rr as f64 / 1000.0);
            }
            if let Some(ref hdr_val) = args.hdr {
                let (state, mode) = match hdr_val.to_lowercase().as_str() {
                    "on" | "enable" => (
                        df_displmgr::types::HdrState::Enabled,
                        df_displmgr::types::HdrMode::Default,
                    ),
                    "off" | "disable" => (
                        df_displmgr::types::HdrState::Disabled,
                        df_displmgr::types::HdrMode::Default,
                    ),
                    _ => bail!("Invalid --hdr value '{hdr_val}', use 'on' or 'off'"),
                };
                editor.set_hdr(state, mode).context("set_hdr failed")?;
                println!("  HDR: {:?}", state);
            }
            if let Some(scale_val) = args.scale {
                if !(0.25..=5.0).contains(&scale_val) {
                    bail!("--scale must be between 0.25 and 5.0, got {scale_val}");
                }
                editor.set_scale(scale_val).context("set_scale failed")?;
                println!("  Scale: {:.0}%", scale_val * 100.0);
            }
            if args.primary {
                editor.set_primary().context("set_primary failed")?;
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
        if lower == "off" || lower == "cloned" || lower == "ext" || lower == "extended" {
            if lower == "ext" || lower == "extended" {
                return "extended".to_string();
            }
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
        return None; // explicit --topo overrides
    }
    if !args.auto_pos && args.mode.is_none() {
        return None; // no auto-position needed
    }
    // Find rightmost active X from df_displmgr_info
    let rightmost_x = df_displmgr_info::collect_monitor_data()
        .ok()
        .map(|monitors| {
            monitors
                .iter()
                .filter(|m| m.is_active)
                .filter_map(|m| m.topology.as_ref())
                .map(|t| t.x.saturating_add(t.width as i32))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    println!(
        "  Auto-position: x={} (right of rightmost monitor)",
        rightmost_x
    );
    Some((rightmost_x, 0i32))
}

// Verify-only

fn verify_topology(target: &df_displmgr_info::MonitorDetails) -> Result<()> {
    println!("--- Verify-Only: Topology Analysis ---");
    println!(
        "  Target: '{}' (id={}) active={}",
        target.friendly_name.trim(),
        target.target_id,
        target.is_active
    );
    if let Some(ref t) = target.topology {
        println!(
            "  Position: ({}, {})  Size: {}x{}  Rotation: {}",
            t.x, t.y, t.width, t.height, t.rotation
        );
    }
    info::scan(None)?;
    Ok(())
}

// Post-commit verification

fn verify_post_commit_extended(friendly_name: &str, mode: &str) -> Result<()> {
    let monitors = match df_displmgr_info::collect_monitor_data() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  Verification: {e}");
            return Ok(());
        }
    };
    let qq = friendly_name.to_lowercase();
    let target = match monitors
        .iter()
        .find(|m| m.friendly_name.trim().to_lowercase().contains(&qq))
    {
        Some(t) => t,
        None => {
            println!("  [WARN] Target '{friendly_name}' not found");
            return Ok(());
        }
    };
    println!("\n--- Post-Commit Verification ---");
    if mode == "off" {
        println!(
            "  {} -> {}",
            if !target.is_active {
                "[OK] OFF"
            } else {
                "[WARN] STILL ACTIVE"
            },
            friendly_name
        );
        return Ok(());
    }
    if !target.is_active {
        println!("  [WARN] '{friendly_name}' is inactive");
        return Ok(());
    }
    let topo = match &target.topology {
        Some(t) => t,
        None => {
            println!("  [WARN] No topology");
            return Ok(());
        }
    };
    println!(
        "  Position: ({}, {})  Size: {}x{}  Rotation: {}",
        topo.x, topo.y, topo.width, topo.height, topo.rotation
    );
    // Check overlaps
    let overlaps: Vec<String> = monitors
        .iter()
        .filter(|m| m.is_active && m.target_id != target.target_id)
        .filter_map(|m| m.topology.as_ref().map(|t| (m, t)))
        .filter(|(_, ot)| {
            rects_overlap(
                topo.x,
                topo.y,
                topo.width,
                topo.height,
                ot.x,
                ot.y,
                ot.width,
                ot.height,
            )
        })
        .map(|(m, _)| format!("'{}'", m.friendly_name.trim()))
        .collect();
    if mode == "cloned" {
        if !overlaps.is_empty() {
            println!("  [OK] CLONED — shares geometry: {}", overlaps.join(", "));
        } else {
            println!("  [WARN] CLONED requested but no overlap");
        }
    } else if overlaps.is_empty() {
        println!(
            "  [OK] EXTENDED — unique geometry at ({}, {})",
            topo.x, topo.y
        );
    } else {
        println!("  [WARN] EXTENDED but overlaps: {}", overlaps.join(", "));
    }
    Ok(())
}

// CCD Wake delegates to the library implementation

pub fn ccd_wake_display(target_id: u32) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use df_displmgr::backends::windows::ccd_wake_display as lib_ccd_wake;
        println!(
            "  CCD Wake: activating display {} via SetDisplayConfig...",
            target_id
        );
        lib_ccd_wake(target_id).map_err(|e| anyhow::anyhow!("CCD wake failed: {e}"))?;
        println!("    Wake OK — target_id={} now active", target_id);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target_id;
        Err(anyhow::anyhow!("CCD Wake is only supported on Windows"))
    }
}

// Parser helpers

pub fn parse_resolution(s: &str) -> anyhow::Result<(u32, u32)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        bail!("Invalid resolution: {s} (expected WxH)");
    }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

pub fn parse_position(s: &str) -> anyhow::Result<(i32, i32)> {
    let parts: Vec<&str> = if s.contains('x') {
        s.split('x').collect()
    } else if s.contains(',') {
        s.split(',').collect()
    } else {
        bail!("Invalid position: {s} (expected XxY or X,Y)");
    };
    if parts.len() != 2 {
        bail!("Invalid position: {s}");
    }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

pub fn parse_rotation(s: &str) -> anyhow::Result<DisplayRotation> {
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
    (ax < b_x2 - CLONE_TOLERANCE)
        && (a_x2 > bx + CLONE_TOLERANCE)
        && (ay < b_y2 - CLONE_TOLERANCE)
        && (a_y2 > by + CLONE_TOLERANCE)
}
