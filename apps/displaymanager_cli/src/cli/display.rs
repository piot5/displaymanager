use clap::{Args, Subcommand};

// ──────────────────────────────────────────────
// Display: Main subcommand dispatcher
// ──────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DisplaySubcommand {
    /// Scan all monitors: topology + EDID + DDC telemetry (also writes edid_dump.txt)
    Scan(ScanArgs),
    /// Show detailed info for a single monitor
    Info(InfoArgs),
    /// Apply topology: position, resolution, rotation, HDR, scale, primary
    ///
    /// Inactive monitors are automatically activated via CCD wake.
    Set(SetArgs),
}

// ─────────────── Scan ───────────────

#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Output as JSON on stdout
    #[arg(short, long)]
    pub json: bool,
    /// Write full monitor data as JSON to file
    #[arg(long = "edid-json")]
    pub edid_json: Option<String>,
}

// ─────────────── Info ───────────────

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Monitor name, target ID, GDI path, or device path
    #[arg(short = 'o', long = "output")]
    pub output: String,
    /// Output as JSON on stdout
    #[arg(short, long)]
    pub json: bool,
}

// ─────────────── Set ───────────────

#[derive(Args, Debug)]
pub struct SetArgs {
    /// Monitor name, target ID, GDI path, or device path
    #[arg(short = 'o', long = "output")]
    pub output: String,

    /// Display mode: extended | cloned | off
    #[arg(long = "mode-type")]
    pub mode_type: Option<String>,

    /// For mode-type=cloned: source display to clone from (e.g. \\.\DISPLAY1)
    #[arg(long = "clone-from")]
    pub clone_from: Option<String>,

    /// Resolution, e.g. 1920x1080
    #[arg(long)]
    pub mode: Option<String>,

    /// Position, e.g. 0x0 or 100,200
    #[arg(long)]
    pub pos: Option<String>,

    /// Rotation: 0, 90, 180, or 270
    #[arg(long)]
    pub rotate: Option<String>,

    /// Auto-position right of the rightmost active monitor
    #[arg(long = "auto-pos")]
    pub auto_pos: bool,

    /// (Windows) CCD wake before editing
    #[arg(long = "ccd-wake")]
    pub ccd_wake: bool,

    /// Refresh rate in mHz, e.g. 60000 for 60 Hz
    #[arg(long = "refresh-rate")]
    pub refresh_rate: Option<u32>,

    /// Enable/disable HDR: "on" or "off"
    #[arg(long)]
    pub hdr: Option<String>,

    /// Desktop scale factor, e.g. 1.0, 1.25, 1.5
    #[arg(long)]
    pub scale: Option<f64>,

    /// Mark this display as primary
    #[arg(long)]
    pub primary: bool,

    /// Dry-run: validate without changing anything
    #[arg(long = "verify-only")]
    pub verify_only: bool,
}
