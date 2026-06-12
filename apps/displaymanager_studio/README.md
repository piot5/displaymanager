# displaymanager_studio

DisplayFlow Studio. egui/eframe GUI on top of `df_displmgr`,
`df_displmgr_info` and `df_ddc`. Adds a window layout manager, a live
wallpaper engine and a wgpu-based animation editor.

The studio is Windows-first because of the `windows = "0.52"` dependency,
but the lower-level crates are still platform-portable.

## Build

```bash
cargo build -p displaymanager_studio --release
# → target/release/displaymanager_studio(.exe)
```

## Tabs

| Tab | Module | Purpose |
|---|---|---|
| Hardware | `monitor_manager.rs` | Live scan, EDID/DDC view, set modes and resolution |
| Windows | `window_manager.rs` | Attach external windows to slots, auto-hook new processes |
| Wallpaper | `live_wallpaper.rs` | Live wallpaper engine, load `.flow` packages |
| Anim Editor | `anim_editor.rs` | wgpu shader editor with per-monitor preview |
| Log | (in `main.rs`) | Scrollable system log |

## Run modes

1. **GUI mode (default)** — opens the egui window "DisplayFlow Studio".
2. **Wallpaper mode (worker)** — spawns a hidden worker process that paints
   a `.flow` package on every connected monitor:

   ```bash
   displaymanager_studio.exe wallpaper ./mypkg.flow
   ```

   The worker mode creates one hidden `HWND` per monitor
   (`init_windows` in `anim_editor.rs`) and renders the configured fragment
   shader through `wgpu::GpuCore`.

## Modules

- `loader.rs` — `FlowPackage`, `Config`, `Step` definitions for `.flow`
  packages, central `AppState`.
- `scan.rs` — `collect_monitor_data()` (aggregates `df_displmgr_info` and
  `df_ddc` into `MonitorDetails` + `DdcData`).
- `set.rs` — high-level apply: takes `DemoArgs` / `OutputArgs`, runs DDC
  and layout changes together.
- `monitor_manager.rs` — Hardware tab UI.
- `window_manager.rs` — `WindowManager`:
  - `add_slot(id, rect, z_order)`
  - `attach(id, target)` / `detach(id)`
  - `setup_hooks()` — `WinEventHook` for auto-detect of new processes
  - `listen_ipc()` — async IPC loop
  - `run_loop()` — layout sync thread
  - `AutoHook`, `AutoWindow` — RAII wrappers for `UnhookWinEvent` and
    `DestroyWindow`
- `live_wallpaper.rs` — wallpaper engine render path.
- `anim_editor.rs` — `GpuCore` (wgpu pipeline), uniforms, worker `HWND`s
  per monitor, `render_anim_editor(ui, state)`.

## `.flow` packages

A `.flow` package bundles:

- a fragment shader source (`shader_src`),
- a default shader name (`fs_default`),
- a `Config` with `Step`s (position / colour / sound stages).

```rust
pub struct FlowPackage {
    pub shader_src: String,
    pub target_shader: String,
    pub config: Config,
    pub steps: Vec<Step>,
}
```

`AppState` holds the loaded flows, active animations, a `rodio::Sink` for
sound effects, and a live reference to the `WindowManager`.

## IPC

`listen_ipc` accepts external commands (e.g. `attach <pid> <slot_id>`)
so other tools or a second studio process can adjust slots at runtime.

## Requirements

- Windows 10/11 with Windows CCD.
- GPU driver supporting Vulkan / DX12 / Metal (wgpu).
- MSVC build tools and Windows 10 SDK.

## Errors

- Modules return `Result<_, anyhow::Error>`; non-fatal messages are
  collected in the in-app Log tab.
- wgpu and `HWND` operations are confined to the smallest possible
  `unsafe` blocks.

## Known limits

- The `windows` crate features in `Cargo.toml` cover only what the studio
  needs (LibraryLoader, Accessibility, GDI, HiDpi, WindowsAndMessaging,
  Foundation). Other subsystems are not linked.
- The `wallpaper` worker mode is Windows-only.

## License

MIT (see crate `Cargo.toml`)
