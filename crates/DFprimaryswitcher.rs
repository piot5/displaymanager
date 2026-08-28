use std::{env, fs::OpenOptions, io::{self, Write}, mem, iter, process::Command, collections::HashMap};
use chrono::Local;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplaySettingsW, ChangeDisplaySettingsExW,
    DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS,
    CDS_UPDATEREGISTRY, CDS_NORESET, CDS_SET_PRIMARY,
    DM_PELSWIDTH, DM_PELSHEIGHT, DM_POSITION, CDS_TYPE, DISP_CHANGE_SUCCESSFUL
};
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

#[derive(Debug, Clone)]
struct DisplayTask {
    query: String,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    is_primary: bool,
    clone_from: Option<String>, // Klon-Erweiterung für V3.1
}

struct MMTEngine {
    log_path: String,
}

impl MMTEngine {
    fn new() -> Self {
        // DPI-Integrität (aus V3.0)
        unsafe { let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }
        Self { log_path: "mmt_execution.log".to_string() }
    }

    fn log(&self, status: &str, msg: &str) {
        let ts = Local::now().format("%H:%M:%S");
        let output = format!("[{}][{}] {}", ts, status, msg);
        if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(&self.log_path) {
            let _ = writeln!(f, "{}", output);
        }
        println!("{}", output);
    }

    pub fn execute_integrated(&self, tasks: Vec<DisplayTask>, save_name: Option<String>, use_hotkey: bool) {
        // A. Force-Activation: Weckt Hardware auf
        unsafe { let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND); }

        let mut applied_configs: HashMap<String, (i32, i32)> = HashMap::new();
        let mut sorted = tasks.clone();
        
        // B. Sortierung: Primary -> Normal -> Clones (Klon-Fix)
        sorted.sort_by_key(|t| {
            if t.is_primary { 0 } 
            else if t.clone_from.is_none() { 1 } 
            else { 2 }
        });

        for task in &sorted {
            let mut found = false;
            for i in 0..64 { 
                unsafe {
                    let mut dd: DISPLAY_DEVICEW = mem::zeroed();
                    dd.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
                    if !EnumDisplayDevicesW(None, i, &mut dd, 0).as_bool() { break; }
                    
                    let device_name = String::from_utf16_lossy(&dd.DeviceName).trim_matches('\0').to_string();
                    let device_id = String::from_utf16_lossy(&dd.DeviceID).trim_matches('\0').to_string();

                    if device_name.contains(&task.query) || device_id.contains(&task.query) {
                        // Klon-Integrität: Erzwinge Koordinaten des Masters
                        let forced_pos = task.clone_from.as_ref().and_then(|cf| applied_configs.get(cf)).cloned();
                        
                        if self.apply_to_registry(&device_name, task, forced_pos) {
                            self.log("OK", &format!("Konfiguriert: {} (Clone: {:?})", device_name, task.clone_from));
                            applied_configs.insert(task.query.clone(), (task.x, task.y));
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found { self.log("ERROR", &format!("Nicht gefunden: {}", task.query)); }
        }

        // C. Atomarer Commit (aus V3.0)
        unsafe { ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
        
        if let Some(name) = save_name {
            self.generate_deployment_package(&name, &tasks, use_hotkey);
        }
    }

    fn apply_to_registry(&self, name: &str, task: &DisplayTask, forced_pos: Option<(i32, i32)>) -> bool {
        let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
        if task.is_primary { flags |= CDS_SET_PRIMARY; }

        let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        unsafe {
            let mut dm: DEVMODEW = mem::zeroed();
            dm.dmSize = mem::size_of::<DEVMODEW>() as u16;

            if EnumDisplaySettingsW(PCWSTR::from_raw(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
                dm.dmPelsWidth = task.width as u32;
                dm.dmPelsHeight = task.height as u32;
                
                // Klon-Logik (aus V3.1)
                let (final_x, final_y) = forced_pos.unwrap_or((task.x, task.y));
                dm.Anonymous1.Anonymous2.dmPosition.x = final_x;
                dm.Anonymous1.Anonymous2.dmPosition.y = final_y;
                
                dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
                let res = ChangeDisplaySettingsExW(PCWSTR::from_raw(name_u16.as_ptr()), Some(&dm), None, flags, None);
                return res == DISP_CHANGE_SUCCESSFUL;
            }
        }
        false
    }

    fn generate_deployment_package(&self, name: &str, tasks: &[DisplayTask], use_hotkey: bool) {
        let exe = match env::current_exe() { Ok(e) => e, Err(_) => return };

        // 1. Batch Payload (Erweitert um Klon-Support)
        let batch_path = format!("{}.bat", name);
        let mut cmd = format!("@echo off\n\"{}\" ", exe.display());
        for t in tasks {
            cmd.push_str(&format!("{}:{}:{}:{}:{}:{}{} ", 
                t.query, t.width, t.height, t.x, t.y, 
                if t.is_primary {1} else {0},
                t.clone_from.as_ref().map(|cf| format!(":{}", cf)).unwrap_or_default()
            ));
        }
        let _ = std::fs::write(&batch_path, cmd);

        // 2. VBS Silent Wrapper
        let vbs_path = format!("{}.vbs", name);
        let vbs_content = format!("Set WshShell = CreateObject(\"WScript.Shell\")\nWshShell.Run \"cmd /c \"\"{}\"\"\", 0, True", batch_path);
        let _ = std::fs::write(&vbs_path, vbs_content);

        // 3. Hotkey Deployment via PowerShell
        if use_hotkey {
            print!("Ziel-Hotkey (z.B. CTRL+ALT+F10): ");
            let _ = io::stdout().flush();
            let mut hotkey = String::new();
            let _ = io::stdin().read_line(&mut hotkey);
            let hotkey = hotkey.trim();

            let desktop = env::var("USERPROFILE").unwrap_or_default() + r"\Desktop";
            let lnk_path = format!(r"{}\{}.lnk", desktop, name);
            let full_vbs_path = std::fs::canonicalize(&vbs_path).unwrap_or_default();

            let ps_cmd = format!(
                "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');\
                 $s.TargetPath='wscript.exe';\
                 $s.Arguments='\"{}\"';\
                 $s.Hotkey='{}';\
                 $s.Save()",
                lnk_path, full_vbs_path.display(), hotkey
            );
            let _ = Command::new("powershell").args(["-Command", &ps_cmd]).output();
        }
        self.log("SAVE", "Paket .bat/.vbs/.lnk bereit.");
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Syntax: <ID:W:H:X:Y:P:[CLONE_ID]> [save:name] [-h]");
        return;
    }

    let mut tasks = Vec::new();
    let (mut save_name, mut use_hotkey) = (None, false);

    for arg in args.iter().skip(1) {
        if arg == "-h" { use_hotkey = true; }
        else if arg.starts_with("save:") { save_name = Some(arg[5..].to_string()); }
        else {
            let p: Vec<&str> = arg.split(':').collect();
            if p.len() >= 6 {
                tasks.push(DisplayTask {
                    query: p[0].to_string(),
                    width: p[1].parse().unwrap_or(1920),
                    height: p[2].parse().unwrap_or(1080),
                    x: p[3].parse().unwrap_or(0),
                    y: p[4].parse().unwrap_or(0),
                    is_primary: p[5] == "1",
                    clone_from: p.get(6).map(|s| s.to_string()),
                });
            }
        }
    }
    MMTEngine::new().execute_integrated(tasks, save_name, use_hotkey);
}
