# To do Checklist (aktualisiert 6. August 2026 — Abend)

## ✅ PRIORITÄT 0 ABGESCHLOSSEN: Alle Build-Blocker behoben

> Ergebnis der Integritätsprüfung 6. Aug 2026: Der Workspace kompilierte NICHT.
> Ursache war die Error-Type-Konsolidierung (`DisplayManagerError`): Varianten
> wurden umbenannt/umstrukturiert, aber Tests + Windows-Backends nutzten noch
> alte Signaturen. **Alle Blocker wurden behoben.** Der Workspace kompiliert
> jetzt mit **0 Fehlern** über alle Targets
> (`cargo check --workspace --all-targets` → 0 Fehler).

### A. df_core — ERLEDIGT

### B. df_ddc — UnsupportedFeature Tuple-Variant — ERLEDIGT
`UnsupportedFeature` ist jetzt `UnsupportedFeature(String)`. Überall String übergeben, in Patterns `UnsupportedFeature(_)`.

### C. df_ddc — entfernte Context-Methoden — ERLEDIGT
`context()`, `with_vcp_code()`, `with_bus_path()` existieren nicht mehr (siehe error.rs).
      `test_ddc_error_with_bus_path` gelöscht (Methoden sind laut error.rs entfernt)

### D. df_ddc — nicht existierende Varianten + Conversions — ERLEDIGT
      `.into()` erhält die Variante. Match auf `&dm_err` (move-safe) mit `other`-Binding.

### E. df_ddc — Benchmark — ERLEDIGT
      aber `criterion_main!` referenzierte sie immer. Fix: No-Op-Stub
      `bench_parallel_probing` für `#[cfg(not(target_os = "linux"))]`, Group unconditional.

### F. df_displmgr — 17 Build-Fehler — ERLEDIGT
**F1. DisplayId/String-Mismatch (6x)** — `MonitorNotFound(String)` bekam `DisplayId`. Mit `.to_string()` konvertiert:

**F2. unsafe_op_in_unsafe_fn (11x)** in `displmgr_gdi_api.rs` — jeden Aufruf in `unsafe {}` eingeschlossen (mit SAFETY-Kommentaren):

**F3. Test-Import + weitere Tests**
      (`pub use error::{DisplayError, DisplayManagerError, DisplayResult};`)
      Timeout-Assertion `timed out` → `timeout`

### G. df_displmgr_info — 3 Test-Fehler — ERLEDIGT
      (entspricht der tatsächlichen `From<EdidError>`-Abbildung in error.rs)

### H. Root-Integrationstests — ERLEDIGT (Nutzer-Entscheidung: löschen)
      Begründung: Root-Cargo.toml ist virtuelles Manifest (kein `[package]`), die Tests
      wurden nie kompiliert/ausgeführt (verwaist), enthielten kaputte Muster und sind
      durch die Crate-eigenen Tests abgedeckt.

### I. Aufräumen + Verifikation — ERLEDIGT
      Doc-Beispiel korrigiert (`MonitorNotFound("HDMI-1".to_string())`)

---

## Verbleibende Prioritäten (größere Änderungen, für nächste Session)

### 1. String-basierte Fehler vermeiden
- `BackendNotAvailable { details: String }` durch strukturierte enum-Varianten ersetzen (z.B. `BackendNotAvailable`, `BackendNotAvailableOnPlatform`, `BackendDisabled`)

### 2. Trait-Object-Overhead reduzieren
- `Box<dyn DdcControl>` in `DisplayDevice` durch generische `impl DdcControl<T>` ersetzen

### 3. Boilerplate-Tests bereinigen
- Redundante Tests identifizieren und entfernen (~150+)
- `test_vcp_code_equality`, `test_display_id_hash`, `Default`-Tests für triviale Structs entfernen

### 4. Deprecated-Aliase migrieren (niedrige Prio)
- `DdcError`/`DisplayError`-Type-Aliase in Tests schrittweise durch `DisplayManagerError`
  ersetzen, um die deprecated-Warnings zu reduzieren (derzeit nur Warnings, kein Build-Blocker)

---

## Feature Items (P3/P4 - unverändert)
- [ ] HDR over Wayland (color-management-v1)
- [ ] XDG Shell integration
- [ ] EDID 1.4 Feature Unterstützung vervollständigen
- [ ] EDID 2.0 Unterstützung
- [ ] CCD-basiertes EDID Lesen (Windows)
- [ ] DDC/CI-basiertes EDID Lesen (Linux)
- [ ] Vollständiges DisplayID Parsing
- [ ] Compositor-Unterstützung prüfen (wlroots, Mutter, etc.)
- [ ] Timing-Parameter validieren

---

## Notizen
- **6. Aug 2026 (Abend):** ALLE Priorität-0-Build-Blocker behoben. Workspace kompiliert
  mit 0 Fehlern über alle Targets. Root-Tests gelöscht (verwaist).
- **Integritätsprüfung-Ursache:** Error-Type-Konsolidierung nicht nachgezogen
  (Varianten-Renames `DdcCommunication`→`CommunicationFailed`, `Platform`→entfernt,
  `NotFound`→`MonitorNotFound`, `UnsupportedFeature` Unit→Tuple, DisplayId/String).
- `df_displmgr_info`-Lib kompiliert sauber (nur 3 Test-Fehler, jetzt behoben).
- Test-Count vor Root-Test-Löschung: 545 Tests (df_core: 84, df_ddc: 77, df_displmgr: 135, df_displmgr_info: 121, CLI: 63, integration: 65). Die 65 verwaisten Root-Integrationstests wurden entfernt.
- Expert-Level-Analyse vom 28. Juli 2026: Bewertung 65/100
