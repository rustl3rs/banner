use std::{error::Error, fs, path::PathBuf, process::exit};

use banner_engine::{Engine, TaskDefinition};
use banner_parser::parser::validate_pipeline;
use clap::{Parser, Subcommand};
use local_engine::LocalEngine;
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

#[tokio::main]
async fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    match args.command {
        Commands::Local { file } => match execute_pipeline(file).await {
            Ok(_) => exit(0),
            Err(e) => {
                println!("{}", e);
                exit(1)
            }
        },
        Commands::Remote {} => {
            println!("Hello World!")
        }
    }
}

async fn execute_pipeline(filepath: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    let pipeline = fs::read_to_string(&filepath).expect("Should have been able to read the file");
    match validate_pipeline(pipeline) {
        Ok(ast) => {
            let engine = LocalEngine::new();
            engine.initialise().await?;
            for task in ast.tasks {
                let task: TaskDefinition = task.into();
                let task_name = task
                    .tags()
                    .iter()
                    .find(|tag| tag.key() == "banner.io/task")
                    .map(|tag| tag.value())
                    .unwrap();
                debug!("Running Task: {task_name}");
                engine.execute(&task.into()).await?;
            }
            ()
        }
        Err(e) => {
            let f = filepath.to_str().unwrap();
            eprintln!("Error parsing pipeline from file: {f}.\n\n{e}")
        }
    }

    Ok(())
}
