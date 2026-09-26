# To do Checklist

## Abgeschlossen 26. September 2026 — Code-Qualität & Tooling
      → Suite 44s → 22s und nicht mehr flaky (Build-Lock-Kontention)
      FFI 12, Doku 9, CI 9, Hygiene 4) — CI-Punkte nur lokal verifiziert, kein GitHub-Lauf

## Nächste Kandidaten (aus dem Review vom 26.09.2026)
- [ ] 3. `rects_overlap` dupliziert in info.rs:730, set.rs:524 (+ Testkopien) → nach df_core ziehen
- [ ] 5. `df_displmgr/src/lib.rs` (1206 Zeilen) in activation/restore-Module aufteilen
- [ ] 6. Property-Tests (proptest) für EDID-Parser und GTF/CVT-Timing, `mockall` für die Trait-Ebene
- [ ] 7. Coverage-Zahl im Repo dokumentieren (nur HTML-Artefakt vorhanden)
- [ ] 8. `crates/DFprimaryswitcher.rs` entfernen (verwaist, nutzt `chrono` ohne Dependency)
- [ ] 9. Stringly-typed `BackendError(format!(...))` in df_displmgr/src/lib.rs:156-213 reduzieren
- [ ] 10. Linux-Backends (~1850 Zeilen) sind ohne grüne Ubuntu-CI unverifiziert
- [ ] 11. `cargo doc --workspace --no-deps`: 17 verbleibende Link-Warnungen (plattform-/feature-gebunden:
      `LinuxTopology`, `WlrTopology`, `list_monitors_with_progress`) + Modul-Links (`EdidError`,
      `overlap`, `unsupported`, `OutputState`, `WinDisplayManager`) — rustdoc ist bisher nicht in der CI

## Priorität 0 — Build-Blocker (alle erledigt)

## Verbleibende Prioritäten
- [ ] 1. String-basierte Fehler vermeiden (BackendNotAvailable → strukturierte enum-Varianten)
- [ ] 2. Trait-Object-Overhead reduzieren (Box<dyn DdcControl> → generisch)

## Feature Items (P3/P4)
- [ ] HDR over Wayland (color-management-v1)
- [ ] XDG Shell integration
- [ ] EDID 1.4 Feature Unterstützung vervollständigen
- [ ] EDID 2.0 Unterstützung
- [ ] CCD-basiertes EDID Lesen (Windows)
- [ ] DDC/CI-basiertes EDID Lesen (Linux)
- [ ] Vollständiges DisplayID Parsing
- [ ] Compositor-Unterstützung prüfen (wlroots, Mutter, etc.)
- [ ] Timing-Parameter validieren
