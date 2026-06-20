//! Deep integration of `df_displmgr_info` into the CLI.
//!
//! Provides rich display scanning, EDID reporting, monitor lookup,
//! topology analysis, and DDC hardware telemetry — all sourced from
//! `df_displmgr_info::collect_monitor_data()`.
//!
//! Features:
//! - `scan()`        — list all monitors with topology, EDID, DDC stats
//! - `scan_json()`   — machine-readable JSON output of all monitor data
//! - `monitor_info()`— deep detail for a single display (used by `--output` with `--info` / `--edid-json`)
//! - `resolve_monitor()` — find monitor by name, ID, path, or GDI name
//! - `write_edid_report()` — plain-text EDID diagnostic dump
//! - `write_edid_json()`   — structured EDID + DDC + topology as JSON
//! - `format_output_tech()` — human-readable output technology strings
//! - Topology analysis: clone / extended detection (same logic as test_extend.rs)

use anyhow::{anyhow, Context};
use df_displmgr::traits::UniversalTopology;
use df_displmgr::NativeTopology;
use df_displmgr_info::edid_types::{DeepDdcStats, EdidData};
use df_displmgr_info::{collect_monitor_data, MonitorDetails};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Maximum allowed geometry difference for clone detection (pixels).
const CLONE_TOLERANCE: i32 = 4;

