# Displaymanager Projekt History (Archiv)

## Archivierte Einträge (älter als die letzten 4)

### Windows FFI Null-Handle Edge Cases (18. Juli 2026)

- `error_path_tests.rs` — `DdcCommunication` → `CommunicationFailed { reason }`
- `error_path_tests.rs` — `Platform` → (entfernt; AccessDenied ist Unit-Variante)
- Conversion-Tests korrigiert: `DdcError` ist Type-Alias von `DisplayManagerError`,
- `benches/ddc_operations.rs` — `parallel_benches`-Group war nur für Linux definiert,
- `displmgr_ccd_editor.rs:192, :202`
- `displmgr_ccd.rs:77, :79`
- `displmgr_gdi_editor.rs:117`
- `displmgr_gdi.rs:126`
- `EnumDisplaySettingsW`, `ChangeDisplaySettingsExW` (apply_resolution, flush)
- `GetDisplayConfigBufferSizes`, `QueryDisplayConfig` (query_ccd_paths)
- `query_ccd_paths`-Aufruf, Union-Feldzugriffe, `SetDisplayConfig` (apply_resolution_for_target)
- `lib.rs` — `DisplayManagerError` zum Root-Re-Export hinzugefügt
- `lib.rs:1087` test_display_error_not_found — `DisplayId(...)` → `"DP-1".to_string()`
- `tests/error_test.rs` — `NotFound` → `MonitorNotFound`, `DisplayId` → `.to_string()`,
- `error_path_tests.rs` — `Platform` → `AccessDenied` (Unit-Variante, match auf `&dm_err`)
- `error_path_tests.rs` — `DdcCommunication` → `CommunicationFailed { reason }` (2x)
- `tests/integration_tests.rs` + `tests/coverage_tests.rs` + `tests/common/` GELÖSCHT.
- `df_displmgr/src/error.rs` — unused Imports entfernt (`thiserror::Error`, `crate::types::DisplayId`),
- `cargo check --workspace --all-targets` → **0 Fehler**
- **Error-Type-Konsolidierung**: `df_core::DisplayManagerError` ist der vereinheitlichte Typ; `df_ddc` und `df_displmgr` re-exportieren ihn; `df_displmgr_info` hat `EdidError` mit `From<EdidError> for DisplayManagerError`
- **DdcErrorWithContext / DisplayErrorWithSource zusammenlegen**: Beide sind bereits als deprecated Type Aliases auf `DisplayManagerError` gesetzt
- **`#![deny(unsafe_op_in_unsafe_fn)]`**: Bereits in `df_core/lib.rs` (Zeile 6) und `df_displmgr/lib.rs` (Zeile 36) vorhanden
- **Async/Sync-Mischung aufräumen**: `hardware_settle_delay()` verwendet bereits `tokio::time::sleep` direkt (kein `spawn_blocking` mehr)
- **`indicatif` als Feature-Flag**: Bereits in `df_displmgr` Cargo.toml enthalten (`progress = ["dep:indicatif"]`)
- **Code-Deduplizierung: MockBackend**: `test_utils.rs` erstellt mit `MockBackend` und `MockBackendWithCalls`; 3 Test-Dateien (`windows_ffi_tests.rs`, `lib.rs`, `ddc_trait_encapsulation_tests.rs`) auf die zentrale Utility migriert
- **Code-Deduplizierung: get_capabilities_string()**: Duplizierte Overrides in `ddc_linux.rs` und `ddc_win.rs` entfernt (Trait Default ist identisch)
- **Unreachable Pattern Warning**: In `ddc_mgr.rs` gefixt (doppelte Pattern-Match entfernt)
- **Dokumentation Deutsch→Englisch**: `constants.rs` (komplette Datei) und `display_benchmark.rs` (alle Szenario-Kommentare) auf Englisch übersetzt
- **Pre-existing Build Error**: Syntax-Fehler in `displmgr_gdi_editor.rs` gefixt (`get_output_mut()` Helper extrahiert, `?`-Operator-Kontext korrigiert)
- **Error-Type-Konsolidierung**: `df_core::DisplayManagerError` ist der vereinheitlichte Typ; `df_ddc` und `df_displmgr` re-exportieren ihn; `df_displmgr_info` hat `EdidError` mit `From<EdidError> for DisplayManagerError`
- **DdcErrorWithContext / DisplayErrorWithSource zusammenlegen**: Beide sind bereits als deprecated Type Aliases auf `DisplayManagerError` gesetzt
- **`#![deny(unsafe_op_in_unsafe_fn)]`**: Bereits in `df_core/lib.rs` (Zeile 6) und `df_displmgr/lib.rs` (Zeile 36) vorhanden
- **Async/Sync-Mischung aufräumen**: `hardware_settle_delay()` verwendet bereits `tokio::time::sleep` direkt (kein `spawn_blocking` mehr)
- Parallel I2C probing performance tests (criterion benchmark)
- Windows FFI null-handle edge cases
- Linux DDC ioctl failure retry exhaustions
- Wayland Protocol Errors abfangen (displmgr_wlr.rs)
- Verbesserte Fehlermeldungen mit Context
- Fortschrittsanzeige bei Hardware-Abfragen (indicatif)
- Compiler-Warnungen bereinigen (timing.rs, main.rs)
- CLI Dispatch & Integration Tests repariert (main.rs verwendet cli.rs)
- Multi-GPU DRM support (Integration abgeschlossen)
- GTF/CVT Timing Unterstützung (calculate_cvt/cvt_rb/gtf + TimingParameters + Tests)
- Rewrote `apps/displaymanager_cli/src/main.rs` to use `cli.rs` module for argument parsing
- Implemented subcommand dispatch for `list`, `info`, `set`, `ddc`, `activate`
- `list --scan` now creates `edid_dump.txt` via `info::write_edid_report()`
- `list --scan --output <file>` now creates JSON output via `info::write_edid_json()`
- `set` command maps CLI args to `SetArgs` struct and uses tokio runtime for async `apply()`
- `ddc` command dispatches to `ddc::apply_ddc()` with proper dry-run support
- `activate` command resolves monitor and applies extended mode via `set::apply()`
- All 39 CLI integration tests now pass (previously 16 failing)
- Full workspace test suite passes with zero warnings

