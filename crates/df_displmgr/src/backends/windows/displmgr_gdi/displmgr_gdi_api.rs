use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use crate::error::DisplayResult;
use crate::types::{
    OutputState, DisplayRotation, HdrState, HdrMode,
    DisplayIdentity, DisplayId, ConnectorId, AdapterId,
    Rect, Point2D, Extent2D,
};
use super::displmgr_gdi_sys::{from_wide, to_wide, create_empty_devmode};
use std::collections::HashMap;

pub fn query_gdi_outputs() -> DisplayResult<(HashMap<String, DEVMODEW>, Vec<OutputState>)> {
    let mut staged_modes = HashMap::new();
    let mut outputs = Vec::new();
    let mut index = 0;

    loop {
        let mut device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        let found = unsafe { EnumDisplayDevicesW(None, index, &mut device, 0).as_bool() };
        if !found { break; }
        index += 1;

        let name_str = from_wide(&device.DeviceName);
        let mut dev_mode = create_empty_devmode();
        let wide_name = to_wide(&name_str);

        let success = unsafe {
            EnumDisplaySettingsW(
                PCWSTR(wide_name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode
            ).as_bool()
        };

        if success {
            let is_active = (device.StateFlags & 0x00000001) != 0;
            let is_primary = (device.StateFlags & 0x00000004) != 0;
            
            // Fix: Explicit unsafe blocks for accessing union fields
            let (orient, pos_x, pos_y) = unsafe {
                (
                    dev_mode.Anonymous1.Anonymous2.dmDisplayOrientation,
                    dev_mode.Anonymous1.Anonymous2.dmPosition.x,
                    dev_mode.Anonymous1.Anonymous2.dmPosition.y,
                )
            };

            let size = Extent2D {
                width: dev_mode.dmPelsWidth,
                height: dev_mode.dmPelsHeight,
            };

            outputs.push(OutputState {
                identity: DisplayIdentity {
                    id:           DisplayId(name_str.clone()),
                    connector_id: ConnectorId(name_str.clone()),
                    adapter_id:   AdapterId(from_wide(&device.DeviceID)),
                    hardware_uuid: None,
                    monitor_name:  from_wide(&device.DeviceString),
                },
                geometry: Rect {
                    origin: Point2D { x: pos_x, y: pos_y },
                    size,
                },
                refresh_rate: dev_mode.dmDisplayFrequency * 1000,
                rotation: match orient {
                    DMDO_90  => DisplayRotation::Rotate90,
                    DMDO_180 => DisplayRotation::Rotate180,
                    DMDO_270 => DisplayRotation::Rotate270,
                    _        => DisplayRotation::Rotate0,
                },
                enabled:          is_active,
                is_primary,
                hdr_state:        HdrState::Disabled,
                hdr_mode:         HdrMode::Default,
                scale:            1.0,
                native_resolution: Some(size),
                supported_modes:  Vec::new(),
            });

            staged_modes.insert(name_str, dev_mode);
        }
    }

    Ok((staged_modes, outputs))
}