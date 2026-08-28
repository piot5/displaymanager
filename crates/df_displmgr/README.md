# df_displmgr

[![Crates.io](https://img.shields.io/crates/v/df_displmgr.svg)](https://crates.io/crates/df_displmgr)
[![License](https://img.shields.io/crates/l/df_displmgr.svg)](https://github.com/piot5/displaymanager/blob/main/LICENSE-MIT)
[![CI](https://github.com/piot5/displaymanager/actions/workflows/ci.yml/badge.svg)](https://github.com/piot5/displaymanager/actions/workflows/ci.yml)

Cross-platform display configuration manager. Wraps the OS-specific
configuration APIs (Windows CCD on Windows, Wayland/wlroots + DRM on Linux)
behind a single async trait model.

## Concepts

- **`NativeTopology`** — handle to the current display topology. Resolved to
  the platform backend at compile time.
- **`trait UniversalTopology`** — lifecycle:
  1. `acquire()` — load topology (sync, FFI-bound)
  2. `get_outputs()` — read current `Vec<OutputState>`
  3. `edit_output(&DisplayId) -> Box<dyn OutputEditable>` — open an editor
  4. `validate().await` — async sanity check
  5. `commit().await` — async apply to the OS
- **`trait OutputEditable`** — `set_resolution`, `set_position`,
  `set_rotation`, `set_enabled`.
- **`ActivationPlan`** — save topology → force activate → restore topology →
  place target display.

## Backends

| OS | Backend | Notes |
|---|---|---|
| Windows | `backends::windows` (CCD + `SetDisplayConfig`) | Also exports `force_activate_by_monitor_name`, `force_all`, `ccd_wake_display`, `activate_display`, `WinDisplayManager`, `query_all_display_targets`, `find_display_target` |
| Linux (Wayland) | `backends::linux` via `wayland-client` + `wayland-protocols-wlr` | Compositor must implement `wlr-output-management` unstable |
| Linux (DRM) | `drm` for hotplug and property I/O | Used alongside the Wayland path |

## Example

```rust
use df_displmgr::NativeTopology;
use df_displmgr::traits::UniversalTopology;
use df_displmgr::types::{DisplayId, DisplayRotation, Extent2D, Point2D};

#[tokio::main]
async fn main() -> df_displmgr::DisplayResult<()> {
    let mut topo = NativeTopology::acquire()?;
    for out in topo.get_outputs() {
        println!("{} @ {}x{} px", out.id.0, out.mode.width, out.mode.height);
    }

    let mut editor = topo.edit_output(&DisplayId("\\\\.\\DISPLAY1".into()))?;
    editor.set_resolution(Extent2D { width: 2560, height: 1440 })?;
    editor.set_position(Point2D { x: 1920, y: 0 })?;
    editor.set_rotation(DisplayRotation::Rotate0)?;
    drop(editor);

    topo.validate().await?;
    topo.commit().await?;
    Ok(())
}
```

## Types (`types.rs`)

| Type | Purpose |
|---|---|
| `DisplayId(String)` | Stable OS-side ID (device path or output name) |
| `ConnectorId(String)` | Physical connector identifier |
| `AdapterId(String)` | GPU/adapter identifier |
| `DisplayIdentity` | Aggregate of ID + name + vendor |
| `Point2D`, `Extent2D`, `Rect` | Geometry |
| `VideoMode` | Resolution and refresh rate |
| `OutputState` | Snapshot of an output, with `is_landscape()` and `refresh_rate_hz()` |
| `ActivationPlan` | Default plan for safe re-activation |
| `DisplayRotation` | `Rotate0`, `Rotate90`, `Rotate180`, `Rotate270` |

## Errors

- `DisplayError` enum and `DisplayResult<T>` alias.
- Conversions from `windows` HRESULTs and Wayland/DRM errors are in
  `error.rs`.

## Build notes

- `target_os = "windows"` — `windows = "0.52"` with GDI and
  `Win32_UI_WindowsAndMessaging` features.
- `target_os = "linux"` — `wayland-client`, `wayland-protocols`,
  `wayland-protocols-wlr` (with `client` feature), `drm`.

## Tests and benchmarks

```bash
cargo test -p df_displmgr
cargo bench -p df_displmgr
```

`benches/display_benchmark.rs` uses `criterion`.

## License

MIT
