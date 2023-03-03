use std::{error::Error, path::PathBuf, sync::Arc};

use banner_engine::{start_engine, Engine, Event};
use clap::{Parser, Subcommand};
use local_engine::LocalEngine;
use log::{self, LevelFilter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tui_logger::{self, init_logger, set_default_level, set_level_for_target};
use ui::terminal::create_terminal_ui;

mod ui;

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
async fn main() -> Result<(), Box<dyn Error>> {
    // install global collector configured based on RUST_LOG env var.
    // Set max_log_level to Trace
    init_logger(LevelFilter::Trace)?;

    // Set default level for unknown targets to Trace
    set_default_level(LevelFilter::Off);
    set_level_for_target("task_log", LevelFilter::Debug);
    set_level_for_target("event_log", LevelFilter::Debug);

    log::debug!(target: "task_log", "Creating channels");
    let (tx, rx) = mpsc::channel(100);

    tokio::select! {
        _ = execute_command(rx, tx.clone()) => {},
        _ = create_terminal_ui(tx.clone()) => {}
    };

    Ok(())
}

async fn execute_command(
    rx: Receiver<Event>,
    tx: Sender<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    match args.command {
        Commands::Local { file } => match execute_pipeline(file, rx, tx).await {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("{}", e);
                Err(e)
            }
        },
        Commands::Remote {} => {
            log::info!("Hello World!");
            Ok(())
        }
    }
}

async fn execute_pipeline(
    filepath: PathBuf,
    rx: Receiver<Event>,
    tx: Sender<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!(target: "task_log", "Starting pipeline");
    let mut engine = LocalEngine::new();
    engine.with_pipeline_from_file(filepath)?;
    let engine = Arc::new(engine);

    log::info!(target: "task_log", "Confirming requirements");
    engine.confirm_requirements().await?;

    log::info!(target: "task_log", "Starting orchestrator");
    start_engine(engine, rx, tx).await?;
    log::info!(target: "task_log", "Exiting orchestrator");

    Ok(())
}
