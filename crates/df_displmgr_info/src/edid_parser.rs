use crate::edid_types::{
    EdidData, MonitorMode, VideoInterfaceInfo, DigitalInterfaceType,
    ChromaticityCoordinates, HdrMetadata, AudioCapabilities
};
use crate::error::EdidError;

const EDID_HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
const BLOCK_SIZE: usize = 128;

pub struct EdidParser;

impl EdidParser {
    pub fn parse(raw: &[u8]) -> Result<EdidData, EdidError> {
        if raw.len() < BLOCK_SIZE {
            return Err(EdidError::ParseError);
        }

        // 1. Base block validation
        if raw[0..8] != EDID_HEADER || !Self::validate_checksum(raw, 0) {
            return Err(EdidError::ParseError);
        }

        // Manufacturer ID: 3 x 5-bit characters packed into 2 bytes
        let mfg_id_raw = ((raw[8] as u16) << 8) | raw[9] as u16;
        let char1 = ((mfg_id_raw >> 10) & 0x1F) as u8 + b'@';
        let char2 = ((mfg_id_raw >> 5) & 0x1F) as u8 + b'@';
        let char3 = (mfg_id_raw & 0x1F) as u8 + b'@';
        let manufacturer_id = format!("{}{}{}", char1 as char, char2 as char, char3 as char);

        let product_code = ((raw[11] as u16) << 8) | raw[10] as u16;
        let serial_number_binary = ((raw[15] as u32) << 24)
            | ((raw[14] as u32) << 16)
            | ((raw[13] as u32) << 8)
            | raw[12] as u32;

        let week_of_manufacture = raw[16];
        let year_of_manufacture = 1990 + raw[17] as i32;

        // Video interface detection (byte 20) per EDID 1.4 specification
        let video_input_byte = raw[20];
        let video_interface = if (video_input_byte & 0x80) != 0 {
            let bit_depth = match (video_input_byte >> 4) & 0x07 {
                1 => 6,
                2 => 8,
                3 => 10,
                4 => 12,
                5 => 14,
                6 => 16,
                _ => 8,
            };

            // FIX: The original fallback set Unknown -> HDMI whenever bit_depth > 0.
            // Since bit_depth defaults to 8 (never 0), this silently misclassified every
            // DisplayPort and other unrecognised interface as HDMI. Now Unknown stays Unknown.
            let interface_type = match video_input_byte & 0x0F {
                1 => DigitalInterfaceType::Dvi,
                2 => DigitalInterfaceType::Hdmi,
                3 => DigitalInterfaceType::DisplayPort,
                _ => DigitalInterfaceType::Unknown,
            };

            VideoInterfaceInfo::Digital { bit_depth, interface_type }
        } else {
            let signal_level = match (video_input_byte >> 5) & 0x03 {
                0 => 0.700,
                1 => 0.714,
                2 => 1.000,
                _ => 0.700,
            };
            let setup_expected = (video_input_byte & 0x10) != 0;
            VideoInterfaceInfo::Analog { signal_level_v: signal_level, setup_expected }
        };

        let chromaticity = Some(Self::parse_chromaticity(&raw[25..35]));

        // Parse the 4 descriptor blocks from the base block
        let mut modes = Vec::new();
        let mut serial_number_ascii = None;
        let mut model_name = String::new();

        for i in 0..4 {
            let offset = 54 + (i * 18);
            if offset + 18 > raw.len() {
                break;
            }
            let descriptor = &raw[offset..offset + 18];

            if descriptor[0] == 0 && descriptor[1] == 0 && descriptor[2] == 0 {
                // Monitor descriptor block
                match descriptor[3] {
                    0xFF => {
                        // Alphanumeric serial number string
                        let sn = String::from_utf8_lossy(&descriptor[5..18]);
                        serial_number_ascii = Some(sn.trim().to_string());
                    }
                    0xFC => {
                        // Monitor name string
                        let name = String::from_utf8_lossy(&descriptor[5..18]);
                        model_name = name.trim().to_string();
                    }
                    _ => {}
                }
            } else {
                // Detailed Timing Descriptor (DTD)
                let pixel_clock_10khz = ((descriptor[1] as u32) << 8) | descriptor[0] as u32;
                if pixel_clock_10khz == 0 {
                    continue;
                }

                let h_active = (((descriptor[4] as u32) & 0xF0) << 4) | descriptor[2] as u32;
                let h_blanking = (((descriptor[4] as u32) & 0x0F) << 8) | descriptor[3] as u32;
                let v_active = (((descriptor[7] as u32) & 0xF0) << 4) | descriptor[5] as u32;
                let v_blanking = (((descriptor[7] as u32) & 0x0F) << 8) | descriptor[6] as u32;
                let interlaced = (descriptor[17] & 0x80) != 0;

                let h_total = h_active + h_blanking;
                let v_total = v_active + v_blanking;

                // FIX: Previously hardcoded to 60 Hz for every mode, breaking 120/144/240 Hz
                // monitors entirely. Refresh rate is derived from the DTD pixel clock:
                //   pixel_clock field is in units of 10 kHz → multiply by 10_000 for Hz
                //   refresh_rate = pixel_clock_hz / (h_total * v_total)
                // For interlaced modes v_total counts half-frames, so we halve it.
                let refresh_rate = if h_total > 0 && v_total > 0 {
                    let pixel_clock_hz = pixel_clock_10khz * 10_000;
                    let v_total_effective = if interlaced { v_total * 2 } else { v_total };
                    (pixel_clock_hz / (h_total * v_total_effective)).max(1)
                } else {
                    60 // Fallback only when timing data is corrupt
                };

                if h_active > 0 && v_active > 0 {
                    modes.push(MonitorMode {
                        width: h_active,
                        height: v_active,
                        refresh_rate,
                        interlaced,
                    });
                }
            }
        }

        if model_name.is_empty() {
            model_name = "Generic Monitor".into();
        }

        let extension_blocks = raw[126];
        let mut hdr_caps = HdrMetadata::default();
        let mut audio_caps = AudioCapabilities::default();

        // 2. CEA-861 extension block parsing (HDR / audio pipeline)
        if extension_blocks > 0 && raw.len() >= 2 * BLOCK_SIZE {
            let ext_offset = BLOCK_SIZE;

            if Self::validate_checksum(raw, ext_offset) && raw[ext_offset] == 0x02 {
                // Valid CEA-861 extension block
                let d_offset = raw[ext_offset + 2] as usize;

                // FIX: d_offset < 4 means there are no data blocks between the CEA header
                // and the first detailed timing descriptor — skip block parsing entirely
                // rather than producing an underflowing range and silently dropping data.
                if d_offset >= 4 {
                    let mut curr = ext_offset + 4;
                    let end = ext_offset + d_offset;

                    while curr < end && curr < raw.len() {
                        let header = raw[curr];
                        let tag = (header >> 5) & 0x07;
                        let length = (header & 0x1F) as usize;

                        if curr + 1 + length > end {
                            break;
                        }
                        let block_data = &raw[curr + 1..curr + 1 + length];

                        match tag {
                            1 => {
                                // Audio Data Block (Short Audio Descriptors)
                                audio_caps.extra_audio_descriptors_count += length / 3;
                                for chunk in block_data.chunks_exact(3) {
                                    let format_code = (chunk[0] >> 3) & 0x0F;
                                    let channels = (chunk[0] & 0x07) + 1;
                                    let codec_str = match format_code {
                                        1 => format!("Linear PCM (channels: {channels})"),
                                        2 => format!("AC-3 / Dolby Digital (channels: {channels})"),
                                        7 => format!("DTS (channels: {channels})"),
                                        10 => format!("DD+ / Dolby Digital Plus (channels: {channels})"),
                                        12 => format!("Dolby TrueHD (channels: {channels})"),
                                        _ => format!("SAD Codec {format_code} (channels: {channels})"),
                                    };
                                    audio_caps.short_audio_descriptors.push(codec_str);
                                }
                            }
                            7 => {
                                // CEA Extended Tag blocks (e.g. HDR Static Metadata)
                                if !block_data.is_empty() && block_data[0] == 0x06 {
                                    // Extended Tag 6: HDR Static Metadata Block
                                    if block_data.len() >= 3 {
                                        let eotf_byte = block_data[1];
                                        hdr_caps.supports_sdr_eotf = (eotf_byte & 0x01) != 0;
                                        hdr_caps.supports_hdr_traditional = (eotf_byte & 0x02) != 0;
                                        hdr_caps.supports_smpte_st2084 = (eotf_byte & 0x04) != 0; // HDR10
                                        hdr_caps.supports_hlg = (eotf_byte & 0x08) != 0;          // Hybrid Log-Gamma
                                    }
                                    if block_data.len() >= 4 {
                                        let max_lum = block_data[3];
                                        if max_lum > 0 {
                                            // Formula per VESA spec: 100 * 2^(max_lum / 32)
                                            hdr_caps.max_luminance_cd_m2 =
                                                Some(100.0 * f32::powf(2.0, max_lum as f32 / 32.0));
                                        }
                                    }
                                    if block_data.len() >= 5 {
                                        let max_fa = block_data[4];
                                        if max_fa > 0 {
                                            hdr_caps.max_frame_average_luminance_cd_m2 =
                                                Some(100.0 * f32::powf(2.0, max_fa as f32 / 32.0));
                                        }
                                    }
                                    if block_data.len() >= 6 {
                                        let min_lum = block_data[5];
                                        if min_lum > 0 {
                                            hdr_caps.min_luminance_cd_m2 = Some(
                                                hdr_caps.max_luminance_cd_m2.unwrap_or(400.0)
                                                    * f32::powf(min_lum as f32 / 255.0, 2.0)
                                                    / 100.0,
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        curr += 1 + length;
                    }
                }
            }
        }

        Ok(EdidData {
            model_name,
            manufacturer_id,
            product_code,
            serial_number_binary,
            serial_number_ascii,
            week_of_manufacture,
            year_of_manufacture,
            video_interface,
            chromaticity,
            extension_blocks,
            modes,
            hdr_caps,
            audio_caps,
        })
    }

    fn parse_chromaticity(b: &[u8]) -> ChromaticityCoordinates {
        let red_x_lsb   = (b[0] >> 6) & 0x03;
        let red_y_lsb   = (b[0] >> 4) & 0x03;
        let green_x_lsb = (b[0] >> 2) & 0x03;
        let green_y_lsb = b[0] & 0x03;

        let blue_x_lsb  = (b[1] >> 6) & 0x03;
        let blue_y_lsb  = (b[1] >> 4) & 0x03;
        let white_x_lsb = (b[1] >> 2) & 0x03;
        let white_y_lsb = b[1] & 0x03;

        let rx = (((b[2] as u16) << 2) | red_x_lsb as u16) as f32 / 1024.0;
        let ry = (((b[3] as u16) << 2) | red_y_lsb as u16) as f32 / 1024.0;
        let gx = (((b[4] as u16) << 2) | green_x_lsb as u16) as f32 / 1024.0;
        let gy = (((b[5] as u16) << 2) | green_y_lsb as u16) as f32 / 1024.0;
        let bx = (((b[6] as u16) << 2) | blue_x_lsb as u16) as f32 / 1024.0;
        let by_ = (((b[7] as u16) << 2) | blue_y_lsb as u16) as f32 / 1024.0;
        let wx = (((b[8] as u16) << 2) | white_x_lsb as u16) as f32 / 1024.0;
        let wy = (((b[9] as u16) << 2) | white_y_lsb as u16) as f32 / 1024.0;

        ChromaticityCoordinates {
            red_x: rx, red_y: ry,
            green_x: gx, green_y: gy,
            blue_x: bx, blue_y: by_,
            white_x: wx, white_y: wy,
        }
    }

    fn validate_checksum(raw: &[u8], offset: usize) -> bool {
        if offset + BLOCK_SIZE <= raw.len() {
            raw[offset..offset + BLOCK_SIZE]
                .iter()
                .fold(0u8, |acc, &x| acc.wrapping_add(x))
                == 0
        } else {
            false
        }
    }
}