# displaymanager_cli (`displaymanager_cli`)

Command-line frontend for DisplayManager. Exposes topology, hardware and
DDC/CI control through a single binary. The binary is registered as
`displaymanager_cli` in `Cargo.toml`.

## Build

```bash
cargo build -p displaymanager_cli --release
# → target/release/displaymanager_cli   (or .exe on Windows)
```

## Subcommands

```
displaymanager_cli <COMMAND>

Commands:
  display  Display topology: scan, info, set
  ddc      DDC/CI brightness, contrast, input, power
  help     Print this message or the help of the given subcommand(s)
```

| Subcommand | Crate | Purpose |
|---|---|---|
| `display` | `df_displmgr` + `df_displmgr_info` | Topology: scan, info, set |
| `ddc` | `df_ddc` | Brightness, contrast, volume, power, input |

## `display` — topology and configuration

### `display scan` — list all monitors

Scans all monitors and outputs topology, EDID, DDC telemetry and
multi-monitor analysis. Always writes `edid_dump.txt` automatically.

```bash
displaymanager_cli display scan              # all monitors, human-readable
displaymanager_cli display scan --json       # same, as JSON on stdout
displaymanager_cli display scan --edid-json ./edid.json  # write JSON to file
```

| Flag | Short | Meaning |
|---|---|---|
| `--json` | `-j` | Output as JSON on stdout |
| `--edid-json <path>` | | Write full monitor data as JSON to file |

### `display info` — single-monitor detail

```bash
displaymanager_cli display info -o "BenQ GL2450H"           # formatted report
displaymanager_cli display info -o "BenQ GL2450H" --json    # JSON on stdout
```

| Flag | Short | Meaning |
|---|---|---|
| `-o`, `--output` | `-o` | Monitor name, target ID, GDI path, or device path (**required**) |
| `--json` | `-j` | Output as JSON on stdout |

### `display set` — apply configuration

Applies topology changes: position, resolution, rotation, HDR, scale,
primary flag. Inactive monitors are automatically activated via CCD wake
on Windows.

```bash
# Extended mode: 2560x1440 right of primary, rotated 90°
displaymanager_cli display set -o "Dell U2719D" \
    --mode-type extended \
    --mode 2560x1440 \
    --pos 1920,0 \
    --rotate 90

# Cloned: secondary mirrors primary
displaymanager_cli display set -o "HDMI-2" \
    --mode-type cloned --clone-from "\\.\DISPLAY1"

# Disable a display
displaymanager_cli display set -o "HDMI-2" --mode-type off

# Set HDR and scale
displaymanager_cli display set -o "BenQ GL2450H" --hdr on --scale 1.50 --primary

# Set refresh rate to 144 Hz
displaymanager_cli display set -o "BenQ GL2450H" --mode 1920x1080 --refresh-rate 144000

# Dry-run: validate without changing anything
displaymanager_cli display set -o "HDMI-2" --mode 1920x1080 --pos 0,0 --verify-only

# Auto-position: place right of the rightmost active monitor
displaymanager_cli display set -o "HDMI-2" --mode 1920x1080 --auto-pos
```

#### Flags for `display set`

| Flag | Meaning |
|---|---|
| `-o`, `--output` | Monitor name, target ID, GDI path or device path (required) |
| `--mode-type` | `extended` \| `cloned` \| `off` |
| `--clone-from` | Source for `cloned` (e.g. `\\.\DISPLAY1`) |
| `--mode` | Resolution, e.g. `1920x1080` |
| `--pos` | Position, e.g. `0x0` or `100,200` |
| `--rotate` | `0`, `90`, `180`, `270` |
| `--auto-pos` | Place right of the rightmost active monitor |
| `--ccd-wake` | (Windows) `SetDisplayConfig` wake before the edit |
| `--refresh-rate` | Refresh rate in mHz, e.g. `60000` for 60 Hz |
| `--hdr` | `on` / `off` — toggle HDR |
| `--scale` | Desktop scale factor, e.g. `1.0`, `1.25`, `1.5` |
| `--primary` | Mark this display as primary |
| `--verify-only` | Read-only validation (dry-run) |

## `ddc` — direct monitor control

```bash
# Capabilities of monitor at index 0
displaymanager_cli ddc list
displaymanager_cli ddc list --json           # JSON output

# Brightness / contrast / volume (0..100)
displaymanager_cli ddc brightness 50
displaymanager_cli ddc contrast   75
displaymanager_cli ddc volume     30

# Power on/off
displaymanager_cli ddc power on
displaymanager_cli ddc power off

# Switch input
displaymanager_cli ddc input hdmi1
displaymanager_cli ddc input dp1
displaymanager_cli ddc input 0x0F     # numeric (DDC input code)

# Color gains (R, G, B 0..100)
displaymanager_cli ddc color-gains 100 90 80
```

| Subcommand | Argument | Meaning |
|---|---|---|
| `list` | – | Print current VCP values |
| `brightness` | `0..100` | Set brightness |
| `contrast` | `0..100` | Set contrast |
| `volume` | `0..100` | Set volume |
| `power` | `on` / `off` | Set power state |
| `input` | `dp1`, `dp2`, `hdmi1`, `hdmi2`, `0xXX` | Set active input |
| `color-gains` | `<R> <G> <B>` | Set RGB color gains |

Global DDC options:

| Flag | Meaning |
|---|---|
| `--id <N>` | Monitor index from `list` (default: 0) |
| `--json` | Output `list` result as JSON |

## Extra binaries

The `src/bin/` directory contains two additional diagnostic binaries:

```bash
cargo run -p displaymanager_cli --bin debug_test
cargo run -p displaymanager_cli --bin test_topology
```

## Errors

- Topology lookups are reported through `anyhow!` with context
  (e.g. `Failed to acquire editor for '...'`).
- DDC operations return `DdcError` variants (`I2c`, `Vcp`, `Handle`, ...).
- The CLI calls `validate().await` before applying; on inconsistency it
  aborts without touching the system.

## Requirements

- Windows 10/11 with Windows CCD.
- Linux: Wayland with a wlroots-based compositor (for `display`), or
  raw I2C-DDC (for `ddc`).

## License

MIT (see crate `Cargo.toml`)