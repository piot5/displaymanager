#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::io::{self, Write};
use std::process;
use std::env;
use std::ptr;
use std::thread;
use std::time::Duration;

use df_displmgr_info::edid_backends::edid_win_ddc::WindowsDdcBackend;
use df_displmgr_info::edid_types::VcpCode;

use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HMONITOR, HDC};
use windows::Win32::Foundation::{LPARAM, RECT, BOOL, HANDLE, HWND};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{w, PCWSTR, PCSTR};
use windows::Win32::System::LibraryLoader::{LoadLibraryW, GetProcAddress};

// ========================================================================
// NVAPI STRUCTURES & FUNCTION TYPES (Official NVIDIA interfaces)
// ========================================================================

type NvPhysicalGpuHandle = *mut std::ffi::c_void;
#[allow(dead_code)]
type NvDisplayHandle = *mut std::ffi::c_void;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct NV_I2C_INFO {
    version: u32,
    display_mask: u32,
    b_is_ddc_port: u8,       // 1 = forces bypassing OS kernel restrictions
    i2c_dev_addr: u8,        // 7-bit address shifted left (0x50 << 1 = 0xA0)
    i2c_reg_addr: *const u8, // internal offset (pointer to memory location)
    reg_addr_size: u32,      // size of the offset (usually 1 byte)
    pb_data: *mut u8,        // pointer to payload buffer
    cb_size: u32,            // number of bytes to write
    i2c_speed: u32,          // speed (kHz, e.g. 27 or 100)
}

// Function pointer types for NVAPI interface queries (QueryInterface)
type NvAPI_QueryInterface_Fn = unsafe extern "C" fn(id: u32) -> *const std::ffi::c_void;
type NvAPI_Initialize_Fn = unsafe extern "C" fn() -> i32;
type NvAPI_EnumPhysicalGPUs_Fn = unsafe extern "C" fn(handles: *mut NvPhysicalGpuHandle, count: *mut u32) -> i32;
type NvAPI_GetPhysicalGPUsFromDisplay_Fn = unsafe extern "C" fn(hNavDisp: NvDisplayHandle, handles: *mut NvPhysicalGpuHandle, count: *mut u32) -> i32;
type NvAPI_GetAssociatedDisplayOutputId_Fn = unsafe extern "C" fn(hNavDisp: NvDisplayHandle, output_id: *mut u32) -> i32;
type NvAPI_I2CWrite_Fn = unsafe extern "C" fn(hPhysicalGpu: NvPhysicalGpuHandle, pI2cInfo: *const NV_I2C_INFO) -> i32;

// NVAPI function IDs obtained from NVIDIA SDK disassembly
const NVAPI_INITIALIZE_ID: u32 = 0x0150E828;
const NVAPI_ENUM_PHYSICAL_GPUS_ID: u32 = 0xEAA33CF1;
const NVAPI_I2C_WRITE_ID: u32 = 0x4A97C258;

// Struct to hold the dynamically loaded NVAPI function pointers
struct NvApiBindings {
    initialize: NvAPI_Initialize_Fn,
    enum_gpus: NvAPI_EnumPhysicalGPUs_Fn,
    i2c_write: NvAPI_I2CWrite_Fn,
}

impl NvApiBindings {
    unsafe fn load() -> Result<Self, String> {
        let h_module = LoadLibraryW(w!("nvapi64.dll")).map_err(|e| format!("nvapi64.dll nicht gefunden: {:?}", e))?;
        
        let query_interface_ptr = GetProcAddress(h_module, PCSTR(b"nvapi_QueryInterface\0".as_ptr()))
            .ok_or_else(|| "nvapi_QueryInterface export missing in DLL".to_string())?;
            
        let query_interface: NvAPI_QueryInterface_Fn = std::mem::transmute(query_interface_ptr);

        let init_ptr = query_interface(NVAPI_INITIALIZE_ID);
        if init_ptr.is_null() { return Err("NvAPI_Initialize ID invalid".to_string()); }
        
        let enum_ptr = query_interface(NVAPI_ENUM_PHYSICAL_GPUS_ID);
        if enum_ptr.is_null() { return Err("NvAPI_EnumPhysicalGPUs ID invalid".to_string()); }
        
        let write_ptr = query_interface(NVAPI_I2C_WRITE_ID);
        if write_ptr.is_null() { return Err("NvAPI_I2CWrite ID invalid".to_string()); }

        Ok(NvApiBindings {
            initialize: std::mem::transmute(init_ptr),
            enum_gpus: std::mem::transmute(enum_ptr),
            i2c_write: std::mem::transmute(write_ptr),
        })
    }
}

