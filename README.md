# DisplayManager (DisplayFlow)

Cross-platform display configuration toolkit for Windows (Windows CCD / GDI / DDC/CI)
and Linux (Wayland / wlroots / DRM / I2C-DDC). Cargo workspace with three library
crates and two applications.

## Members

| Path | Purpose |
|---|---|
| `crates/df_ddc` | DDC/CI monitor control backend (brightness, contrast, input, power) |
| `crates/df_displmgr` | Display topology abstraction (modes, position, rotation) |
| `crates/df_displmgr_info` | EDID parser and hardware telemetry |
| `apps/displaymanager_cli` | Command-line frontend (`flux-cli`) |
| `apps/displaymanager_studio` | GUI (egui/eframe) with live wallpaper and animation editor |

## Build

```bash
cargo build --release
cargo build -p displaymanager_cli --release
cargo build -p displaymanager_studio --release
```

Binaries land in `target/release/`.

## Architecture

```
displaymanager_studio  ─┐
                        ├─► df_displmgr (topology) ─► df_ddc (DDC/CI)
displaymanager_cli     ─┘                      ─► df_displmgr_info (EDID/telemetry)
```

`df_displmgr` is the central layer. `df_ddc` and `df_displmgr_info` are
independent backends that the higher-level apps call.

## Platform support

| OS | Display config | DDC/CI |
|---|---|---|
| Windows 10/11 | Windows CCD + GDI | `EnumDisplayMonitors` + `HighLevelMonitorConfigurationAPI` |
| Linux (Wayland) | wlroots output manager | I2C via `/dev/i2c-*` (ddcutil-style) |
| Linux (X11) | none | I2C via `/dev/i2c-*` |

## Requirements

- Rust 1.75+ (edition 2021)
- Windows: MSVC build tools, Windows 10 SDK
- Linux: `libwayland-dev`, `libdrm-dev`, `i2c-dev` kernel module, read/write
  access to `/dev/i2c-*`. Compositor must implement `wlr-output-management`
  (unstable).

## Workspace commands

```bash
cargo test --workspace
cargo test -p df_ddc
cargo clippy --workspace --all-targets
cargo fmt --all
```

## License

Each crate declares its own license in its `Cargo.toml` (MIT and/or
Apache-2.0). The root workspace contains no additional code.

## Author

piot5 — https://github.com/piot5
