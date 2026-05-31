#[cfg(target_os = "windows")]
pub mod edid_win_reg;
#[cfg(target_os = "windows")]
pub mod edid_win_ddc;
#[cfg(target_os = "windows")]
pub mod edid_win_ccd;

#[cfg(target_os = "linux")]
pub mod edid_linux_sys;
#[cfg(target_os = "linux")]
pub mod edid_linux_ddc;