// ════════════════════════════════════════════════════════════════════
// Public API
// ════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
/// Full scan: list all monitors with topology, EDID, DDC, and OS state.
pub fn scan() -> anyhow::Result<()> {
    let monitors = collect_monitor_data().context("Hardware scan failed")?;
    if monitors.is_empty() {
        println!("No monitors detected on this system.");
        return Ok(());
    }

    // Acquire NativeTopology for enriched OutputState fields (primary, HDR, scale, etc.)
    let output_map = get_output_state_map();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                Display Hardware Report                  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("Total monitors found: {}\n", monitors.len());

    for (idx, m) in monitors.iter().enumerate() {
        let glyph = if m.is_active { "🖥" } else { "⏹" };

        // Look up enriched state from NativeTopology by matching target_id
        let state = output_map.get(&m.target_id);

        println!(
            "─── Monitor {} {} ───────────────────────────",
            idx + 1,
            glyph
        );
        println!(
            "  Name:          {}",
            clean_display_string(&m.friendly_name)
        );
        println!("  Target ID:     {}", m.target_id);
        println!("  Active:        {}", m.is_active);

        // OS-level state from OutputState (df_displmgr)
        if let Some(s) = state {
            println!("  Primary:       {}", s.is_primary);
            println!("  Connector:     {}", s.identity.connector_id.0);
            println!("  Adapter:       {}", s.identity.adapter_id.0);
            println!("  Refresh:       {:.1} Hz", s.refresh_rate as f64 / 1000.0);
            println!("  Scale:         {:.0}%", s.scale * 100.0);
            println!(
                "  HDR:           {}",
                match s.hdr_state {
                    df_displmgr::types::HdrState::Enabled => "Enabled",
                    _ => "Disabled",
                }
            );
        }

        println!("  GDI Name:      {}", m.gdi_name);
        println!("  Interface:     {}", format_output_tech(&m.output_tech));
        println!("  Device Path:   {}", m.device_path);

        // Topology from df_displmgr_info + native resolution from OutputState
        if let Some(ref t) = m.topology {
            println!(
                "  Topology:      {}x{} @ ({}, {})  rotation={}",
                t.width, t.height, t.x, t.y, t.rotation
            );
        }
        if let Some(s) = state {
            if let Some(nat) = s.native_resolution {
                println!("  Native Res:    {}x{}", nat.width, nat.height);
            }
        }

        // EDID summary
        if let Some(ref edid) = m.edid {
            println!("  EDID:");
            println!("    Model:       {}", edid.model_name);
            println!(
                "    Manufacturer:{}  Product:{}  Serial:{}",
                edid.manufacturer_id, edid.product_code, edid.serial_number_binary
            );
            println!(
                "    Manufactured: week {} / year {}",
                edid.week_of_manufacture, edid.year_of_manufacture
            );
            if let Some(ref chroma) = edid.chromaticity {
                println!(
                    "    Chromaticity: R({:.3},{:.3}) G({:.3},{:.3}) B({:.3},{:.3}) W({:.3},{:.3})",
                    chroma.red_x,
                    chroma.red_y,
                    chroma.green_x,
                    chroma.green_y,
                    chroma.blue_x,
                    chroma.blue_y,
                    chroma.white_x,
                    chroma.white_y
                );
            }
            if edid.hdr_caps.supports_hdr_traditional || edid.hdr_caps.supports_smpte_st2084 {
                println!(
                    "    HDR:         {} Traditional  {} HDR10  {} HLG",
                    if edid.hdr_caps.supports_hdr_traditional {
                        "✓"
                    } else {
                        "–"
                    },
                    if edid.hdr_caps.supports_smpte_st2084 {
                        "✓"
                    } else {
                        "–"
                    },
                    if edid.hdr_caps.supports_hlg {
                        "✓"
                    } else {
                        "–"
                    }
                );
                if let Some(lum) = edid.hdr_caps.max_luminance_cd_m2 {
                    println!("    Max Luminance: {:.0} cd/m²", lum);
                }
            }
            if !edid.modes.is_empty() {
                println!("    Modes:       {} available", edid.modes.len());
                // Show the first 3 native modes
                for mode in edid.modes.iter().take(3) {
                    println!(
                        "      {}x{} @ {} Hz{}",
                        mode.width,
                        mode.height,
                        mode.refresh_rate,
                        if mode.interlaced { "i" } else { "p" }
                    );
                }
                if edid.modes.len() > 3 {
                    println!("      ... and {} more", edid.modes.len() - 3);
                }
            }
        } else {
            println!("  EDID:         not available");
        }

        // Supported OS video modes from OutputState
        if let Some(s) = state {
            if !s.supported_modes.is_empty() {
                println!("  OS Modes:      {} available", s.supported_modes.len());
                for vm in s.supported_modes.iter().take(3) {
                    println!(
                        "    {}x{} @ {:.1} Hz",
                        vm.resolution.width,
                        vm.resolution.height,
                        vm.refresh_rate as f64 / 1000.0
                    );
                }
                if s.supported_modes.len() > 3 {
                    println!("    ... and {} more", s.supported_modes.len() - 3);
                }
            }
        }

        // DDC hardware stats (live from monitor via DDC/CI)
        if let Some(ref ddc) = m.ddc_stats {
            println!("  DDC Telemetry (live):");
            println!(
                "    Brightness:  {}/{}",
                ddc.core_caps.brightness, ddc.core_caps.brightness_max
            );
            println!(
                "    Contrast:    {}/{}",
                ddc.core_caps.contrast, ddc.core_caps.contrast_max
            );
            println!("    Input:       {:?}", ddc.input_source);
            println!("    Power:       {:?}", ddc.power_state);
            if let Some((vol, max_vol)) = ddc.volume {
                println!("    Volume:      {}/{}", vol, max_vol);
            }
            if let Some(hz) = ddc.horizontal_freq_hz {
                println!("    H-Freq:      {} Hz", hz);
            }
            if let Some(chz) = ddc.vertical_freq_centihz {
                println!("    V-Freq:      {:.2} Hz", chz as f64 / 100.0);
            }
            if let Some(hours) = ddc.operating_hours {
                println!("    Op Hours:    {}", hours);
            }
        }

        println!();
    }

    // Clone / Extended analysis (same algorithm as test_extend.rs)
    print_topology_analysis(&monitors);

    Ok(())
}

