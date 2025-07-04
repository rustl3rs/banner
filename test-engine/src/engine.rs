use std::{error::Error, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use banner_engine::{
    start, Engine, Event, EventType, ExecutionResult, Metadata, Pipeline, PipelineSpecification,
    PragmasBuilder, SystemEventResult, SystemEventScope, SystemEventType, TaskDefinition, JOB_TAG,
    PIPELINE_TAG, TASK_TAG,
};
use from_pest::FromPest;
use local_engine::LocalEngine;
use pest::Parser;
use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::{
    ast::{ActionOrAssert, Outcome, Pragmas, Target},
    grammar::{Rule, TestPragmaParser},
};

/// A test engine that can be used to test pipelines.
#[allow(clippy::module_name_repetitions)]
pub struct TestEngine {
    local_engine: Arc<dyn Engine + Send + Sync>,
    tx: Option<Sender<Event>>,
    // test_pragmas: Pragmas,
}

impl TestEngine {
    /// Create a new [`TestEngine`] from a pipeline file.
    ///
    /// # Panics
    ///
    /// Panics if the file can't be found or loaded.
    pub async fn new(filepath: PathBuf) -> Self {
        let mut engine = LocalEngine::new();
        engine
            .with_pipeline_from_file(
                filepath.clone(),
                PragmasBuilder::new().register_context("test"),
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to load pipeline from file - {filepath:?}"));
        Self {
            local_engine: Arc::new(engine),
            tx: None,
        }
    }

    /// Starts this [`TestEngine`].
    ///
    /// # Panics
    ///
    /// Panics if the local engine fails to confirm requirements or initialise.
    pub async fn start(&mut self) {
        self.local_engine.confirm_requirements().await.unwrap();
        self.local_engine.initialise().await.unwrap();

        let (tx, rx) = broadcast::channel(100);

        self.tx = Some(tx.clone());
        let engine = self.local_engine.clone();
        tokio::spawn(async move { start(&engine, rx, tx).await });

        // for each pipeline:
        //      get the pragmas and execute each action/assert and verify the results.
        for pipeline in self.local_engine.get_pipelines() {
            let pragmas = &(pipeline.pragmas);
            println!("Executing pipeline: {}", pipeline.name);
            println!("Pipeline pragmas: {:?}", pragmas);

            // get the actions and execute each action/assert and verify the results.
            for test_pragma in pipeline
                .pragmas
                .iter()
                .filter(|pragma| pragma.context == "test")
            {
                println!("Executing test pragma: {}", test_pragma.context);
                let pragma_source = test_pragma.src.trim();
                println!("{pragma_source}");
                let mut parse_tree = TestPragmaParser::parse(Rule::pragma, &pragma_source).unwrap();
                let pragmas = match Pragmas::from_pest(&mut parse_tree) {
                    Ok(tree) => tree,
                    Err(e) => panic!("{e:?}"),
                };

                let mut events: Vec<Event> = Vec::new();

                for pragma in pragmas.pragma.into_iter() {
                    match pragma {
                        ActionOrAssert::Trigger(target) => {
                            match target {
                                Target::Pipeline(pipeline) => {
                                    let mut rx = self.tx.as_ref().unwrap().subscribe();

                                    println!("triggering pipeline; {pipeline}");
                                    banner_engine::Event::new_builder(EventType::System(
                                        SystemEventType::Trigger(SystemEventScope::Pipeline),
                                    ))
                                    .with_pipeline_name(&pipeline)
                                    .send_from(&(self.tx.as_ref().unwrap()));

                                    // wait for a pipeline success or finish event.
                                    events =
                                        capture_events_until(&mut rx, SystemEventScope::Pipeline)
                                            .await;
                                    println!("EVENTS: {events:?}");
                                }
                                Target::Job(pipeline, job) => {
                                    // TODO: Implement job trigger logic
                                    println!("triggering job; {pipeline}/{job}");
                                }
                                Target::Task(pipeline, job, task) => {
                                    // TODO: Implement task trigger logic
                                    println!("triggering task; {pipeline}/{job}/{task}");
                                }
                            }
                            println!("Complete!")
                        }
                        ActionOrAssert::Asserts(asserts) => {
                            let mut errors: Vec<String> = Vec::new();
                            for assert in asserts.into_iter() {
                                match assert {
                                    (target, outcome) => {
                                        let required_outcome = match outcome {
                                            Outcome::Success => SystemEventResult::Success,
                                            Outcome::Failure => SystemEventResult::Failed,
                                        };
                                        // Handle success outcome
                                        let matching_event =
                                            events.iter().find(|event| match event.get_type() {
                                                EventType::System(et) => {
                                                    match et {
                                                        SystemEventType::Done(SystemEventScope::Pipeline, outcome) => {
                                                            match &target {
                                                                // TODO: check the metadata to ensure the pipeline names match.
                                                                Target::Pipeline(pipeline_name) => {
                                                                    let pipeline_metadata = Metadata::new(PIPELINE_TAG,pipeline_name);
                                                                    if !matching_banner_metadata(&[pipeline_metadata], event.metadata()) {
                                                                        return false;
                                                                    }

                                                                    if outcome == required_outcome {
                                                                        true
                                                                    } else {
                                                                        println!("Matching event with incorrect result.... :( ");
                                                                        false
                                                                    }
                                                                },
                                                                // TODO: make sure to do the Job and Task matching arms.
                                                                _ => false,
                                                            }
                                                        },
                                                        SystemEventType::Done(SystemEventScope::Job, outcome) => {
                                                            match &target {
                                                                // TODO: check the metadata to ensure the pipeline names match.
                                                                Target::Job(pipeline_name, job_name) => {
                                                                    let pipeline_metadata = Metadata::new(PIPELINE_TAG,pipeline_name);
                                                                    let job_metadata = Metadata::new(JOB_TAG, job_name);
                                                                    if !matching_banner_metadata(&[pipeline_metadata], event.metadata())
                                                                        && !matching_banner_metadata(&[job_metadata.clone()], event.metadata()) {
                                                                        return false;
                                                                    }

                                                                    if outcome == required_outcome {
                                                                        true
                                                                    } else {
                                                                        println!("Matching event with incorrect result.... :( \n{outcome} +++ {job_metadata:?}");
                                                                        false
                                                                    }
                                                                },
                                                                _ => false,
                                                            }
                                                        },
                                                        SystemEventType::Done(SystemEventScope::Task, outcome) => {
                                                            match &target {
                                                                // TODO: check the metadata to ensure the pipeline names match.
                                                                Target::Task(pipeline_name, job_name, task_name) => {
                                                                    let pipeline_metadata = Metadata::new(PIPELINE_TAG,pipeline_name);
                                                                    let job_metadata = Metadata::new(JOB_TAG, job_name);
                                                                    let task_metadata = Metadata::new(TASK_TAG, task_name);
                                                                    if !matching_banner_metadata(&[pipeline_metadata], event.metadata())
                                                                        && !matching_banner_metadata(&[job_metadata], event.metadata())
                                                                        && !matching_banner_metadata(&[task_metadata], event.metadata()) {
                                                                        return false;
                                                                    }

                                                                    if outcome == required_outcome {
                                                                        true
                                                                    } else {
                                                                        println!("Matching event with incorrect result.... :( ");
                                                                        false
                                                                    }
                                                                },
                                                                _ => false,
                                                            }
                                                        },
                                                        _ => false,
                                                    }
                                                },
                                                _ => false,
                                            });
                                        let Some(event) = matching_event else {
                                            errors.push(format!(
                                                "No matching event found for {:?}",
                                                &target.clone()
                                            ));
                                            continue;
                                        };
                                        println!("Event found: {:?}", event);
                                    }
                                }
                            }
                            if !errors.is_empty() {
                                println!("Errors:");
                                for error in errors {
                                    println!("{}", error);
                                }
                                panic!("Errors occurred!");
                            }
                        }
                    }
                }
            }
        }
    }
}

// TODO: consolidate this.
pub(crate) fn matching_banner_metadata(lhs: &[Metadata], rhs: &[Metadata]) -> bool {
    lhs.iter().all(|tag| rhs.iter().any(|t| t == tag))
}

async fn capture_events_until(
    rx: &mut Receiver<Event>,
    system_event_scope: SystemEventScope,
) -> Vec<Event> {
    let mut events = Vec::new();

    loop {
        match rx.recv().await {
            Ok(event) => match event.get_type() {
                EventType::System(system_event_type) => match system_event_type {
                    SystemEventType::Done(ses, _) => {
                        events.push(event);
                        if ses == system_event_scope {
                            return events;
                        }
                    }
                    _ => {
                        events.push(event);
                    }
                },
                _ => {
                    events.push(event);
                }
            },
            Err(_) => {
                panic!("Failed to receive event");
            }
        }
    }
}

#[async_trait]
impl Engine for TestEngine {
    async fn confirm_requirements(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.local_engine.confirm_requirements().await
    }

    async fn initialise(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.local_engine.initialise().await
    }

    async fn execute(
        &self,
        task: &TaskDefinition,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
        self.local_engine.execute(task).await
    }

    async fn execute_task_name_in_scope(
        &self,
        scope_name: &str,
        pipeline_name: &str,
        job_name: &str,
        task_name: &str,
    ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
        self.local_engine
            .execute_task_name_in_scope(scope_name, pipeline_name, job_name, task_name)
            .await
    }

    fn get_pipelines(&self) -> Vec<&Pipeline> {
        self.local_engine.get_pipelines()
    }

    fn get_pipeline_specification(&self) -> &Vec<PipelineSpecification> {
        self.local_engine.get_pipeline_specification()
    }

    async fn get_state_for_id(&self, key: &str) -> Option<String> {
        self.local_engine.get_state_for_id(key).await
    }

    async fn set_state_for_id(
        &self,
        key: &str,
        value: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.local_engine.set_state_for_id(key, value).await
    }
}
