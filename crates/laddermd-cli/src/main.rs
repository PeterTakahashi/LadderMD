use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use laddermd_core::{parser, renderer::MarkdownRenderer, validator};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "laddermd", about = "PLC ladder diagram to Markdown converter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum InputFormat {
    /// PLCopen XML (auto-detected from .xml extension)
    Xml,
    /// Mitsubishi MELSEC mnemonic format (auto-detected from .mn extension)
    Mnemonic,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert PLC ladder diagram to Markdown
    Convert {
        /// Input file (.xml for PLCopen XML, .mn for Mitsubishi mnemonic)
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Input format (auto-detected from extension if omitted)
        #[arg(short, long)]
        format: Option<InputFormat>,
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
        /// Input file
        input: PathBuf,
        /// Input format (auto-detected from extension if omitted)
        #[arg(short, long)]
        format: Option<InputFormat>,
    },
    /// Validate roundtrip conversion (PLCopen XML only)
    Validate {
        /// Input XML file
        input: PathBuf,
    },
    /// Convert Mitsubishi mnemonic to PLCopen XML
    Mn2xml {
        /// Input mnemonic file
        input: PathBuf,
        /// Output XML file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Convert PLCopen XML to Mitsubishi mnemonic
    Xml2mn {
        /// Input XML file
        input: PathBuf,
        /// Output mnemonic file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn detect_format(path: &std::path::Path, explicit: Option<&InputFormat>) -> InputFormat {
    if let Some(fmt) = explicit {
        return fmt.clone();
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("mn") | Some("mnemonic") | Some("lst") => InputFormat::Mnemonic,
        _ => InputFormat::Xml,
    }
}

fn parse_input(
    path: &PathBuf,
    format: &InputFormat,
) -> Result<laddermd_core::model::Project> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    match format {
        InputFormat::Xml => parser::parse(&content)
            .with_context(|| format!("Failed to parse XML: {}", path.display())),
        InputFormat::Mnemonic => parser::parse_mnemonic(&content, None, None)
            .with_context(|| format!("Failed to parse mnemonic: {}", path.display())),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            format,
            no_diagram,
            no_table,
            no_logic,
        } => {
            let fmt = detect_format(&input, format.as_ref());
            let project = parse_input(&input, &fmt)?;

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
        Commands::Info { input, format } => {
            let fmt = detect_format(&input, format.as_ref());
            let project = parse_input(&input, &fmt)?;

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

            println!("Parse OK: {} rungs found", result.total_rungs);
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
        Commands::Mn2xml { input, output } => {
            let content = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;
            let project = parser::parse_mnemonic(&content, None, None)
                .with_context(|| format!("Failed to parse mnemonic: {}", input.display()))?;
            let xml = laddermd_core::writer::write(&project)
                .with_context(|| "Failed to write XML")?;

            if let Some(out_path) = output {
                fs::write(&out_path, &xml)
                    .with_context(|| format!("Failed to write {}", out_path.display()))?;
                eprintln!("Written to {}", out_path.display());
            } else {
                print!("{xml}");
            }
        }
        Commands::Xml2mn { input, output } => {
            let xml = fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;
            let project = parser::parse(&xml)
                .with_context(|| format!("Failed to parse XML: {}", input.display()))?;
            let mn = parser::write_mnemonic(&project);

            if let Some(out_path) = output {
                fs::write(&out_path, &mn)
                    .with_context(|| format!("Failed to write {}", out_path.display()))?;
                eprintln!("Written to {}", out_path.display());
            } else {
                print!("{mn}");
            }
        }
    }

    Ok(())
}