#[allow(dead_code)]
/// JSON dump of all monitor data (topology + EDID + DDC) to stdout.
pub fn scan_json() -> anyhow::Result<()> {
    let monitors = collect_monitor_data().context("Hardware scan failed")?;
    let json = serde_json::to_string_pretty(&monitors)
        .context("Failed to serialize monitor data to JSON")?;
    println!("{json}");
    Ok(())
}

/// Print topology analysis: clone detection and extended mode verification.
fn print_topology_analysis(monitors: &[MonitorDetails]) {
    let active: Vec<&MonitorDetails> = monitors.iter().filter(|m| m.is_active).collect();
    if active.len() < 2 {
        return; // No multi-monitor analysis needed
    }

    println!("─── Multi-Monitor Analysis ────────────────────────");

    for m in &active {
        let topo = match m.topology.as_ref() {
            Some(t) => t,
            None => continue,
        };

        let overlapping: Vec<String> = active
            .iter()
            .filter(|o| o.target_id != m.target_id)
            .filter_map(|o| o.topology.as_ref().map(|ot| (o, ot)))
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
            .map(|(o, _)| {
                format!(
                    "'{}' (id={})",
                    clean_display_string(&o.friendly_name),
                    o.target_id
                )
            })
            .collect();

        if overlapping.is_empty() {
            println!(
                "  ✅ {} — EXTENDED (unique geometry)",
                clean_display_string(&m.friendly_name)
            );
        } else {
            println!(
                "  🔄 {} — CLONED with: {}",
                clean_display_string(&m.friendly_name),
                overlapping.join(", ")
            );
        }
    }
    println!();
}

#[allow(dead_code)]
/// Rich detail output for a single monitor (called from `display --output X --info`).
pub fn monitor_info(query: &str) -> anyhow::Result<()> {
    let m = resolve_monitor(query)?;
    let glyph = if m.is_active { "🖥" } else { "⏹" };

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              Monitor Detail Report                      ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!(
        "  Name:          {}  {}",
        clean_display_string(&m.friendly_name),
        glyph
    );
    println!("  Target ID:     {}", m.target_id);
    println!("  Active:        {}", m.is_active);
    println!("  GDI Name:      {}", m.gdi_name);
    println!("  Interface:     {}", format_output_tech(&m.output_tech));
    println!("  Device Path:   {}", m.device_path);

    if let Some(ref t) = m.topology {
        println!("\n─── Topology ──────────────────────────────────────");
        println!("  Position:      ({}, {})", t.x, t.y);
        println!("  Resolution:    {}x{}", t.width, t.height);
        println!("  Rotation:      {}", t.rotation);
    }

    if let Some(ref edid) = m.edid {
        print_edid_detail(edid);
    } else {
        println!("\n  EDID:          not available from this backend");
    }

    if let Some(ref ddc) = m.ddc_stats {
        print_ddc_detail(ddc);
    } else {
        println!("\n─── DDC Telemetry ─────────────────────────────────");
        println!("  (not available — monitor may not support DDC/CI)");
    }

    Ok(())
}

#[allow(dead_code)]
/// Single-monitor info as JSON on stdout (called from `display --output X --info-json`).
pub fn monitor_info_json(query: &str) -> anyhow::Result<()> {
    let m = resolve_monitor(query)?;
    let json =
        serde_json::to_string_pretty(&m).context("Failed to serialize monitor data to JSON")?;
    println!("{json}");
    Ok(())
}

