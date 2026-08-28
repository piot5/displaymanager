//! GTF / CVT-RB video timing calculations.
//!
//! This module provides pure calculation helpers for deriving
//! modeline parameters (pixel clock, front/back porch, sync width)
//! from a target resolution and refresh rate.
//!
//! References:
//! - VESA GTF Standard
//! - VESA CVT Standard (Revision 1.2)
//! - Linux kernel: drivers/gpu/drm/drm_modes.c (drm_cvt_mode)

use serde::{Deserialize, Serialize};

/// Derived timing values for a given mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimingParameters {
    /// Total horizontal pixels (including blanking).
    pub h_total: u32,
    /// Total vertical lines (including blanking).
    pub v_total: u32,
    /// Pixel clock in kilohertz (10 kHz used by EDID conventions).
    pub pixel_clock_10khz: u32,
    /// Horizontal front porch in pixels.
    pub h_front_porch: u32,
    /// Horizontal sync width in pixels.
    pub h_sync_width: u32,
    /// Horizontal back porch in pixels.
    pub h_back_porch: u32,
    /// Vertical front porch in lines.
    pub v_front_porch: u32,
    /// Vertical sync width in lines.
    pub v_sync_width: u32,
    /// Vertical back porch in lines.
    pub v_back_porch: u32,
    /// Characteristic refresh rate (mHz).
    pub refresh_rate_mhz: u32,
}

/// Characteristic margins (as a fraction of total active pixels) for CVT.
pub struct MarginSettings {
    pub h_margin_percent: u32,
    pub v_margin_percent: u32,
    pub reduced_blanking: bool,
}

impl Default for MarginSettings {
    fn default() -> Self {
        MarginSettings {
            h_margin_percent: 0,
            v_margin_percent: 0,
            reduced_blanking: true,
        }
    }
}

/// Calculates a CVT timing for the given mode.
///
/// Based on the Linux kernel implementation (`drm_cvt_mode` in `drivers/gpu/drm/drm_modes.c`).
pub fn calculate_cvt(
    width: u32,
    height: u32,
    refresh_rate_mhz: u32,
    margins: MarginSettings,
) -> TimingParameters {
    let hdisplay = width;
    let vdisplay = height;
    let vrefresh = (refresh_rate_mhz / 1000) as i32;

    if hdisplay == 0 || vdisplay == 0 {
        return TimingParameters {
            h_total: width,
            v_total: height,
            pixel_clock_10khz: 0,
            h_front_porch: 0,
            h_sync_width: 0,
            h_back_porch: 0,
            v_front_porch: 0,
            v_sync_width: 0,
            v_back_porch: 0,
            refresh_rate_mhz,
        };
    }

    let vfieldrate = vrefresh; // interlaced not supported
    let interlace = 0;

    // Horizontal pixels rounded to granularity
    let hdisplay_rnd = (hdisplay - (hdisplay % 8)) as i32;

    // Margins
    let hmargin = if margins.h_margin_percent > 0 {
        let m = hdisplay_rnd * margins.h_margin_percent as i32 / 1000;
        m - (m % 8)
    } else {
        0
    };

    let hdisplay_total = hdisplay_rnd + 2 * hmargin;

    let vdisplay_rnd = vdisplay as i32;
    let vmargin = if margins.v_margin_percent > 0 {
        vdisplay_rnd * margins.v_margin_percent as i32 / 1000
    } else {
        0
    };

    let vdisplay_total = vdisplay_rnd + 2 * vmargin;

    // Determine VSync width from aspect ratio
    let vsync = if vdisplay % 3 == 0 && (vdisplay / 3) * 4 == hdisplay {
        4
    } else if vdisplay % 9 == 0 && (vdisplay / 9) * 16 == hdisplay {
        5
    } else if vdisplay % 10 == 0 && (vdisplay / 10) * 16 == hdisplay {
        6
    } else if (vdisplay % 4 == 0 && (vdisplay / 4) * 5 == hdisplay)
        || (vdisplay % 9 == 0 && (vdisplay / 9) * 15 == hdisplay)
    {
        7
    } else {
        10
    };

    let htotal: i32;
    let vtotal: i32;
    let mut hsync_start: i32;
    let hsync_end: i32;
    let vsync_start: i32;
    let vsync_end: i32;
    let hperiod: i32;

    if !margins.reduced_blanking {
        // Standard CVT (GTF-based)
        let cv_t_min_vsync_bp = 550;
        let cv_t_h_sync_percentage = 8;
        let cv_t_min_v_porch = 3;
        let cv_t_m_factor = 600;
        let cv_t_c_factor = 40;
        let cv_t_k_factor = 128;
        let cv_t_j_factor = 20;
        let cv_t_m_prime = cv_t_m_factor * cv_t_k_factor / 256;
        let cv_t_c_prime = (cv_t_c_factor - cv_t_j_factor) * cv_t_k_factor / 256 + cv_t_j_factor;

        // Estimate horizontal period
        let tmp1 = 1000 * 1000000 - cv_t_min_vsync_bp * 1000 * vfieldrate;
        let tmp2 = (vdisplay_rnd + 2 * vmargin + cv_t_min_v_porch) * 2 + interlace;
        hperiod = tmp1 * 2 / (tmp2 * vfieldrate);

        let tmp1 = cv_t_min_vsync_bp * 1000 / hperiod + 1;
        let vsyncandback_porch = if tmp1 < (vsync + cv_t_min_v_porch) {
            vsync + cv_t_min_v_porch
        } else {
            tmp1
        };
        vtotal = vdisplay_rnd + 2 * vmargin + vsyncandback_porch + cv_t_min_v_porch;

        let mut hblank_percentage = cv_t_c_prime * 1000 - cv_t_m_prime * hperiod / 1000;
        if hblank_percentage < 20 * 1000 {
            hblank_percentage = 20 * 1000;
        }
        let mut hblank = hdisplay_total * hblank_percentage / (100 * 1000 - hblank_percentage);
        hblank -= hblank % (2 * 8);

        htotal = hdisplay_total + hblank;
        hsync_end = hdisplay_total + hblank / 2;
        hsync_start = hsync_end - (htotal * cv_t_h_sync_percentage) / 100;
        hsync_start += 8 - (hsync_start % 8);

        vsync_start = vdisplay_total + cv_t_min_v_porch;
        vsync_end = vsync_start + vsync;
    } else {
        // Reduced blanking (CVT-RB)
        let cv_t_rb_min_vblank = 460;
        let cv_t_rb_h_sync = 32;
        let cv_t_rb_h_blank = 160;
        let cv_t_rb_vfporch = 3;
        let cv_t_min_v_bporch = 6;

        let tmp1 = 1000 * 1000000 - cv_t_rb_min_vblank * 1000 * vfieldrate;
        let tmp2 = vdisplay_rnd + 2 * vmargin;
        hperiod = tmp1 / (tmp2 * vfieldrate);

        let mut vbilines = cv_t_rb_min_vblank * 1000 / hperiod + 1;
        if vbilines < (cv_t_rb_vfporch + vsync + cv_t_min_v_bporch) {
            vbilines = cv_t_rb_vfporch + vsync + cv_t_min_v_bporch;
        }

        vtotal = vdisplay_rnd + 2 * vmargin + vbilines;
        htotal = hdisplay_total + cv_t_rb_h_blank;
        hsync_end = hdisplay_total + cv_t_rb_h_blank / 2;
        hsync_start = hsync_end - cv_t_rb_h_sync;
        vsync_start = vdisplay_total + cv_t_rb_vfporch;
        vsync_end = vsync_start + vsync;
    }

    let h_front_porch = (hsync_start - hdisplay_total) as u32;
    let h_sync_width = (hsync_end - hsync_start) as u32;
    let h_back_porch = (htotal - hsync_end) as u32;
    let v_front_porch = (vsync_start - vdisplay_total) as u32;
    let v_sync_width = (vsync_end - vsync_start) as u32;
    let v_back_porch = (vtotal - vsync_end) as u32;

    // Pixel clock from kernel: clock = htotal * HV_FACTOR * 1000 / hperiod
    // HV_FACTOR = 1000, result is in kHz; divide by 10 to get 10 kHz units.
    let mut clock_khz = (htotal as u64 * 1000 * 1000) / hperiod as u64;
    clock_khz -= clock_khz % 250;
    let pixel_clock_10khz = (clock_khz / 10) as u32;

    TimingParameters {
        h_total: htotal as u32,
        v_total: vtotal as u32,
        pixel_clock_10khz,
        h_front_porch,
        h_sync_width,
        h_back_porch,
        v_front_porch,
        v_sync_width,
        v_back_porch,
        refresh_rate_mhz,
    }
}


