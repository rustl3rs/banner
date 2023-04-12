use std::{error::Error, sync::Arc};

use async_trait::async_trait;
// use banner_parser::{
//     grammar::{BannerParser, Rule},
//     FromPest, Parser,
// };

use log::debug;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{event_handler::EventHandler, Event, Events, Pipeline, TaskDefinition};

// This is the trait that needs to be implemented by all Engines.
// Haven't thought it thru yet, but might also want a way to
// communicate capability of an engine.  In this way, a pipeline
// that is designed to work on something like Kubernetes may or may not
// be capable of running on Azure Functions or AWS Lambda
#[async_trait]
pub trait Engine {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    // Does everything exist that I need to run.
    // If I'm a Kubernetes engine, am I actually running in k8s?
    async fn confirm_requirements(&self) -> Result<(), Box<dyn Error + Send + Sync>>;

    // Now's your chance to load state from wherever
    // it's stored.  The engine won't be considered ready until this returns
    // successfully; possibly with retries.
    async fn initialise(&self) -> Result<(), Box<dyn Error + Send + Sync>>;

    // Not sure about the return type at all.
    async fn execute(
        &self,
        task: &TaskDefinition,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>>;

    // found a need for this when writing the pipeline event_handling stuff.
    async fn execute_task_name_in_scope(
        &self,
        scope_name: &str,
        pipeline_name: &str,
        job_name: &str,
        task_name: &str,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>>;

    fn get_pipelines(&self) -> Vec<&Pipeline>;
}

#[derive(Debug)]
pub enum ExecutionResult {
    Success(Events),
    Failed(Events),
}

pub async fn start_engine(
    engine: Arc<dyn Engine + Send + Sync>,
    mut rx: Receiver<Event>,
    tx: Sender<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!(target: "task_log", "initialising");
    engine.initialise().await?;

    loop {
        let event = rx.recv().await;
        debug!(target: "task_log", "received event: {:?}", event);

        // artificial_pause().await;

        if let Some(event) = event {
            if event == Event::new(crate::EventType::UserDefined).build() {
                log::debug!(target: "task_log", "received user defined event");
                engine.get_pipelines().into_iter().for_each(|p| {
                    log::debug!(target: "task_log", "Number of event handlers: {}", p.event_handlers.len());
                    p.event_handlers
                        .iter()
                        .for_each(|eh| log::debug!(target: "task_log", "event handler: {:?}", eh))
                });
                engine.get_pipelines().into_iter().for_each(|p| {
                    p.tasks.iter().for_each(|t| {
                        log::debug!(target: "task_log", "task: {:?}", t);
                    });
                });
                continue;
            }

            // get all event handlers that are listening for this event.
            let pipelines = engine.get_pipelines();
            let event_handlers: Vec<EventHandler> = pipelines
                .into_iter()
                .filter_map(|pipeline| {
                    let handlers = pipeline.events_matching(&event);
                    if handlers.len() > 0 {
                        Some(handlers)
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();
            // and execute them all.
            for eh in event_handlers.into_iter() {
                let e = engine.clone();
                let tx = tx.clone();
                let ev = event.clone();
                eh.execute(e, tx, ev).await;
            }
        }
    }
}

#[inline]
async fn artificial_pause() {
    let pause = 10000;
    tokio::time::sleep(std::time::Duration::from_millis(pause)).await;
}