### Compiler Warning Cleanup (20. Juli 2026)
- Removed unnecessary `mut` qualifiers in `crates/df_core/src/timing.rs` (6 of 7 variables)
- Kept `mut` on `hsync_start` which requires it for `+=` operation in CVT standard branch
- Removed unused imports (`DisplayId`, `Extent2D`, `Point2D`) and `mut` in `apps/displaymanager_cli/src/main.rs`
- Created `.clinerules/todo.md` from project todo list
- Updated `.clinerules/history.md` and archived older entries to `.ltmclinerules/ltmhistory.md`

### GTF/CVT Timing Calculation Module (20. Juli 2026)
- Added `crates/df_core/src/timing.rs` with kernel-derived CVT/GTF formulas
- Implemented `calculate_cvt`, `cvt_rb`, and `gtf` helpers
- Introduced `TimingParameters` struct for modeline data
- Added unit tests for preset timing calculations
- Re-exported timing utilities from `df_displmgr`
- Verified workspace tests continue to pass

### Wayland HDR Error Path Improvements (20. Juli 2026)
- Verified `wayland-protocols-wlr` 0.3.12 lacks `zwlr_color_manager_v1`
- Improved HDR error message in `WlrTopology::commit()` for Wayland backend
- Returns clear `DisplayError::UnsupportedFeature` with actionable context when HDR is requested
- All workspace tests passing (533 tests verified)
- [x] Added explicit handling for null physical monitor handles in Windows enumeration
- [x] Added graceful handling for disconnected monitors during EnumDisplayMonitors callback
- [x] Improved error messages with context for null handle scenarios
- [x] Added Windows FFI tests module

### Linux DDC ioctl Failure Retry Exhaustions (18. Juli 2026)
- [x] Added detailed error context for each retry attempt in get_vcp_feature
- [x] Added checksum validation for DDC/CI responses
- [x] Improved retry backoff with exponential delay based on constants
- [x] Added tests for LinuxBackend retry logic and checksum validation

### Multi-GPU DRM Support - Full Integration (18. Juli 2026)
- [x] Added `detect_drm_devices()` function to scan for all available DRM cards
- [x] Added `probe_all_drm_devices()` for multi-GPU output enumeration
- [x] Added `probe_drm_hardware()` with adapter_id parameter for device-specific probing
- [x] Updated `DrmTopology` struct to use `HashMap<AdapterId, File>` for multi-GPU device tracking
- [x] Modified `resource_map` to include `(AdapterId, DrmResourceIds)` tuples for multi-GPU routing
- [x] Updated `acquire()` to use `probe_all_drm_devices()` for full multi-GPU enumeration
- [x] Updated `validate()` to iterate across all detected GPU devices
- [x] Updated all `set_*` methods in `DrmOutputEditor` to correctly unpack the resource tuple
- [x] Project compiles successfully with multi-GPU support

### Multi-GPU DRM Support - Foundation (18. Juli 2026)
- [x] Added `detect_drm_devices()` function to scan for all available DRM cards
- [x] Added `probe_all_drm_devices()` for multi-GPU output enumeration
- [x] Added `probe_drm_hardware()` with adapter_id parameter for device-specific probing
