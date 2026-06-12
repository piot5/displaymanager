# df_ddc

DDC/CI (VESA MCCS) monitor control backend. Low-level I/O layer for VCP
features: brightness, contrast, volume, input, power.

## Backends

| OS | Enumeration | I/O |
|---|---|---|
| Windows | `EnumDisplayMonitors` + `GetPhysicalMonitorsFromHMONITOR` | `HighLevelMonitorConfigurationAPI` |
| Linux | scan of `/dev/i2c-*` with VCP `0x10` probe | raw I2C, DDC packets over `i2c-dev` |

On Linux, buses that do not answer a standard VCP probe are skipped, which
filters out notebook-internal `smbus` devices.

## Usage

```rust
use df_ddc::list_monitors;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for (i, dev) in list_monitors().iter().enumerate() {
        let caps = dev.inner.get_capabilities()?;
        println!("[{i}] {}  bright={}/{}  contrast={}/{}",
            dev.info, caps.brightness, caps.brightness_max,
            caps.contrast, caps.contrast_max);

        dev.inner.set_brightness(50)?;
    }
    Ok(())
}
```

## API

- `list_monitors() -> Vec<DisplayDevice>` — enumerate DDC-capable monitors.
- `DisplayDevice::info: String` — OS-supplied description
  (e.g. `\\.\DISPLAY1\Monitor0` or `I2C Display Bus (i2c-4)`).
- `trait DdcControl` (`ddc_trait.rs`):
  - `get_capabilities() -> Result<MonitorCapabilities, DdcError>`
  - `get_vcp_feature(code: u8) -> Result<VcpValue, DdcError>`
  - `set_vcp_feature(code: u8, value: u16) -> Result<(), DdcError>`
  - `set_brightness(value: u32)`
  - `set_contrast(value: u32)`
  - `set_power(state: PowerState)`
  - `set_input(source: InputSource)`
- Types (`ddc_types.rs`): `MonitorCapabilities`, `InputSource`, `PowerState`,
  `VcpCode`, `VcpValue`.

## Bundled binary

`src/bin/ddc_mgr.rs` — small standalone CLI for smoke tests:

```bash
cargo run -p df_ddc --bin ddc_mgr -- --help
```

## Benchmarks

```bash
cargo bench -p df_ddc
```

`benches/ddc_operations.rs` uses `criterion`.

## Platform gates

- `cfg(target_os = "windows")` — pulls `windows` crate with
  `Win32_Graphics_Gdi` and `Win32_Devices_Display`.
- `cfg(target_os = "linux")` — pulls `libc` for I2C `ioctl`.

## Safety notes

- The Windows callback in `list_monitors()` is `unsafe extern "system"`. The
  vector must outlive the call to `EnumDisplayMonitors`; this is guaranteed
  by the call structure.
- On Linux, each I2C bus is briefly `open()`/`ioctl()`-probed. Measured at
  well under 50 ms for a single-thread enumeration.

## License

MIT OR Apache-2.0
