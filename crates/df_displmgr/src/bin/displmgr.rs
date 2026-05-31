// demo.rs
use anyhow::{anyhow, Result};
use clap::{Parser, Args};
use df_displmgr::{
    DisplayRotation, 
    NativeTopology, 
    UniversalTopology, 
    types::{Extent2D, Point2D}
};
use std::io::{self, Write};

/// DisplayFlow Full Info Demo
#[derive(Parser, Debug)]
#[command(author, version, about = "DisplayFlow Full Info Demo")]
struct DemoArgs {
    #[arg(short = 's', long)]
    pub scan: bool,

    #[command(flatten)]
    pub output_config: OutputArgs,
}

#[derive(Args, Debug)]
struct OutputArgs {
    #[arg(long)]
    pub output: Option<String>,
    #[arg(long, requires = "output")]
    pub mode: Option<String>,
    #[arg(long, requires = "output")]
    pub pos: Option<String>,
    #[arg(long, requires = "output")]
    pub rotate: Option<String>,
    #[arg(long, requires = "output")]
    pub off: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = DemoArgs::parse();
    let mut topology = NativeTopology::acquire()?;

    if args.scan {
        for out in topology.get_outputs() {
            // DEBUG: Here you can see the exact IDs the system uses
            println!("DEBUG: Vorhandene DisplayId: {:?}", out.identity.id);
            println!("{:?}: {} - {:?}", out.identity.id, out.identity.monitor_name, out.geometry.size);
        }
    } else if let Some(id_str) = args.output_config.output {
        // Search for the monitor that matches the input string exactly
        let target_id = topology.get_outputs()
            .into_iter()
            .find(|o| o.identity.id.0 == id_str)
            .map(|o| o.identity.id)
            .ok_or_else(|| anyhow!("Monitor mit ID '{}' nicht in der Topologie gefunden.", id_str))?;

        // Editor scope encapsulation to satisfy the borrow checker
        {
            let mut editor = topology.edit_output(&target_id)?;
            
            if args.output_config.off {
                editor.set_enabled(false)?;
            } else {
                if let Some(mode_str) = args.output_config.mode {
                    let parts: Vec<&str> = mode_str.split('x').collect();
                    if parts.len() == 2 {
                        editor.set_resolution(Extent2D { 
                            width: parts[0].parse()?, 
                            height: parts[1].parse()? 
                        })?;
                    }
                }

                if let Some(pos_str) = args.output_config.pos {
                    // IMPORTANT: Ensure to split the string appropriately here
                    let parts: Vec<&str> = pos_str.split('x').collect();
                    if parts.len() == 2 {
                        editor.set_position(Point2D { 
                            x: parts[0].parse()?, 
                            y: parts[1].parse()? 
                        })?;
                    }
                }

                if let Some(rot_str) = args.output_config.rotate {
                    let rot = match rot_str.to_lowercase().as_str() {
                        "left" => DisplayRotation::Rotate90,
                        "inverted" => DisplayRotation::Rotate180,
                        "right" => DisplayRotation::Rotate270,
                        _ => DisplayRotation::Rotate0,
                    };
                    editor.set_rotation(rot)?;
                }
            }
        }

        print!("Validating configuration... ");
        io::stdout().flush()?;
        
        match topology.validate().await {
            Ok(_) => {
                println!("OK");
                topology.commit().await?;
                println!("Changes applied successfully.");
            },
            Err(e) => {
                println!("FAILED: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}