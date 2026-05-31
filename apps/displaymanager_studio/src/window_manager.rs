// During incremental refactor, allow dead code to preserve API shape
#![allow(dead_code)]
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ServerOptions;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use eframe::egui;
use crate::loader::AppState;

// --- Data structures ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SlotRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ZOrder {
    TopMost,
    NoTopMost,
    Top,
    Bottom,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case")]
pub enum WindowTarget {
    Title(String),
    Pid(u32),
    ClassName(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Empty,
    Attached,
    Stale,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    AddSlot { id: u32, rect: SlotRect, z_order: ZOrder },
    Attach { slot_id: u32, target: WindowTarget },
    Detach { slot_id: u32 },
    GetState,
    SyncAll,
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SlotState {
    pub id: u32,
    pub status: HealthStatus,
    pub title: Option<String>,
    pub rect: SlotRect,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub success: bool,
    pub data: Option<Vec<SlotState>>,
    pub error: Option<WindowError>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Error)]
pub enum WindowError {
    #[error("Window not found")]
    NotFound,
    #[error("Slot {0} missing")]
    SlotMissing(u32),
    #[error("IPC Failure")]
    IpcError,
}

// --- Manager Kern ---

static EVENT_TRIGGERED: AtomicBool = AtomicBool::new(false);
const PIPE_NAME: &str = r"\\.\pipe\displayflow_wm";
const CLASS_NAME: &str = "DisplayFlowSlotAnchor";

pub struct AutoHook(HWINEVENTHOOK);
impl Drop for AutoHook {
    fn drop(&mut self) { unsafe { UnhookWinEvent(self.0); } }
}

pub struct AutoWindow(HWND);
impl Drop for AutoWindow {
    fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.0); } }
}

pub struct Slot {
    pub id: u32,
    pub rect: RECT,
    pub anchor: Arc<AutoWindow>,
    pub attached_hwnd: Option<HWND>,
    pub status: HealthStatus,
    pub original_style: i32,
    pub z_order: ZOrder,
}

struct EnumCtx { target_pid: u32, result: Option<HWND> }

pub struct WindowManager {
    pub slots: RwLock<HashMap<u32, Slot>>,
    sync_mutex: Mutex<()>,
    _hook: RwLock<Option<AutoHook>>,
}

