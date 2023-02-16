use std::{error::Error, fs, path::PathBuf, process::exit};

use banner_parser::ast::validate_pipeline;
use clap::{Parser, Subcommand};
use tracing::debug;

/// The Banner CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    /// Runs the pipeline locally
    Local {
        file: PathBuf,
    },
    Remote {},
}

fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    match args.command {
        Commands::Local { file } => match execute_pipeline(file) {
            Ok(_) => exit(0),
            Err(_) => exit(1),
        },
        Commands::Remote {} => {
            println!("Hello World!")
        }
    }
}

fn execute_pipeline(filepath: PathBuf) -> Result<(), Box<dyn Error>> {
    let pipeline = fs::read_to_string(&filepath).expect("Should have been able to read the file");
    match validate_pipeline(pipeline) {
        Ok(ast) => ast.print(),
        Err(e) => {
            // debug!("Description:: {e}");
            let f = filepath.to_str().unwrap();
            eprintln!("Error parsing pipeline from file: {f}.\n\n{e}")
        }
    }

    Ok(())
}