/// Write a plain-text EDID diagnostic report to a file.
pub fn write_edid_report(path: &str) -> anyhow::Result<()> {
    let monitors = collect_monitor_data().context("Failed to collect monitor metadata")?;

    let mut report = String::new();
    report.push_str("╔══════════════════════════════════════════════════════════╗\n");
    report.push_str("║              EDID Diagnostic Report                    ║\n");
    report.push_str("╚══════════════════════════════════════════════════════════╝\n\n");

    for (idx, m) in monitors.iter().enumerate() {
        let name = clean_display_string(&m.friendly_name);
        let tech = format_output_tech(&m.output_tech);

        report.push_str(&format!("─── Monitor {}: {}\n", idx + 1, name));
        report.push_str(&format!("  Target ID:     {}\n", m.target_id));
        report.push_str(&format!("  Active:        {}\n", m.is_active));
        report.push_str(&format!("  GDI Name:      {}\n", m.gdi_name));
        report.push_str(&format!("  Interface:     {}\n", tech));
        report.push_str(&format!("  Device Path:   {}\n", m.device_path));

        if let Some(ref t) = m.topology {
            report.push_str(&format!(
                "  Topology:      {}x{} @ ({},{}) rotation={}\n",
                t.width, t.height, t.x, t.y, t.rotation
            ));
        }

        if let Some(ref edid) = m.edid {
            report.push_str(&format!("  EDID Model:    {}\n", edid.model_name));
            report.push_str(&format!("  Manufacturer:  {}\n", edid.manufacturer_id));
            report.push_str(&format!("  Product Code:  {}\n", edid.product_code));
            report.push_str(&format!("  Serial:        {}\n", edid.serial_number_binary));
            report.push_str(&format!(
                "  Manufactured:  week {} / year {}\n",
                edid.week_of_manufacture, edid.year_of_manufacture
            ));
            report.push_str(&format!("  Video Iface:   {:?}\n", edid.video_interface));
            report.push_str(&format!("  Modes:         {}\n", edid.modes.len()));
            for mode in &edid.modes {
                report.push_str(&format!(
                    "    {}x{} @ {} Hz{}\n",
                    mode.width,
                    mode.height,
                    mode.refresh_rate,
                    if mode.interlaced { "i" } else { "p" }
                ));
            }
        } else {
            report.push_str("  EDID:          not available\n");
        }

        if let Some(ref ddc) = m.ddc_stats {
            report.push_str(&format!(
                "  DDC Brightness: {}/{}\n",
                ddc.core_caps.brightness, ddc.core_caps.brightness_max
            ));
            report.push_str(&format!(
                "  DDC Contrast:  {}/{}\n",
                ddc.core_caps.contrast, ddc.core_caps.contrast_max
            ));
            report.push_str(&format!("  DDC Input:     {:?}\n", ddc.input_source));
            report.push_str(&format!("  DDC Power:     {:?}\n", ddc.power_state));
        }
        report.push('\n');
    }

    File::create(Path::new(path))
        .with_context(|| format!("Failed to create {path}"))?
        .write_all(report.as_bytes())
        .with_context(|| format!("Failed to write {path}"))?;

    println!("Diagnostic report saved to '{path}'.");
    Ok(())
}

/// Write a full JSON dump of all monitors (topology + EDID + DDC) to a file.
pub fn write_edid_json(path: &str) -> anyhow::Result<()> {
    let monitors = collect_monitor_data().context("Failed to collect monitor metadata")?;
    let json = serde_json::to_string_pretty(&monitors)
        .context("Failed to serialize monitor data to JSON")?;

    File::create(Path::new(path))
        .with_context(|| format!("Failed to create {path}"))?
        .write_all(json.as_bytes())
        .with_context(|| format!("Failed to write {path}"))?;

    println!("Structured monitor data saved to '{path}' (JSON).");
    Ok(())
}

