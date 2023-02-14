use std::error::Error;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

// I fully expect this entire contract to change the minute I start implementing something.

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
}

#[derive(Debug)]
pub enum ExecutionResult {
    Success(Events),
    Failed(Events),
}

pub type Events = Vec<Event>;

// At any rate, Events need to be thrown around the system.
// When I started thinking about them, I did think of them
// as completely separate; possibly mutually exclusive things.
// As I typed this out, they all started having the same properties.
// So ¯\_(ツ)_/¯
#[derive(Debug)]
pub enum Event {
    Task(TaskEvent),
    Job(JobEvent),
    Pipeline(PipelineEvent),
}

#[derive(Debug)]
pub struct TaskEvent {
    r#type: TaskEventType,
    time_emitted: DateTime<Utc>,
    metadata: Vec<TaskMetadata>,
}

#[derive(Debug)]
pub struct JobEvent {
    r#type: JobEventType,
    time_emitted: DateTime<Utc>,
    metadata: Vec<JobMetadata>,
}

#[derive(Debug)]
pub struct PipelineEvent {
    r#type: PipelineEventType,
    time_emitted: DateTime<Utc>,
    metadata: Vec<PipelineMetadata>,
}

pub type TaskMetadata = Metadata;
pub type JobMetadata = Metadata;
pub type PipelineMetadata = Metadata;

// Metadata became a way of evolving the usage; without having to care
// too much about blowing up the implementations between versions.
// It might work.. It might not... Truth of the matter is, I don't know
// all the stuff I might want to pass around about an event yet.
#[derive(Debug, Clone)]
pub struct Metadata {
    key: String,
    value: String,
}

impl Metadata {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    pub fn value(&self) -> &str {
        self.value.as_ref()
    }
}

pub type TaskEventType = EventType;
pub type JobEventType = EventType;
pub type PipelineEventType = EventType;
// Well, in some part this is the list of possible states of a job/task in Concourse.
// External: an event triggered by an external system.
// System: something that might trigger a task for instance, set by the banner server.
// Success: represents successful completion of the task/job/pipeline (called a stop)
// Failed: represents faulty completion
// Aborted: means the step was cut short by some kind of intervention via Banner
// Errored: means the step was cut short by some external means; might be best to merge Errored and Aborted with a descriminator.
// Metric: emit a metric for Banner to interpret and track
// Log: informational event with data.
// Notification: informational event that should result in a notification being sent to system users/operational systems.
// UserDefined: just what it says.
#[derive(Debug)]
pub enum EventType {
    External,
    System,
    Success,
    Failed,
    Aborted,
    Errored,
    Metric,
    Log,
    Notification,
    UserDefined,
}

pub type Tag = Metadata;
pub struct TaskDefinition {
    tags: Vec<Tag>,
    image: Image,
    command: Vec<String>,
    inputs: Vec<TaskResource>,
    outputs: Vec<TaskResource>,
}

impl TaskDefinition {
    pub fn new(
        tags: Vec<Tag>,
        image: Image,
        command: Vec<String>,
        inputs: Vec<TaskResource>,
        outputs: Vec<TaskResource>,
    ) -> Self {
        Self {
            tags,
            image,
            command,
            inputs,
            outputs,
        }
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn tags(&self) -> &[Metadata] {
        self.tags.as_ref()
    }

    pub fn inputs(&self) -> &[TaskResource] {
        self.inputs.as_ref()
    }

    pub fn outputs(&self) -> &[TaskResource] {
        self.outputs.as_ref()
    }

    pub fn command(&self) -> &[String] {
        self.command.as_ref()
    }
}

pub type Uri = String;
pub struct Image {
    // TBD
    source: Uri,
    credentials: Option<ImageRepositoryCredentials>,
}

impl Image {
    pub fn new(source: Uri, credentials: Option<ImageRepositoryCredentials>) -> Self {
        Self {
            source,
            credentials,
        }
    }

    pub fn source(&self) -> &str {
        self.source.as_ref()
    }

    pub fn credentials(&self) -> Option<&ImageRepositoryCredentials> {
        self.credentials.as_ref()
    }
}

pub enum TaskResource {
    Semver(semver::Version),
    Counter(u128),
    Path(String),
    EnvVar(String),
    Secret(String),
}

pub enum ImageRepositoryCredentials {
    UserPass(String, String),
    DockerConfig(String),
}
