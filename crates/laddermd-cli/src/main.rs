use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use laddermd_core::{parser, renderer::MarkdownRenderer, validator};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "laddermd", about = "PLCopen XML to Markdown/DSL converter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert PLCopen XML to Markdown
    Convert {
        /// Input XML file
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Disable ASCII diagram output
        #[arg(long)]
        no_diagram: bool,
        /// Disable device table output
        #[arg(long)]
        no_table: bool,
        /// Disable logic expression output
        #[arg(long)]
        no_logic: bool,
    },
    /// Show project information
    Info {
        /// Input XML file
        input: PathBuf,
    },
    /// Validate roundtrip conversion
    Validate {
        /// Input XML file
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            no_diagram,
            no_table,
            no_logic,
        } => {
            let xml = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;
            let project = parser::parse(&xml)
                .with_context(|| format!("Failed to parse {}", input.display()))?;

            let renderer = MarkdownRenderer {
                render_diagram: !no_diagram,
                render_device_table: !no_table,
                render_logic: !no_logic,
            };

            let md = renderer.render(&project);

            if let Some(out_path) = output {
                fs::write(&out_path, &md)
                    .with_context(|| format!("Failed to write {}", out_path.display()))?;
                eprintln!("Written to {}", out_path.display());
            } else {
                print!("{md}");
            }
        }
        Commands::Info { input } => {
            let xml = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;
            let project = parser::parse(&xml)
                .with_context(|| format!("Failed to parse {}", input.display()))?;

            println!("Project: {}", project.name);
            for prog in &project.programs {
                println!("  Program: {}", prog.name);
                println!("    Rungs: {}", prog.rungs.len());

                let mut contacts = 0u32;
                let mut coils = 0u32;
                let mut blocks = 0u32;
                for rung in &prog.rungs {
                    for elem in &rung.elements {
                        match elem {
                            laddermd_core::model::RungElement::Contact(_) => contacts += 1,
                            laddermd_core::model::RungElement::Coil(_) => coils += 1,
                            laddermd_core::model::RungElement::Block(_) => blocks += 1,
                        }
                    }
                }
                println!("    Contacts: {contacts}, Coils: {coils}, Blocks: {blocks}");
            }
        }
        Commands::Validate { input } => {
            let xml = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;

            let result = validator::validate(&xml)
                .with_context(|| format!("Validation failed for {}", input.display()))?;

            println!(
                "Parse OK: {} rungs found",
                result.total_rungs
            );
            println!(
                "Devices: {} contacts, {} coils, {} blocks",
                result.contacts, result.coils, result.blocks
            );

            if result.roundtrip_ok {
                println!("Roundtrip OK: all rungs logically equivalent");
            } else {
                println!("Roundtrip FAILED:");
                for err in &result.errors {
                    println!("  - {err}");
                }
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