// ========================================================================
// UTILS & PRIVILEGE ESCALATION (UAC)
// ========================================================================

fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            if GetTokenInformation(token, TokenElevation, Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _), size, &mut size).is_ok() {
                return elevation.TokenIsElevated != 0;
            }
        }
        false
    }
}

fn elevate_privileges() {
    if !is_elevated() {
        println!("[!] Administrator privileges required. Launching UAC elevation...");
        let current_exe = env::current_exe().unwrap();
        let path_w: Vec<u16> = current_exe.to_str().unwrap().encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("runas"),
                PCWSTR(path_w.as_ptr()),
                w!(""),
                w!(""),
                SW_SHOWNORMAL,
            );
        }
        process::exit(0);
    }
}

unsafe extern "system" fn monitor_enum_callback(monitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
    let monitors = lparam.0 as *mut Vec<isize>;
    (*monitors).push(monitor.0);
    BOOL::from(true)
}

// ========================================================================
// MAIN LOGIC & EDID GENERATOR
// ========================================================================

fn generate_1080p_edid() -> Vec<u8> {
    let mut edid = vec![0u8; 128];
    // 1. Fixed EDID header
    edid[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    
    // 2. Manufacturer ID (example: "MNT" -> 0x35, 0xAC)
    edid[8] = 0x35; edid[9] = 0xAC;
    
    // 3. Monitor name in descriptor block (detailed descriptor type 0xFC)
    // For simplicity we only use a standard 1080p timing starting at byte 54
    let timing_1080p_60hz = [
        0x02, 0x3A, 0x80, 0x18, 0x71, 0x38, 0x2D, 0x40, 
        0x58, 0x2C, 0x45, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1E
    ];
    edid[54..72].copy_from_slice(&timing_1080p_60hz);
    
    // 4. VESA checksum calculation (byte 127)
    let mut sum: u8 = 0;
    for i in 0..127 {
        sum = sum.wrapping_add(edid[i]);
    }
    edid[127] = (256u16).wrapping_sub(sum as u16) as u8;
    edid
}

fn main() {
    elevate_privileges();
    println!("======================================================");
    println!("        CRITICAL EEPROM HARDWARE WRITER ENGINE        ");
    println!("======================================================");

    // Secure loaded NVAPI instance
    let nvapi = unsafe {
        match NvApiBindings::load() {
            Ok(bindings) => {
                let status = (bindings.initialize)();
                if status != 0 {
                    println!("[-] NvAPI_Initialize fehlgeschlagen. Status-Code: {}", status);
                    return;
                }
                bindings
            }
            Err(e) => {
                println!("[-] NVAPI bindings error: {}", e);
                return;
            }
        }
    };
    println!("[+] NVAPI subsystem successfully initialized and bound.");

    // Enumerate monitors via GDI
    let mut monitor_handles: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(monitor_enum_callback),
            LPARAM(&mut monitor_handles as *mut _ as isize),
        );
    }

    if monitor_handles.is_empty() {
        println!("[-] No physical monitors found via Windows GDI.");
        return;
    }

    let target_hmonitor = monitor_handles[0];
    println!("[+] Target monitor handle located: HMONITOR (0x{:X})", target_hmonitor);

    // Enforce safety confirmation
    print!("\nWARNING! Overwriting the monitor EEPROM can permanently damage hardware (bricking).\n");
    print!("Please type 'CONFIRM' to proceed: ");
    io::stdout().flush().unwrap();
    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation).unwrap();
    if confirmation.trim() != "CONFIRM" {
        println!("[-] Operation cancelled by user.");
        return;
    }

    // Query scaler unlock via vendor-specific registers
    print!("Enter the vendor-specific VCP register (e.g. Realtek = 243): ");
    io::stdout().flush().unwrap();
    let mut reg_str = String::new();
    io::stdin().read_line(&mut reg_str).unwrap();
    let vendor_reg: u8 = reg_str.trim().parse().unwrap_or(0xF3);

    print!("Enter the unlock code (magic key hex, e.g. 0x55): ");
    io::stdout().flush().unwrap();
    let mut key_str = String::new();
    io::stdin().read_line(&mut key_str).unwrap();
    let vendor_key: u32 = u32::from_str_radix(key_str.trim().trim_start_matches("0x"), 16).unwrap_or(0x55);

    // Schritt 1: DDC/CI Unlock-Kommando absetzen
    println!("\n[1/2] Sending DDC/CI hardware unlock via Win32 subsystem...");
    let ddc_backend = WindowsDdcBackend { h_monitor: target_hmonitor };
    
    // Safe assignment of the transmuted u8 into the valid VcpCode type range
    let target_vcp_code: VcpCode = unsafe { std::mem::transmute(vendor_reg) };
    
    match ddc_backend.set_vcp_feature(target_vcp_code, vendor_key) {
        Ok(_) => println!("      Scaler unlock command to register 0x{:X} successfully sent.", vendor_reg),
        Err(e) => println!("      [WARN] DDC/CI unlock failed: {:?}. Attempting to continue write operation.", e),
    }

    // Generate data
    let target_edid = generate_1080p_edid();

    // Step 2: Obtain physical GPU handle
    let mut gpu_handles = [ptr::null_mut(); 64];
    let mut gpu_count: u32 = 0;
    let enum_status = unsafe { (nvapi.enum_gpus)(gpu_handles.as_mut_ptr(), &mut gpu_count) };
    if enum_status != 0 || gpu_count == 0 {
        println!("[-] Could not enumerate physical GPU handles via NVAPI.");
        return;
    }
    let primary_gpu = gpu_handles[0];

    // Step 3: EEPROM I2C write transactions (page/byte writes)
    println!("\n[2/2] Starting direct I2C EEPROM write transaction to address 0x50...");
    
    // EEPROM chips require byte-by-byte or 8-byte page writes with delays,
    // because a sequential 128-byte burst write will overflow the FIFO buffer.
    let chunk_size = 1; // safe byte-wise write mode
    let mut operational_success = true;

    for offset in (0..target_edid.len()).step_by(chunk_size) {
        let current_offset = offset as u8;
        let mut chunk_data = target_edid[offset..(offset + chunk_size)].to_vec();

        let i2c_transaction = NV_I2C_INFO {
            version: (std::mem::size_of::<NV_I2C_INFO>() as u32) | 0x30000, // Version 3 Mapping
            display_mask: 1 << 0, // primary output port
            b_is_ddc_port: 1,     // forces access bypassing Windows
            i2c_dev_addr: 0x50 << 1, // 8-bit addressing for NVAPI (0xA0)
            i2c_reg_addr: &current_offset as *const u8,
            reg_addr_size: 1,     // offset size: 1 byte for standard EEPROMs
            pb_data: chunk_data.as_mut_ptr(),
            cb_size: chunk_data.len() as u32,
            i2c_speed: 27,
        };

        let status = unsafe { (nvapi.i2c_write)(primary_gpu, &i2c_transaction) };
        if status != 0 {
            operational_success = false;
        }
        thread::sleep(Duration::from_millis(10));
    }

    if operational_success {
        println!("[+] EEPROM write completed successfully.");
    } else {
        println!("[-] EEPROM write encountered errors.");
    }
}
