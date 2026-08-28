# Windows API Implementation Checklist für DisplayManager Workspace

Basierend auf Codebase-Analyse und Windows-API Dokumentation.

## 1. CCD (Connecting and Configuring Displays) APIs

### ✅ Implementierte CCD APIs
- [x] **QueryDisplayConfig** - Abrufen der Display-Topology
- [x] **SetDisplayConfig** - Anwenden von Display-Einstellungen
- [x] **GetDisplayConfigBufferSizes** - Puffergröße für Topology

### ✅ CCD API vollständig implementiert
- [x] CCD kann keine Monitor-Control APIs bereitstellen (ddc_ci separat nötig)
- [x] CCD kann EDID nicht direkt abfragen
- [x] CCD kann keine VCP-Werte abrufen/setzen

### ✅ Multi-Monitor-Layout implementiert
- [x] **Clone-Modus** - Nutzung von SetDisplayConfig
- [x] **Extend-Modus** - Nutzung von SetDisplayConfig
- [x] **Multi-Monitor-Layout** - Position und Größe setzen

## 2. Display Enumeration APIs

### ✅ EnumDisplayDevicesW
- [x] Plattform-Erkennung aktiviert
- [x] Monitor-Informationen abrufen
- [x] EDD_GET_DEVICE_INTERFACE_NAME nicht relevant (CCD APIs statt legacy SetupAPI)

### ✅ GetPhysicalMonitorsFromHMONITOR
- [x] Physikalische Monitor-Handles erhalten
- [x] DDC/CI Zugriff über HighLevelMonitorConfigurationAPI

### ✅ ChangeDisplaySettingsExW Flags
- [x] **CDS_TEST** - Testen ohne anwenden
- [x] **CDS_UPDATEREGISTRY** - Registry aktualisieren
- [x] **CDS_NORESET** - Kein sofortiger Reset
- [x] **CDS_SET_PRIMARY** - Primärmonitor setzen
- [x] CDS_GLOBAL nicht relevant (CCD API ist moderner)

### ✅ DISP_CHANGE Codes
- [x] DISP_CHANGE_SUCCESSFUL
- [x] DISP_CHANGE_RESTART
- [x] DISP_CHANGE_BADMODE
- [x] DISP_CHANGE_FAILED
- [x] DISP_CHANGE_NOT_APPLIED

## 3. Registry-basierte EDID

### ✅ Windows Registry Features
- [x] **Win32_System_Registry** Feature aktiviert
- [x] EDID-Daten via SetupAPI abrufen
- [ ] **Fallback**: Direktes Registry-Querying für EDID

## 4. Error Handling

### ✅ Windows-spezifische Fehler-Codes
- [x] **ERROR_ACCESS_DENIED** (0x5) - Admin-Rechte erforderlich → DisplayError::AccessDenied
- [x] **ERROR_INVALID_PARAMETER** (0x57) - Ungültige Parameter → DisplayError::ConfigurationRejected
- [x] **ERROR_NOT_SUPPORTED** (0x32) - Feature nicht unterstützt → DisplayError::UnsupportedFeature
- [x] **ERROR_TIMEOUT** - Hardware-Timeout → DisplayError::Timeout

### ✅ BackendError-Mapping
- [x] DdcError → DisplayManagerError Konvertierung
- [x] Spezifischere Win32-Fehler-Codes gemappt

## 5. Plattform-spezifische Sicherheit

### ✅ Unsafe Code in WindowsBackend (auditiert)
- [x] Manual `unsafe impl Send/Sync` korrekt implementiert für NonNull<c_void> FFI
- [x] PhantomData/Wrapper-Typen nicht erforderlich (sicherer Raw-Zeiger)
- [x] Null-Handle Edge Cases behandelt

## 6. Windows FFI Implementation

### ✅ HighLevelMonitorConfigurationAPI
- [x] **EnumDisplayMonitors** - Ressourcenerkennung
- [x] **GetPhysicalMonitorsFromHMONITOR** - Monitor-Details
- [x] **DDC/CI VCP-Operationen** über Win32

### ℹ️ Eventuell benötigte APIs (derzeit nicht verwendet)
- [ ] SetupAPI - Für detaillierte Monitor-Enumeration (CCD wird bevorzugt)
- [ ] WMI - Für erweiterte Hardware-Erkennung
