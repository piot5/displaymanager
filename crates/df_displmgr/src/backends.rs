/// This module acts as the internal router for platform-specific implementations.
/// It uses conditional compilation (`cfg`) to ensure only the relevant code 
/// for the target operating system is compiled.

#[cfg(target_os = "windows")]
pub mod windows; // This automatically points to src/backends/windows.rs

#[cfg(target_os = "linux")]
pub mod linux; // This automatically points to src/backends/linux.rs

/// Re-export the primary topology for the current platform.
/// This allows the rest of the crate to use `NativeTopology` regardless of the OS.

#[cfg(target_os = "windows")]
/// We point to WinDisplayManager in the windows module, which routes 
/// between CCD and GDI based on the environment (e.g., Local vs RDP).
pub use self::windows::WinDisplayManager as NativeTopology;

#[cfg(target_os = "linux")]
pub use self::linux::WlrTopology as NativeTopology;

#[allow(unused_imports)]
use crate::traits::{UniversalTopology, OutputEditable};