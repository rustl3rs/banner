use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use banner_parser::{
    ast::{Pipeline, Task},
    grammar::{BannerParser, Rule},
    FromPest, Parser,
};
use log::trace;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    Event, Events, Image, SystemEventResult, SystemEventScope, SystemEventType, Tag,
    TaskDefinition, TaskEvent, TaskEventType,
};

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

    async fn get_pipelines(&self) -> Vec<String>;
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
    log::info!("initialising");
    engine.initialise().await?;

    loop {
        let event = rx.recv().await;
        match event {
            Some(event) => match event {
                Event::Task(event) => {
                    if let Some(task_name) = event.metadata().into_iter().find_map(|x| {
                        if x.key() == "banner.io/task"
                            && event.r#type()
                                == &TaskEventType::System(SystemEventType::Trigger(
                                    SystemEventScope::Task,
                                ))
                        {
                            Some(x.value().to_string())
                        } else {
                            None
                        }
                    }) {
                        let pipelines: Vec<Pipeline> = engine
                            .get_pipelines()
                            .await
                            .into_iter()
                            .map(|pipeline| pipeline_to_ast(&pipeline))
                            .collect();
                        let task = find_task(&pipelines, &task_name);
                        let task: TaskDefinition = task.into();

                        log::info!(target: "event_log", "Running Task: {task_name}");
                        let tx = tx.clone();
                        let e = engine.clone();
                        tokio::spawn(async move {
                            // send a start event
                            TaskEvent::new(TaskEventType::System(SystemEventType::Starting(
                                SystemEventScope::Task,
                            )))
                            .with_name(&task_name)
                            .send_from(&tx)
                            .await;

                            match e.execute(&task).await {
                                Ok(_) => {
                                    // send an end event
                                    TaskEvent::new(TaskEventType::System(SystemEventType::Done(
                                        SystemEventScope::Task,
                                        SystemEventResult::Success,
                                    )))
                                    .with_name(&task_name)
                                    .send_from(&tx)
                                    .await;
                                }
                                Err(e) => {
                                    log::error!(target: "task_log", "{e}");

                                    // send an end event
                                    TaskEvent::new(TaskEventType::System(SystemEventType::Done(
                                        SystemEventScope::Task,
                                        SystemEventResult::Failed,
                                    )))
                                    .with_name(&task_name)
                                    .send_from(&tx)
                                    .await;
                                }
                            }
                        });
                    } else {
                        continue;
                    };
                }
                Event::Job(_event) => todo!(),
                Event::Pipeline(_event) => todo!(),
            },
            None => (),
        }
    }
}

fn find_task(pipelines: &[Pipeline], task_name: &str) -> TaskDefinition {
    pipelines
        .iter()
        .find_map(|pipeline| {
            let tasks = pipeline.tasks.iter();
            tasks.into_iter().find_map(|task| {
                if task.name == task_name {
                    Some(task.into())
                } else {
                    None
                }
            })
        })
        .unwrap()
}

impl From<&Task> for TaskDefinition {
    fn from(task: &Task) -> Self {
        let tags = task
            .tags
            .iter()
            .map(|t| Tag::new(&t.key, &t.value))
            .collect();
        let image = Image::new(task.image.clone(), None);
        let mut command: Vec<String> = task
            .command
            .as_str()
            .split_whitespace()
            .map(|s| s.into())
            .collect();
        command.push(task.script.clone());
        let td = Self::new(tags, image, command, vec![], vec![]);
        td
    }
}
// This should be infallible.
// The pipelines that get passed in should already have undergone transformation and validation
fn pipeline_to_ast(code: &str) -> Pipeline {
    let mut parse_tree = BannerParser::parse(Rule::pipeline_definition, &code).unwrap();
    match Pipeline::from_pest(&mut parse_tree) {
        Ok(tree) => tree,
        Err(e) => {
            trace!("ERROR = {:#?}", e);
            panic!("{:?}", e);
        }
    }
}
