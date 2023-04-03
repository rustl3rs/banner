use std::fmt::Display;

use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::mpsc::Sender;

use crate::{metadata::Metadata, PIPELINE_TAG};

pub type Events = Vec<Event>;

#[derive(Debug, Clone)]
pub struct Event {
    r#type: EventType,
    time_emitted: i64,
    metadata: Vec<Metadata>,
}

impl Event {
    pub fn new(r#type: EventType) -> EventBuilder {
        EventBuilder {
            event: Self {
                r#type,
                time_emitted: 0,
                metadata: vec![],
            },
        }
    }

    pub fn r#type(&self) -> &EventType {
        &self.r#type
    }

    pub fn time_emitted(&self) -> DateTime<Utc> {
        let dt = Utc.timestamp_millis_opt(self.time_emitted).unwrap();
        dt
    }

    pub fn metadata(&self) -> &[Metadata] {
        self.metadata.as_ref()
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | {:?} | [", self.time_emitted, self.r#type())?;
        let first = true;
        for md in &self.metadata {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{{{}: {}}}", md.key(), md.value())?;
        }
        writeln!(f, "]")?;
        Ok(())
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        match (self.r#type, other.r#type) {
            (EventType::System(sel), EventType::System(ser)) => match (sel, ser) {
                (SystemEventType::Trigger(sesl), SystemEventType::Trigger(sesr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task) => true,
                        (_, _) => false,
                    }
                }
                (SystemEventType::Starting(sesl), SystemEventType::Starting(sesr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task)
                        | (SystemEventScope::EventHandler, SystemEventScope::EventHandler) => true,
                        (_, _) => false,
                    }
                }
                (SystemEventType::Done(sesl, serl), SystemEventType::Done(sesr, serr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task)
                        | (SystemEventScope::EventHandler, SystemEventScope::EventHandler) => {
                            match (serl, serr) {
                                (SystemEventResult::Success, SystemEventResult::Success)
                                | (SystemEventResult::Failed, SystemEventResult::Failed)
                                | (SystemEventResult::Aborted, SystemEventResult::Aborted)
                                | (SystemEventResult::Errored, SystemEventResult::Errored) => true,
                                (_, _) => false,
                            }
                        }
                        (_, _) => false,
                    }
                }
                (_, _) => false,
            },
            (EventType::External, EventType::External)
            | (EventType::Metric, EventType::Metric)
            | (EventType::Log, EventType::Log)
            | (EventType::Notification, EventType::Notification) => true,
            (EventType::UserDefined, EventType::UserDefined) => todo!(),
            (_, _) => false,
        }
    }
}

// metadata from the left is checked for existence in the right. If all are present; TRUE; otherwise; FALSE
pub(crate) fn matching_banner_metadata(lhs: &[Metadata], rhs: &[Metadata]) -> bool {
    lhs.iter().all(|tag| rhs.iter().any(|t| t == tag))
}

fn get_pipeline_name(e: &Event) -> Option<&str> {
    e.metadata.iter().find_map(|t| {
        if t.key() == PIPELINE_TAG {
            Some(t.value())
        } else {
            None
        }
    })
}

pub struct EventBuilder {
    event: Event,
}

// TODO: Make impossible states impossible; where possible.
//   eg: log messages should only be attachable to Log event types.
impl EventBuilder {
    pub fn with_pipeline_name(mut self, pipeline_name: &str) -> EventBuilder {
        let metadata = Metadata::new_banner_pipeline(pipeline_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_job_name(mut self, job_name: &str) -> EventBuilder {
        let metadata = Metadata::new_banner_job(job_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_task_name(mut self, task_name: &str) -> EventBuilder {
        let metadata = Metadata::new_banner_task(task_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> EventBuilder {
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_event(mut self, event: &Event) -> EventBuilder {
        let metadata = Metadata::new_banner_event(event);
        self.event.metadata.push(metadata);
        self
    }

    pub async fn send_from(&self, tx: &Sender<Event>) {
        let mut send_task = self.event.clone();
        send_task.time_emitted = Utc::now().timestamp();
        log::info!(target: "event_log", "{send_task}");
        tx.send(send_task).await.unwrap_or_default();
    }

    pub fn blocking_send_from(&self, tx: &Sender<Event>) {
        let mut send_task = self.event.clone();
        send_task.time_emitted = Utc::now().timestamp();
        log::info!(target: "event_log", "{send_task}");
        tx.blocking_send(send_task).unwrap_or_default();
    }

    pub(crate) fn build(&self) -> Event {
        self.event.clone()
    }

    pub fn with_log_message(mut self, message: &str) -> EventBuilder {
        let metadata = Metadata::new_log_message(message);
        self.event.metadata.push(metadata);
        self
    }
}

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
    System(SystemEventType),
    Metric,
    Log,
    Notification,
    UserDefined,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemEventType {
    Trigger(SystemEventScope),
    Starting(SystemEventScope),
    Done(SystemEventScope, SystemEventResult),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemEventScope {
    Pipeline,
    Job,
    Task,
    EventHandler,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemEventResult {
    Success,
    Failed,
    Aborted,
    Errored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_like_each_other() {
        let e1 = Event {
            r#type: EventType::System(SystemEventType::Done(
                SystemEventScope::Task,
                SystemEventResult::Success,
            )),
            time_emitted: 0,
            metadata: vec![],
        };

        let e2 = Event {
            r#type: EventType::System(SystemEventType::Done(
                SystemEventScope::Task,
                SystemEventResult::Success,
            )),
            time_emitted: 1000,
            metadata: vec![],
        };

        let e3 = Event {
            r#type: EventType::System(SystemEventType::Done(
                SystemEventScope::Job,
                SystemEventResult::Success,
            )),
            time_emitted: 0,
            metadata: vec![],
        };

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn construct_log_event() {
        let event = Event::new(EventType::Log)
            .with_job_name("job_name")
            .with_pipeline_name("pipeline_name")
            .with_log_message("my log message")
            .build();
        assert!(true)
    }
}
