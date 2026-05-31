use windows::Win32::Graphics::Gdi::{DEVMODEW, CDS_TYPE, CDS_UPDATEREGISTRY, CDS_NORESET, CDS_SET_PRIMARY};

/// Converts a standard Rust string slice (`&str`) into a null-terminated
/// UTF-16 wide string (`Vec<u16>`) required for Win32 FFI interactions.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Converts a slice of UTF-16 wide characters (`&[u16]`) back into a standard
/// Rust `String`. Automatically truncates at the first null terminator.
pub fn from_wide(data: &[u16]) -> String {
    let end = data.iter().position(|&c| c == 0).unwrap_or(data.len());
    String::from_utf16_lossy(&data[..end]).to_string()
}

/// Initialises an empty `DEVMODEW` structure with its `dmSize` field correctly
/// set. This is mandatory before passing the structure to Win32 GDI functions
/// such as `EnumDisplaySettingsW`.
pub fn create_empty_devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

/// Encapsulates raw Windows GDI bitfield flags used for registry staging and
/// the global display layout flush pipeline.
pub mod gdi_flags {
    use super::*;

    /// Flags used to write changes to the registry without triggering an
    /// immediate hardware reset. Changes are batched and applied together
    /// during the final global flush.
    pub const STAGE_FLAGS: CDS_TYPE = CDS_TYPE(CDS_UPDATEREGISTRY.0 | CDS_NORESET.0);

    /// Flags required to register and promote a monitor as the Primary Display
    /// inside the registry without immediately resetting the hardware.
    pub const STAGE_PRIMARY_FLAGS: CDS_TYPE = CDS_TYPE(
        CDS_UPDATEREGISTRY.0 | CDS_NORESET.0 | CDS_SET_PRIMARY.0,
    );

    /// Flags for the final global flush step.
    ///
    /// Passing `CDS_TYPE(0)` — no flags — together with a null device name and
    /// a null `DEVMODEW` pointer in `ChangeDisplaySettingsExW` instructs the
    /// kernel to read all previously staged registry entries and render the new
    /// layout immediately without any further reset or registry write.
    ///
    /// Note: `CDS_RESET` (0x40000000) would instead *restore* the previously
    /// saved state, which is the opposite of what the flush step requires.
    pub const GLOBAL_FLUSH_FLAGS: CDS_TYPE = CDS_TYPE(0);
}