impl WindowManager {
    pub fn new() -> Self {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let h_inst = GetModuleHandleW(None).unwrap();
            let name: Vec<u16> = CLASS_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                hInstance: h_inst.into(),
                lpszClassName: PCWSTR(name.as_ptr()),
                hbrBackground: HBRUSH(0),
                ..Default::default()
            };
            RegisterClassW(&wc);
        }
        Self { 
            slots: RwLock::new(HashMap::with_capacity(32)), 
            sync_mutex: Mutex::new(()), 
            _hook: RwLock::new(None) 
        }
    }

    unsafe extern "system" fn wnd_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT { 
        DefWindowProcW(h, m, w, l) 
    }

    pub fn add_slot(&self, id: u32, r: SlotRect, z: ZOrder) {
        let mut slots = self.slots.write();
        let name: Vec<u16> = CLASS_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let h = CreateWindowExW(
                WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0 | WS_EX_NOACTIVATE.0),
                PCWSTR(name.as_ptr()), PCWSTR(std::ptr::null()), WS_POPUP,
                r.left, r.top, r.right - r.left, r.bottom - r.top, 
                None, None, GetModuleHandleW(None).unwrap(), None
            );
            let _ = SetLayeredWindowAttributes(h, COLORREF(0), 0, LWA_ALPHA);
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            slots.insert(id, Slot {
                id, rect: RECT { left: r.left, top: r.top, right: r.right, bottom: r.bottom },
                anchor: Arc::new(AutoWindow(h)), attached_hwnd: None, 
                status: HealthStatus::Empty, original_style: 0, z_order: z
            });
        }
    }

    pub fn attach(&self, id: u32, target: WindowTarget) -> Result<(), WindowError> {
        let _g = self.sync_mutex.lock();
        let hwnd = match target {
            WindowTarget::Title(t) => unsafe { 
                let t_u16: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
                FindWindowW(None, PCWSTR(t_u16.as_ptr()))
            },
            WindowTarget::Pid(p) => self.find_by_pid(p).ok_or(WindowError::NotFound)?,
            WindowTarget::ClassName(c) => unsafe {
                let c_u16: Vec<u16> = c.encode_utf16().chain(std::iter::once(0)).collect();
                FindWindowW(PCWSTR(c_u16.as_ptr()), None)
            }
        };
        if hwnd.0 == 0 { return Err(WindowError::NotFound); }
        let mut slots = self.slots.write();
        let s = slots.get_mut(&id).ok_or(WindowError::SlotMissing(id))?;
        unsafe {
            s.original_style = GetWindowLongW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, s.anchor.0.0 as isize);
            s.attached_hwnd = Some(hwnd);
            s.status = HealthStatus::Attached;
        }
        self.perform_sync(s)
    }

    pub fn detach(&self, id: u32) -> Result<(), WindowError> {
        let mut slots = self.slots.write();
        let s = slots.get_mut(&id).ok_or(WindowError::SlotMissing(id))?;
        if let Some(h) = s.attached_hwnd {
            unsafe {
                SetWindowLongPtrW(h, GWLP_HWNDPARENT, 0);
                let _ = SetWindowPos(h, HWND_NOTOPMOST, 100, 100, 800, 600, SWP_SHOWWINDOW|SWP_FRAMECHANGED);
            }
        }
        s.attached_hwnd = None;
        s.status = HealthStatus::Empty;
        Ok(())
    }

    fn find_by_pid(&self, pid: u32) -> Option<HWND> {
        let mut ctx = EnumCtx { target_pid: pid, result: None };
        unsafe { let _ = EnumWindows(Some(Self::enum_proc), LPARAM(&mut ctx as *mut _ as isize)); }
        ctx.result
    }

    unsafe extern "system" fn enum_proc(h: HWND, lp: LPARAM) -> BOOL {
        let ctx = &mut *(lp.0 as *mut EnumCtx);
        let mut p = 0;
        let _ = GetWindowThreadProcessId(h, Some(&mut p));
        if p == ctx.target_pid && IsWindowVisible(h).as_bool() {
            let style = GetWindowLongW(h, GWL_STYLE) as u32;
            if (style & WS_CHILD.0) == 0 { ctx.result = Some(h); return BOOL(0); }
        }
        BOOL(1)
    }

    pub fn perform_sync(&self, s: &mut Slot) -> Result<(), WindowError> {
        if let Some(h) = s.attached_hwnd {
            unsafe {
                if !IsWindow(h).as_bool() { s.status = HealthStatus::Stale; return Ok(()); }
                let r = s.rect;
                let z = match s.z_order {
                    ZOrder::TopMost => HWND_TOPMOST,
                    ZOrder::NoTopMost => HWND_NOTOPMOST,
                    ZOrder::Bottom => HWND_BOTTOM,
                    _ => HWND_TOP,
                };
                let _ = SetWindowPos(h, z, r.left, r.top, r.right-r.left, r.bottom-r.top, SWP_NOACTIVATE|SWP_ASYNCWINDOWPOS|SWP_SHOWWINDOW);
            }
        }
        Ok(())
    }

    pub fn setup_hooks(&self) {
        unsafe {
            let h = SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(Self::winevent_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
            *self._hook.write() = Some(AutoHook(h));
        }
    }

    unsafe extern "system" fn winevent_proc(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) { 
        EVENT_TRIGGERED.store(true, Ordering::Relaxed); 
    }

    pub async fn listen_ipc(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let mut srv = ServerOptions::new().first_pipe_instance(true).create(PIPE_NAME)?;
            srv.connect().await?;
            let mgr = Arc::clone(&self);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                while let Ok(n) = srv.read(&mut buf).await {
                    if n == 0 { break; }
                    if let Ok(cmd) = serde_json::from_slice::<Command>(&buf[..n]) {
                        let res = match cmd {
                            Command::AddSlot { id, rect, z_order } => { 
                                mgr.add_slot(id, rect, z_order); 
                                Response { success: true, data: None, error: None } 
                            },
                            Command::Attach { slot_id, target } => match mgr.attach(slot_id, target) {
                                Ok(_) => Response { success: true, data: None, error: None },
                                Err(e) => Response { success: false, data: None, error: Some(e) }
                            },
                            Command::Detach { slot_id } => match mgr.detach(slot_id) {
                                Ok(_) => Response { success: true, data: None, error: None },
                                Err(e) => Response { success: false, data: None, error: Some(e) }
                            },
                            Command::GetState => {
                                let s = mgr.slots.read();
                                let data = s.values().map(|sl| SlotState { 
                                    id: sl.id, status: sl.status.clone(), title: None, 
                                    rect: SlotRect { left: sl.rect.left, top: sl.rect.top, right: sl.rect.right, bottom: sl.rect.bottom }
                                }).collect();
                                Response { success: true, data: Some(data), error: None }
                            },
                            Command::SyncAll => {
                                { let mut s = mgr.slots.write(); for sl in s.values_mut() { let _ = mgr.perform_sync(sl); } }
                                Response { success: true, data: None, error: None }
                            },
                            Command::Shutdown => std::process::exit(0),
                        };
                        if let Ok(ser) = serde_json::to_vec(&res) { let _ = srv.write_all(&ser).await; }
                    }
                }
            });
        }
    }

    pub fn run_loop(&self) {
        let mut cd = 0;
        loop {
            unsafe {
                let mut m = MSG::default();
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() { 
                    TranslateMessage(&m); 
                    DispatchMessageW(&m); 
                    if m.message == WM_QUIT { return; }
                }
            }
            if EVENT_TRIGGERED.swap(false, Ordering::Relaxed) { cd = 10; }
            if cd > 0 { { let mut s = self.slots.write(); for sl in s.values_mut() { let _ = self.perform_sync(sl); } } cd -= 1; }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

// --- UI Integration ---

pub fn render_window_manager(ui: &mut egui::Ui, _state: &mut AppState) {    ui.heading("Window Layout Engine");
    ui.separator();

    ui.vertical(|ui| {
        ui.label("Active Window Slots:");
        
        // Here you could visualize the slots directly from the state
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Slots are controlled via configuration or IPC.");
            // Example visualization:
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Status: Engine Running");
                if ui.button("Sync all windows").clicked() {
                    // Logic to trigger SyncAll (via state or local reference)
                }
            });
        });
    });
}