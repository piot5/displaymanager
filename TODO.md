# Task Progress

## Phase 1: Studio auslagern in eigenes Repo
- [ ] Studio-Quellcode in eigenes Repo verschieben (piot5/displaymanager_studio)
- [ ] Eigenständiges Cargo.toml für Studio erstellen (mit path-deps zu df_* crates)
- [ ] Studio aus main workspace entfernen  
- [ ] Studio-Repo initialisieren und pushen
- [ ] Build testen (beide Projekte)

## Phase 2: Error-Handling in CLI und Crates
- [ ] `apps/displaymanager_cli/` - `let` durch `let` mit Fehlerbehandlung ersetzen
- [ ] `crates/df_ddc/src/` - Fehlerbehandlung prüfen und verbessern
- [ ] `crates/df_displmgr/src/` - Fehlerbehandlung prüfen und verbessern
- [ ] `crates/df_displmgr_info/src/` - Fehlerbehandlung prüfen und verbessern
- [ ] Alle `unwrap()`/`expect()` durch `?` oder `context()` ersetzen
- [ ] `cargo clippy` clean (keine warnings)
- [ ] `cargo build` clean