/// Convenience helpers for common presets.
pub fn cvt_rb(
    width: u32,
    height: u32,
    refresh_rate_mhz: u32,
) -> TimingParameters {
    calculate_cvt(
        width,
        height,
        refresh_rate_mhz,
        MarginSettings {
            h_margin_percent: 0,
            v_margin_percent: 0,
            reduced_blanking: true,
        },
    )
}

pub fn gtf(
    width: u32,
    height: u32,
    refresh_rate_mhz: u32,
) -> TimingParameters {
    calculate_cvt(
        width,
        height,
        refresh_rate_mhz,
        MarginSettings {
            h_margin_percent: 0,
            v_margin_percent: 0,
            reduced_blanking: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvt_rb_1080_60() {
        let params = cvt_rb(1920, 1080, 60_000);
        assert_eq!(params.h_total, 2080);
        assert_eq!(params.v_total, 1111);
        assert_eq!(params.h_front_porch, 48);
        assert_eq!(params.h_sync_width, 32);
        assert_eq!(params.h_back_porch, 80);
        assert_eq!(params.v_front_porch, 3);
        assert_eq!(params.v_sync_width, 5);
        assert_eq!(params.v_back_porch, 23);
        assert_eq!(params.pixel_clock_10khz, 13850);
    }

    #[test]
    fn test_gtf_1080_60() {
        let params = gtf(1920, 1080, 60_000);
        assert_eq!(params.h_total, 2576);
        assert_eq!(params.v_total, 1120);
        assert_eq!(params.h_front_porch, 128);
        assert_eq!(params.h_sync_width, 200);
        assert_eq!(params.h_back_porch, 328);
        assert_eq!(params.v_front_porch, 3);
        assert_eq!(params.v_sync_width, 5);
        assert_eq!(params.v_back_porch, 32);
        assert_eq!(params.pixel_clock_10khz, 17300);
    }

    #[test]
    fn test_cvt_rb_1440_144() {
        let params = cvt_rb(2560, 1440, 144_000);
        assert_eq!(params.h_total, 2720);
        assert_eq!(params.v_total, 1543);
        assert_eq!(params.h_front_porch, 48);
        assert_eq!(params.h_sync_width, 32);
        assert_eq!(params.h_back_porch, 80);
        assert_eq!(params.v_front_porch, 3);
        assert_eq!(params.v_sync_width, 5);
        assert_eq!(params.v_back_porch, 95);
        assert_eq!(params.pixel_clock_10khz, 60400);
    }
}
