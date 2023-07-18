use std::{error::Error, fs, path::PathBuf, sync::Arc};

use banner_engine::{parse_file, start_engine, Engine, Event};
use clap::{Parser, Subcommand};
use local_engine::LocalEngine;
use log::{self, LevelFilter};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};
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
    Local { file: PathBuf },
    /// Not yet implemented
    Remote {},
    /// Validates the pipeline so it can be checked without trying to load it.
    #[command(verbatim_doc_comment, visible_alias = "vp")]
    ValidatePipeline { file: PathBuf },
    /// Loads a pipeline into the local engine and the prints the internal respresentation of it.
    /// This is useful for debugging pipelines.
    /// The output consists of
    ///  * Tasks; and their tags, inputs, outputs, and dependencies
    ///  * EventHandlers; identifying the events they listen for.
    #[command(verbatim_doc_comment, visible_aliases = ["pp", "print-pipeline"])]
    PipelineRepresentation { file: PathBuf },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting Banner CLI...");

    // install global collector configured based on RUST_LOG env var.
    // Set max_log_level to Trace
    init_logger(LevelFilter::Trace)?;

    // Set default level for unknown targets to Trace
    set_default_level(LevelFilter::Off);
    let log_level = match std::env::var("RUST_LOG") {
        Ok(level) => level.parse::<LevelFilter>()?,
        Err(_) => LevelFilter::Info,
    };
    set_level_for_target("task_log", log_level);
    set_level_for_target("event_log", log_level);

    log::info!(target: "task_log", "Log level set to: {log_level}");
    log::debug!(target: "task_log", "Creating channels");
    let (tx, rx) = mpsc::channel(100);
    let (ostx, osrx) = oneshot::channel::<bool>();

    tokio::select! {
        _ = execute_command(rx, tx.clone(), ostx) => {},
        _ = create_terminal_ui(tx.clone(), osrx) => {}
    };

    Ok(())
}

async fn execute_command(
    rx: Receiver<Event>,
    tx: Sender<Event>,
    ostx: oneshot::Sender<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    match args.command {
        Commands::Local { file } => match execute_pipeline(file, rx, tx, ostx).await {
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
        Commands::ValidatePipeline { file } => {
            let pipeline =
                fs::read_to_string(&file).expect("Should have been able to read the file");
            match parse_file(pipeline) {
                Ok(()) => {
                    println!("Pipeline validated successfully! 👍🏽 🎉 ✅");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error occurred validating pipeline: 😞\n{e}");
                    Err(e)
                }
            }
        }
        Commands::PipelineRepresentation { file } => {
            let mut engine = LocalEngine::new();
            engine.with_pipeline_from_file(file).await?;
            let pipeline = engine.get_pipelines()[0];
            println!("{:#?}", pipeline);
            Ok(())
        }
    }
}

async fn execute_pipeline(
    filepath: PathBuf,
    rx: Receiver<Event>,
    tx: Sender<Event>,
    ostx: oneshot::Sender<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Loading pipeline from file: {:?}", filepath);
    log::info!(target: "task_log", "Starting pipeline");
    let mut engine = LocalEngine::new();
    engine.with_pipeline_from_file(filepath).await?;
    let engine = Arc::new(engine);

    log::info!(target: "task_log", "Confirming requirements");
    println!("Confirming requirements...");
    engine.confirm_requirements().await?;

    // now we are ready to start messing with the terminal window.
    let _ = ostx.send(true);

    log::info!(target: "task_log", "Starting orchestrator");
    println!("Starting orchestrator...");
    start_engine(engine, rx, tx).await?;
    log::info!(target: "task_log", "Exiting orchestrator");

    Ok(())
}
