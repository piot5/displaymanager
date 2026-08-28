# GPU Backend Checklist für DisplayManager Workspace

## 1. AMD Display Library (ADL) Spezifika

### ⚠️ AMD-DAL Spezifikation (aus AMD-DAL-Library-Spec.pdf)
- [ ] **Nicht direkt relevant**: AMD ADL ist proprietär und Windows-only
- [ ] **Alternative**: Nutzung von DDC/CI statt proprietärer AMD APIs
- [ ] **Alternative**: Open-Source AMDGPU-DRV auf Linux

### ⚠️ AMD GPU Unterstützung
- [ ] **Limitation**: Workspace nutzt keine AMD-spezifischen APIs
- [ ] **Empfehlung**: DDC/CI ist Hersteller-neutral und vorzuziehen

## 2. GPU Enumeration (generisch)

### ✅ Plattform-übergreifendes Vorgehen
- [x] Windows: EnumDisplayDevices für GPU/Monitor Enumeration
- [x] Linux: `/sys/class/drm/` für DRM-Karten
- [x] Linux: `/dev/i2c-*` für DDC/CI-Zugriff

## 3. GPU-Features im Workspace

### ⚠️ Noch nicht implementiert
- [ ] GPU-Temperatur via DRM
- [ ] GPU-Auslastung via sysfs
- [ ] VRAM-Auslastung (Linux)
- [ ] GPU-Produkt-Information via EDID

## 4. GPU-agnostische Ansatz

### ✅ Empfohlener Weg
- [x] DDC/CI für Monitor-Steuerung (Hersteller-neutral)
- [x] EDID für Monitor-Erkennung (Hersteller-neutral)
- [ ] **Optional**: GPU-spezifische Erweiterungen als Feature-Plugins