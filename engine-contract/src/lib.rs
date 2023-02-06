use std::io::Error;

use chrono::{DateTime, Utc};

// I fully expect this entire contract to change the minute I start implementing something.

// This is the trait that needs to be implemented by all Engines.
// Haven't thought it thru yet, but might also want a way to
// communicate capability of an engine.  In this way, a pipeline
// that is designed to work on something like Kubernetes may or may not
// be capable of running on Azure Functions or AWS Lambda
pub trait Engine {
    // Does everything exist that I need to run.
    // If I'm a Kubernetes engine, am I actually running in k8s?
    fn confirm_requirements() -> Result<(), Error>;

    // Now's your chance to load state from wherever
    // it's stored.  The engine won't be considered ready until this returns
    // successfully; possibly with retries.
    fn initialise() -> Result<(), Error>;

    // Not sure about the return type at all.
    fn execute() -> ExecutionResult;
}

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
pub enum Event {
    Task(TaskEvent),
    Job(JobEvent),
    Pipeline(PipelineEvent),
}

pub struct TaskEvent {
    r#type: TaskEventType,
    time_emitted: DateTime<Utc>,
    metadata: Vec<TaskMetadata>,
}

pub struct JobEvent {
    r#type: JobEventType,
    time_emitted: DateTime<Utc>,
    metadata: Vec<JobMetadata>,
}

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
pub struct Metadata {
    key: String,
    value: String,
}

pub type TaskEventType = EventType;
pub type JobEventType = EventType;
pub type PipelineEventType = EventType;
// Well, in some part this is the list of possible states of a job/task in Concourse.
// Success: represents successful completion of the task/job/pipeline (called a stop)
// Failed: represents faulty completion
// Aborted: means the step was cut short by some kind of intervention via Banner
// Errored: means the step was cut short by some external means; might be best to merge Errored and Aborted with a descriminator.
// Metric: emit a metric for Banner to interpret and track
// Log: informational event with data.
// Notification: informational event that should result in a notification being sent to system users/operational staff.
// UserDefined: just what it says.
pub enum EventType {
    Success,
    Failed,
    Aborted,
    Errored,
    Metric,
    Log,
    Notification,
    UserDefined,
}
