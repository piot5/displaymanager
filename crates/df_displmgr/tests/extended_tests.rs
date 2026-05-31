use df_displmgr::types::*;

#[test]
fn outputstate_default_and_landscape() {
    let mut s = OutputState::default();
    // default geometry is 0x0 -> treated as landscape (width >= height)
    assert_eq!(s.scale, 1.0);
    assert_eq!(s.hdr_state, HdrState::Disabled);
    // set a landscape geometry
    s.geometry.size.width = 1920;
    s.geometry.size.height = 1080;
    s.rotation = DisplayRotation::Rotate0;
    assert!(s.is_landscape());

    // portrait when rotated 90 degrees
    s.rotation = DisplayRotation::Rotate90;
    assert!(!s.is_landscape());

    // swapped sizes with rotation should be considered landscape
    s.geometry.size.width = 1080;
    s.geometry.size.height = 1920;
    s.rotation = DisplayRotation::Rotate90;
    assert!(s.is_landscape());
}

#[test]
fn refresh_rate_hz_conversion() {
    let mut s = OutputState::default();
    s.refresh_rate = 60000; // 60 Hz represented as 60000 mHz
    assert!((s.refresh_rate_hz() - 60.0).abs() < 0.001);
}

#[test]
fn wide_roundtrip() {
    use df_displmgr::backends::windows::displmgr_gdi::displmgr_gdi_sys::{to_wide, from_wide};
    let text = "Hello, 世界";
    let wide = to_wide(text);
    // last element must be NUL
    assert_eq!(*wide.last().unwrap(), 0);
    let out = from_wide(&wide);
    assert_eq!(out, text.to_string());
}

#[cfg(windows)]
#[test]
fn create_empty_devmode_has_size() {
    use std::mem;
    use df_displmgr::backends::windows::displmgr_gdi::displmgr_gdi_sys::create_empty_devmode;
    use windows::Win32::Graphics::Gdi::DEVMODEW;
    let dm = create_empty_devmode();
    assert_eq!(dm.dmSize as usize, mem::size_of::<DEVMODEW>());
}

#[test]
fn gdi_flag_constants_sanity() {
    use df_displmgr::backends::windows::displmgr_gdi::displmgr_gdi_sys::gdi_flags;
    // GLOBAL_FLUSH_FLAGS should be zero
    assert_eq!(gdi_flags::GLOBAL_FLUSH_FLAGS.0, 0);
    // STAGE_FLAGS should not equal GLOBAL_FLUSH_FLAGS
    assert_ne!(gdi_flags::STAGE_FLAGS.0, gdi_flags::GLOBAL_FLUSH_FLAGS.0);
}
