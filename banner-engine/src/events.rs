use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::mpsc::Sender;

use crate::metadata::{JobMetadata, PipelineMetadata, TaskMetadata};

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

#[derive(Debug, Clone)]
pub struct TaskEvent {
    r#type: TaskEventType,
    time_emitted: i64,
    metadata: Vec<TaskMetadata>,
}

impl TaskEvent {
    pub fn new(r#type: TaskEventType) -> TaskEventBuilder {
        TaskEventBuilder {
            task_event: Self {
                r#type,
                time_emitted: 0,
                metadata: vec![],
            },
        }
    }

    pub fn r#type(&self) -> &TaskEventType {
        &self.r#type
    }

    pub fn time_emitted(&self) -> DateTime<Utc> {
        let dt = Utc.timestamp_millis_opt(self.time_emitted).unwrap();
        dt
    }

    pub fn metadata(&self) -> &[TaskMetadata] {
        self.metadata.as_ref()
    }
}

pub struct TaskEventBuilder {
    task_event: TaskEvent,
}

impl TaskEventBuilder {
    pub fn with_metadata(&mut self, metadata: TaskMetadata) -> &TaskEventBuilder {
        self.task_event.metadata.push(metadata);
        self
    }

    pub async fn send_from(&self, tx: &Sender<Event>) {
        let mut send_task = self.task_event.clone();
        send_task.time_emitted = Utc::now().timestamp();
        tx.send(Event::Task(send_task)).await.unwrap_or_default();
    }
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
