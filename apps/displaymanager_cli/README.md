# displaymanager_cli (`displaymanager_cli`)

Command-line frontend for DisplayManager. Exposes display topology, hardware
information and configuration through a single flat-flag binary.

## Build

```bash
cargo build -p displaymanager_cli --release
# → target/release/displaymanager_cli   (or .exe on Windows)
```

## Usage

```
displaymanager_cli [OPTIONS]
```

All operations use a flat set of flags — no subcommands.

| Flag | Short | Meaning |
|------|-------|---------|
| `--scan` | `-s` | `file` \| hardware inventory, save to file if wanted |
| `--id` | `-i` | Monitor name, target ID, GDI path or device path (required) |
| `--off` | `-o` | Set off display |
| `--mode` | `-m` | `ext` \| `clone` |
| `--cloned` | `-c` | Source for `cloned` (e.g. `\\.\DISPLAY1`) |
| `--res` | `-r` | Resolution, e.g. `1920x1080` |
| `--topo` | `-t` | Position, e.g. `0x0` or `100,200` |
| `--freq` | `-f` | Refresh rate in mHz, e.g. `144000` for 144 Hz |
| `--rotate` | `-R` | `0`, `90`, `180`, `270` |
| `--hdr` | | `on` / `off` — toggle HDR |
| `--scale` | | Desktop scale factor, e.g. `1.0`, `1.25`, `1.5` |
| `--primary` | `-p` | Mark this display as primary |
| `--verify` | | Read-only validation (dry-run) |
| `--brightness` | | DDC brightness (0..100) |
| `--contrast` | | DDC contrast (0..100) |
| `--volume` | | DDC volume (0..100) |
| `--input` | | DDC input source (`dp1`, `dp2`, `hdmi1`, `hdmi2`, `0x0F`) |
| `--power` | | DDC power state: `on` / `off` |

## Examples

```bash
# Hardware scan (no changes)
displaymanager_cli
displaymanager_cli --scan
displaymanager_cli -s                         # same
displaymanager_cli -s ./inventory.json         # save as JSON

# Disable a display
displaymanager_cli -i "HDMI-2" --off
displaymanager_cli -i "HDMI-2" -o             # same

# Set HDR and scale
displaymanager_cli -i "BenQ GL2450H" --hdr on --scale 1.50 --primary

# Set refresh rate to 144 Hz
displaymanager_cli -i "BenQ GL2450H" -r 1920x1080 -f 144000

# Extended mode at specific position
displaymanager_cli -i "Dell U2719D" -r 2560x1440 -t 1920,0 -R 90

# Clone a display
displaymanager_cli -i "HDMI-2" --mode clone -c "\\.\DISPLAY1"

# DDC brightness / contrast / volume
displaymanager_cli -i "BenQ GL2450H" --brightness 50
displaymanager_cli -i "BenQ GL2450H" --contrast 75 --brightness 60
displaymanager_cli -i "BenQ GL2450H" --volume 30

# DDC power on/off
displaymanager_cli -i "BenQ GL2450H" --power off
displaymanager_cli -i "BenQ GL2450H" --power on

# DDC input source
displaymanager_cli -i "BenQ GL2450H" --input hdmi1
displaymanager_cli -i "BenQ GL2450H" --input 0x0F

# Dry-run: validate without changing anything
displaymanager_cli -i "HDMI-2" -r 1920x1080 -t 0,0 --verify
```

## Extra binaries

The `src/bin/` directory contains two additional diagnostic binaries:

```bash
cargo run -p displaymanager_cli --bin debug_test
cargo run -p displaymanager_cli --bin test_topology
```

## Errors

- Topology lookups are reported through `anyhow!` with context
  (e.g. `Failed to acquire editor for '...'`).
- The CLI calls `validate().await` before applying; on inconsistency it
  aborts without touching the system.

## Requirements

- Windows 10/11 with Windows CCD.
- Linux: Wayland with a wlroots-based compositor.

## License

MIT (see crate `Cargo.toml`)