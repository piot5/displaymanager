#[cfg(target_os = "windows")]
pub mod ddc_win;

#[cfg(target_os = "linux")]
pub mod ddc_linux;

pub mod ddc_debug;