/// Resolve a monitor by name, target ID, device path, or GDI name.
pub fn resolve_monitor(query: &str) -> anyhow::Result<MonitorDetails> {
    let monitors = collect_monitor_data().context("Hardware scan failed")?;
    let query_lower = query.to_lowercase();

    monitors
        .iter()
        .find(|m| {
            m.friendly_name.to_lowercase().contains(&query_lower)
                || m.target_id.to_string() == query
                || m.device_path.to_lowercase().contains(&query_lower)
                || m.gdi_name.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .ok_or_else(|| {
            let available: Vec<String> = monitors
                .iter()
                .map(|m| {
                    format!(
                        "'{}' (id={})",
                        clean_display_string(&m.friendly_name),
                        m.target_id
                    )
                })
                .collect();
            anyhow!(
                "Monitor '{query}' not found. Available displays: {:?}",
                available
            )
        })
}

// ════════════════════════════════════════════════════════════════════
// Internal helpers
// ════════════════════════════════════════════════════════════════════

/// Acquire NativeTopology and build a map: target_id → OutputState
/// This enriches the output with primary, connector, adapter, HDR, scale, native res, modes.
fn get_output_state_map() -> HashMap<u32, df_displmgr::types::OutputState> {
    let mut map = HashMap::new();
    if let Ok(topology) = NativeTopology::acquire() {
        for output in topology.get_outputs() {
            if let Ok(tid) = output.identity.id.0.parse::<u32>() {
                map.insert(tid, output);
            }
        }
    }
    map
}

#[allow(dead_code)]
fn print_edid_detail(edid: &EdidData) {
    println!("\n─── EDID Details ───────────────────────────────────");
    println!("  Model:         {}", edid.model_name);
    println!(
        "  Manufacturer:  {}  Product: {}  Serial: {}",
        edid.manufacturer_id, edid.product_code, edid.serial_number_binary
    );
    if let Some(ref sn_ascii) = edid.serial_number_ascii {
        println!("  Serial (ASCII): {}", sn_ascii);
    }
    println!(
        "  Manufactured:  week {} / year {}",
        edid.week_of_manufacture, edid.year_of_manufacture
    );
    println!("  Video Iface:   {:?}", edid.video_interface);
    println!("  Extensions:    {} blocks", edid.extension_blocks);

    if let Some(ref chroma) = edid.chromaticity {
        println!("  Chromaticity:");
        println!("    Red:   ({:.3}, {:.3})", chroma.red_x, chroma.red_y);
        println!("    Green: ({:.3}, {:.3})", chroma.green_x, chroma.green_y);
        println!("    Blue:  ({:.3}, {:.3})", chroma.blue_x, chroma.blue_y);
        println!("    White: ({:.3}, {:.3})", chroma.white_x, chroma.white_y);
    }

    // HDR
    let hdr = &edid.hdr_caps;
    if hdr.supports_sdr_eotf
        || hdr.supports_hdr_traditional
        || hdr.supports_smpte_st2084
        || hdr.supports_hlg
    {
        println!("  HDR Capabilities:");
        println!(
            "    SDR EOTF:      {}",
            if hdr.supports_sdr_eotf { "✓" } else { "–" }
        );
        println!(
            "    HDR Trad:      {}",
            if hdr.supports_hdr_traditional {
                "✓"
            } else {
                "–"
            }
        );
        println!(
            "    HDR10 ST.2084: {}",
            if hdr.supports_smpte_st2084 {
                "✓"
            } else {
                "–"
            }
        );
        println!(
            "    HLG:           {}",
            if hdr.supports_hlg { "✓" } else { "–" }
        );
        if let Some(lum) = hdr.max_luminance_cd_m2 {
            println!("    Max Luminance: {:.0} cd/m²", lum);
        }
        if let Some(lum) = hdr.max_frame_average_luminance_cd_m2 {
            println!("    Max FALL:      {:.0} cd/m²", lum);
        }
        if let Some(lum) = hdr.min_luminance_cd_m2 {
            println!("    Min Luminance: {:.3} cd/m²", lum);
        }
    }

    // Audio
    if !edid.audio_caps.short_audio_descriptors.is_empty() {
        println!("  Audio Capabilities:");
        for codec in &edid.audio_caps.short_audio_descriptors {
            println!("    {}", codec);
        }
    }

    // Modes
    if !edid.modes.is_empty() {
        println!("  Supported Modes ({} total):", edid.modes.len());
        for mode in &edid.modes {
            println!(
                "    {}x{} @ {} Hz{}",
                mode.width,
                mode.height,
                mode.refresh_rate,
                if mode.interlaced { "i" } else { "p" }
            );
        }
    }
}

#[allow(dead_code)]
fn print_ddc_detail(ddc: &DeepDdcStats) {
    println!("\n─── DDC Telemetry (live) ───────────────────────────");
    println!(
        "  Brightness:    {}/{}",
        ddc.core_caps.brightness, ddc.core_caps.brightness_max
    );
    println!(
        "  Contrast:      {}/{}",
        ddc.core_caps.contrast, ddc.core_caps.contrast_max
    );
    println!("  Input Source:  {:?}", ddc.input_source);
    println!("  Power State:   {:?}", ddc.power_state);
    if let Some((vol, max_vol)) = ddc.volume {
        println!("  Volume:        {}/{}", vol, max_vol);
    }
    println!("  Audio Mute:    {:?}", ddc.audio_mute);
    if let Some((r, g, b)) = ddc.color_gains {
        println!("  Color Gains:   R={}  G={}  B={}", r, g, b);
    }
    if let Some(hz) = ddc.horizontal_freq_hz {
        println!("  H-Frequency:   {} Hz", hz);
    }
    if let Some(chz) = ddc.vertical_freq_centihz {
        println!("  V-Frequency:   {:.2} Hz", chz as f64 / 100.0);
    }
    if let Some(hours) = ddc.operating_hours {
        println!("  Operating Hrs: {} hours", hours);
    }
    if let Some(lang) = ddc.osd_language_code {
        println!("  OSD Language:  code {}", lang);
    }
    if let Some(panel) = ddc.panel_type_code {
        println!("  Panel Type:    code {}", panel);
    }
}

fn format_output_tech(tech_str: &str) -> String {
    // Parse the raw CCD enum format: "DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY(5)"
    // or "5" or "DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY_HDMI"
    let s = tech_str.trim();

    // Extract numeric code from parenthesized form
    let code = if let Some(start) = s.find('(') {
        if let Some(end) = s.find(')') {
            s[start + 1..end].trim().parse::<u32>().ok()
        } else {
            None
        }
    } else {
        None
    };

    let code = code.unwrap_or(0);

    // Map CCD DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY constants to human-readable strings.
    // See: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_video_signal_info
    // Note: 0x80000001 = DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY_INTERNAL (eDP/laptop panel)
    match code {
        0x01 => "DVI-D".to_string(),
        0x02 => "Other DVI".to_string(),
        0x03 => "DVI".to_string(),
        0x04 => "HDMI".to_string(),
        0x05 => "DisplayPort".to_string(),
        0x06 => "DisplayPort (External)".to_string(),
        0x07 => "S-Video".to_string(),
        0x08 => "Composite".to_string(),
        0x09 => "Component".to_string(),
        0x0A => "VGA".to_string(),
        0x0B => "USB-C".to_string(),
        0x0C => "SDI".to_string(),
        0x0D => "Micro HDMI".to_string(),
        0x80_00_00_01 => "eDP (Internal)".to_string(),
        _ => {
            // Fallback: try to extract from the string name
            let cleaned = s
                .replace("DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY_", "")
                .replace("DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY", "")
                .trim_matches(|c| c == '(' || c == ')' || c == ' ')
                .to_string();
            if cleaned.is_empty() || cleaned == code.to_string() {
                format!("Unknown (code {})", code)
            } else {
                cleaned
            }
        }
    }
}

fn clean_display_string(input: &str) -> String {
    input
        .replace('\0', "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Rectangle overlap test (same algorithm as test_extend.rs and extended_tests.rs).
/// Returns true if the two rectangles have a non-touching intersection area.
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
