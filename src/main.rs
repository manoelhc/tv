use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tv::{get_value, set_value, scan_files};

#[derive(Parser)]
#[command(name = "tv")]
#[command(about = "Terraform Version control - manage module versions in .tf files", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a value from a .tf file
    Get {
        /// Query path (e.g., module.name.source["ref"])
        query: String,
        /// Default value if not found
        #[arg(default_value = "")]
        default: String,
        /// Path to .tf file (defaults to current directory)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Set a value in a .tf file
    Set {
        /// Query path (e.g., module.name.source["ref"])
        query: String,
        /// Value to set
        value: String,
        /// Path to .tf file (defaults to current directory)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Scan for .tf files that match a query pattern
    Scan {
        /// Query pattern (e.g., module.*, terraform.required_providers.aws)
        query: String,
        /// Directory to scan (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get {
            query,
            default,
            file,
        } => {
            let result = get_value(&query, file.as_deref())?;
            println!("{}", result.unwrap_or(default));
        }
        Commands::Set { query, value, file } => {
            set_value(&query, &value, file.as_deref())?;
        }
        Commands::Scan { query, dir } => {
            let results = scan_files(&query, &dir)?;
            for (file, module_name) in results {
                println!("\"{}\": \"module.{}\"", file.display(), module_name);
            }
        }
    }

    Ok(())
}
