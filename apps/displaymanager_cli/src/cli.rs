use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Output format for display information.
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output.
    Text,
    /// JSON-formatted output.
    Json,
}

/// Unified Display/DDC Manager CLI
#[derive(Parser)]
#[command(
    name = "displaymanager",
    version = "1.0",
    about = "Unified Display/DDC Manager",
    subcommand_required = true,
    arg_required_else_help = true,
)]
pub struct Cli {
    /// Output format (global): text, json
    #[arg(short = 'F', long, value_enum, global = true)]
    pub format: Option<OutputFormat>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List connected displays
    List {
        /// Scan and save hardware inventory
        #[arg(short, long)]
        scan: bool,

        /// Optional file path for hardware map output (implies --scan)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show detailed monitor information
    Info {
        /// Monitor identifier (name, target ID, GDI path, or device path)
        #[arg(short = 'i', long)]
        id: String,

        /// Read from EDID file instead of live query
        #[arg(long)]
        edid_file: Option<PathBuf>,
    },

    /// Modify display topology (resolution, position, rotation)
    Set {
        /// Monitor identifier
        #[arg(short = 'i', long)]
        id: String,

        /// Display mode: ext | clone
        #[arg(short = 'm', long)]
        mode: Option<String>,

        /// Source for cloned (e.g. \\.\DISPLAY1)
        #[arg(short = 'c', long)]
        cloned: Option<String>,

        /// Resolution, e.g. 1920x1080
        #[arg(short = 'r', long)]
        res: Option<String>,

        /// Position, e.g. 0x0 or 100,200
        #[arg(short = 't', long)]
        topo: Option<String>,

        /// Refresh rate in mHz, e.g. 60000 for 60 Hz
        #[arg(short = 'f', long)]
        freq: Option<u32>,

        /// Rotation: 0, 90, 180, 270
        #[arg(short = 'R', long)]
        rotate: Option<String>,

        /// HDR: on / off
        #[arg(long)]
        hdr: Option<String>,

        /// Desktop scale factor, e.g. 1.0, 1.25, 1.5
        #[arg(long)]
        scale: Option<f64>,

        /// Mark this display as primary
        #[arg(short = 'p', long)]
        primary: bool,

        /// Dry-run: validate without applying changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Control DDC/CI monitor settings
    Ddc {
        /// Monitor identifier
        #[arg(short = 'i', long)]
        id: String,

        /// DDC brightness (0..100)
        #[arg(long)]
        brightness: Option<u32>,

        /// DDC contrast (0..100)
        #[arg(long)]
        contrast: Option<u32>,

        /// DDC volume (0..100)
        #[arg(long)]
        volume: Option<u32>,

        /// DDC input source (dp1, dp2, hdmi1, hdmi2, or hex code e.g. 0x0F)
        #[arg(long)]
        input: Option<String>,

        /// DDC power state: on | off
        #[arg(long)]
        power: Option<String>,

        /// Dry-run: validate without applying changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Activate an inactive display
    Activate {
        /// Monitor identifier
        #[arg(short = 'i', long)]
        id: String,

        /// Resolution
        #[arg(long)]
        resolution: Option<String>,

        /// Position
        #[arg(long)]
        position: Option<String>,

        /// Dry-run: validate without applying changes
        #[arg(long)]
        dry_run: bool,
    },
}