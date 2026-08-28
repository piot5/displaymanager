# df_displmgr_info

[![Crates.io](https://img.shields.io/crates/v/df_displmgr_info.svg)](https://crates.io/crates/df_displmgr_info)
[![License](https://img.shields.io/crates/l/df_displmgr_info.svg)](https://github.com/piot5/displaymanager/blob/main/LICENSE-MIT)
[![CI](https://github.com/piot5/displaymanager/actions/workflows/ci.yml/badge.svg)](https://github.com/piot5/displaymanager/actions/workflows/ci.yml)

Display management and hardware telemetry. Reads raw EDID blocks, parses them,
combines them with DDC statistics (`DeepDdcStats`) and topology
(`MonitorTopology`), and exposes a single `MonitorDetails` per display.

## Modules

| Module | Purpose |
|---|---|
| `edid_parser.rs` | EDID parser (base block + extensions) with checksum validation |
| `edid_trait.rs` | `DisplayDevice` trait with `fetch_edid()` |
| `edid_backends` | OS-specific sources (Windows: `SetupAPI`/CCD; Linux: `sysfs` + `ddcutil`) |
| `backends.rs` | `get_platform_enumerator() -> Box<dyn MonitorEnumerator>` and `MonitorDetails` |
| `edid_types.rs` | `EdidData`, `MonitorCapabilities`, `MonitorTopology`, `MonitorMode`, `ChromaticityCoordinates`, `HdrMetadata`, `AudioCapabilities`, `DeepDdcStats` |
| `error.rs` | `EdidError` (parse, IO, backend, ...) |

## Example

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitors = df_displmgr_info::collect_monitor_data()?;
    for m in &monitors {
        println!(
            "{} — {} {}\n  panel: {} {}  max: {}x{} @ {} Hz",
            m.target_id, m.manufacturer, m.model,
            m.panel_size_in, m.aspect_ratio,
            m.max_resolution.0, m.max_resolution.1,
            m.max_refresh_hz
        );

        if let Some(edid) = &m.edid {
            println!("  HDR: {:?}", edid.hdr);
            println!("  Audio: {:?}", edid.audio);
        }
    }
    Ok(())
}
```

## EDID parser coverage

- Base block (128 bytes): vendor ID, product code, serial number, display
  range limits, established timings.
- Chromaticity coordinates (10-bit BCD → float).
- Extension blocks (CEA, VTB, DisplayID) are read for the parts covered by
  the standard part. Unknown extensions are skipped.
- Checksum validation (`validate_checksum`).

## Backends

- **Windows**: enumerates devices via `SetupAPI` and Windows CCD; reads raw
  EDID through the registered `PHYSICAL_MONITOR` handle.
- **Linux**: reads `/sys/class/drm/card*-*/edid`, supplements with DDC
  statistics from `df_ddc` (I2C) when reachable.

## Errors

- Single `EdidError` enum.
- Corrupted EDID blocks return `EdidError::Parse { offset, reason }` with
  the byte offset.

## Examples

```bash
cargo run -p df_displmgr_info --example dump_edid
```

## Tests and benchmarks

```bash
cargo test -p df_displmgr_info
cargo bench -p df_displmgr_info
```

## License

MIT OR Apache-2.0
