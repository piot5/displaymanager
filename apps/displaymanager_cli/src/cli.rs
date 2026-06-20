use clap::Parser;

#[derive(Parser)]
#[command(
    name = "displaymanager_cli",
    version = "1.0",
    about = "Unified Display/DDC Manager"
)]
pub struct Cli {
    /// Hardware inventory scan — optionally save to file
    #[arg(short = 's', long)]
    pub scan: Option<Option<String>>,

    /// Monitor name, target ID, GDI path, or device path
    #[arg(short = 'i', long)]
    pub id: Option<String>,

    /// Set off display
    #[arg(short = 'o', long)]
    pub off: bool,

    /// Display mode: ext | clone
    #[arg(short = 'm', long)]
    pub mode: Option<String>,

    /// Source for cloned (e.g. \\.\DISPLAY1)
    #[arg(short = 'c', long)]
    pub cloned: Option<String>,

    /// Resolution, e.g. 1920x1080
    #[arg(short = 'r', long)]
    pub res: Option<String>,

    /// Position, e.g. 0x0 or 100,200
    #[arg(short = 't', long)]
    pub topo: Option<String>,

    /// Refresh rate in mHz, e.g. 60000 for 60 Hz
    #[arg(short = 'f', long)]
    pub freq: Option<u32>,

    /// Rotation: 0, 90, 180, 270
    #[arg(short = 'R', long)]
    pub rotate: Option<String>,

    /// HDR: on / off
    #[arg(long)]
    pub hdr: Option<String>,

    /// Desktop scale factor, e.g. 1.0, 1.25, 1.5
    #[arg(long)]
    pub scale: Option<f64>,

    /// Mark this display as primary
    #[arg(short = 'p', long)]
    pub primary: bool,

    /// Read-only validation (dry-run)
    #[arg(long)]
    pub verify: bool,

    // ── DDC/CI controls ──
    /// DDC brightness (0..100)
    #[arg(long)]
    pub brightness: Option<u32>,

    /// DDC contrast (0..100)
    #[arg(long)]
    pub contrast: Option<u32>,

    /// DDC volume (0..100)
    #[arg(long)]
    pub volume: Option<u32>,

    /// DDC input source (dp1, dp2, hdmi1, hdmi2, or hex code e.g. 0x0F)
    #[arg(long)]
    pub input: Option<String>,

    /// DDC power state: on | off
    #[arg(long)]
    pub power: Option<String>,
}
