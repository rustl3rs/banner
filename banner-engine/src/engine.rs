use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use banner_parser::ast::PipelineSpecification;
use tokio::sync::broadcast::{Receiver, Sender};

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

    fn get_pipeline_specification(&self) -> &Vec<PipelineSpecification>;

    /// Returns a value from state based on the key.
    async fn get_state_for_id(&self, key: &str) -> Option<String>;

    /// Sets a value in state based on the key.
    async fn set_state_for_id(
        &self,
        key: &str,
        value: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[derive(Debug)]
pub enum ExecutionResult {
    Success(Events),
    Failed(Events),
}

/// .
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub async fn start(
    engine: &Arc<dyn Engine + Send + Sync>,
    mut rx: Receiver<Event>,
    tx: Sender<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!(target: "task_log", "Initialising");
    engine.initialise().await?;

    loop {
        let event = rx.recv().await;
        // if we have fallen behind and the channel is full, we'll get an error.
        // continue on, because we can continue to process events.
        if event.is_err() {
            log::error!(target: "task_log", "error receiving event: {:?}", event);
            continue;
        }

        let event = event.unwrap();
        log::debug!(target: "task_log", "received event: {:?}", event);

        if event == Event::new_builder(crate::EventType::UserDefined).build() {
            log::debug!(target: "task_log", "received user defined event");
            engine.get_pipelines().into_iter().for_each(|p| {
                log::debug!(target: "task_log", "Number of event handlers: {}", p.event_handlers.len());
                p.event_handlers
                    .iter()
                    .for_each(|eh| log::debug!(target: "task_log", "event handler: {:?}", eh));
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
                log::debug!(target: "event_log", "handlers: {:?}", handlers);
                if handlers.is_empty() {
                    None
                } else {
                    Some(handlers)
                }
            })
            .flatten()
            .collect();
        // and execute them all.
        for eh in event_handlers {
            let e = engine.clone();
            let ev = event.clone();
            eh.execute(e, &tx, ev);
        }
    }
}
