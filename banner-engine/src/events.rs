use std::fmt::Display;
use std::fmt::{Debug, Formatter, Result};

use chrono::{DateTime, TimeZone, Utc};
use strum_macros::{EnumIter, EnumString};
use tokio::sync::broadcast::Sender;

use crate::metadata::Metadata;

pub type Events = Vec<Event>;

#[derive(Debug, Clone, rune::Any)]
pub struct Event {
    r#type: EventType,
    time_emitted: i64,
    metadata: Vec<Metadata>,
}

impl Event {
    pub fn new_builder(r#type: EventType) -> EventBuilder {
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

    pub fn get_type(&self) -> EventType {
        self.r#type
    }

    pub fn time_emitted(&self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.time_emitted).unwrap()
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
        tracing::info!("Checking equality of {self:?} and {other:?}");
        match (self.r#type, other.r#type) {
            (EventType::System(sel), EventType::System(ser)) => match (sel, ser) {
                (SystemEventType::Trigger(sesl), SystemEventType::Trigger(sesr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task) => {
                            tracing::info!("=====> Triggering.");
                            matching_banner_metadata(&self.metadata, &other.metadata)
                        }
                        (_, _) => false,
                    }
                }
                (SystemEventType::Starting(sesl), SystemEventType::Starting(sesr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task)
                        | (SystemEventScope::EventHandler, SystemEventScope::EventHandler) => {
                            tracing::info!("=====> Starting something");
                            matching_banner_metadata(&self.metadata, &other.metadata)
                        }
                        (_, _) => false,
                    }
                }
                (SystemEventType::Done(sesl, serl), SystemEventType::Done(sesr, serr)) => {
                    match (sesl, sesr) {
                        (SystemEventScope::Pipeline, SystemEventScope::Pipeline)
                        | (SystemEventScope::Job, SystemEventScope::Job)
                        | (SystemEventScope::Task, SystemEventScope::Task)
                        | (SystemEventScope::EventHandler, SystemEventScope::EventHandler) => {
                            tracing::info!("=====> Done -> Pipeline/Job/Task/EventHandler");
                            match (serl, serr) {
                                (SystemEventResult::Success, SystemEventResult::Success)
                                | (SystemEventResult::Failed, SystemEventResult::Failed)
                                | (SystemEventResult::Aborted, SystemEventResult::Aborted)
                                | (SystemEventResult::Errored, SystemEventResult::Errored) => {
                                    tracing::info!("=====> Done -> ANY");
                                    matching_banner_metadata(&self.metadata, &other.metadata)
                                }
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
            | (EventType::Notification, EventType::Notification) => {
                tracing::info!("=====> External/Metric/Log/Notification");
                matching_banner_metadata(&self.metadata, &other.metadata)
            }
            (EventType::UserDefined, EventType::UserDefined) => {
                tracing::info!("=====> UserDefined");
                matching_banner_metadata(&self.metadata, &other.metadata)
            }
            (_, _) => false,
        }
    }
}

// metadata from the left is checked for existence in the right. If all are present; TRUE; otherwise; FALSE
pub(crate) fn matching_banner_metadata(lhs: &[Metadata], rhs: &[Metadata]) -> bool {
    tracing::debug!("LHS = {lhs:?}");
    tracing::debug!("RHS = {rhs:?}");
    lhs.iter().all(|tag| rhs.iter().any(|t| t == tag))
}

// fn get_pipeline_name(e: &Event) -> Option<&str> {
//     e.metadata.iter().find_map(|t| {
//         if t.key() == PIPELINE_TAG {
//             Some(t.value())
//         } else {
//             None
//         }
//     })
// }

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
        tx.send(send_task).unwrap_or_default();
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
// Metric: emit a metric for Banner to interpret and track
// Log: informational event with data.
// Notification: informational event that should result in a notification being sent to system users/operational systems.
// UserDefined: just what it says.
#[derive(Clone, Copy, PartialEq, rune::Any)]
pub enum EventType {
    External,
    System(#[rune(get)] SystemEventType),
    Metric,
    Log,
    Notification,
    UserDefined,
    Error,
}

impl Debug for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            EventType::External => write!(f, "External"),
            EventType::System(et) => write!(f, "System({et:?})"),
            EventType::Metric => write!(f, "Metric"),
            EventType::Log => write!(f, "Log"),
            EventType::Notification => write!(f, "Notification"),
            EventType::UserDefined => write!(f, "UserDefined"),
            EventType::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, rune::Any)]
pub enum SystemEventType {
    Trigger(#[rune(get)] SystemEventScope),
    Starting(#[rune(get)] SystemEventScope),
    Done(
        #[rune(get)] SystemEventScope,
        #[rune(get)] SystemEventResult,
    ),
}

#[derive(Debug, Clone, Copy, PartialEq, rune::Any)]
pub enum SystemEventScope {
    Pipeline,
    Job,
    Task,
    EventHandler,
}

// Success: represents successful completion of the task/job/pipeline (called a stop)
// Failed: represents faulty completion
// Aborted: means the step was cut short by some kind of intervention via Banner
// Errored: means the step was cut short by some external means; might be best to merge Errored and Aborted with a descriminator.
// TODO: Executions are either successful or not.  if not, then they have a result of execution-failed, aborted or system-errored. Maybe, think it thru more.
#[derive(Debug, Clone, Copy, PartialEq, rune::Any, EnumString, EnumIter)]
pub enum SystemEventResult {
    Success,
    Failed,
    Aborted,
    Errored,
    // Incomplete,
}

impl Display for SystemEventResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemEventResult::Success => write!(f, "Success"),
            SystemEventResult::Failed => write!(f, "Failed"),
            SystemEventResult::Aborted => write!(f, "Aborted"),
            SystemEventResult::Errored => write!(f, "Errored"),
            // SystemEventResult::Incomplete => write!(f, "incomplete"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_like_each_other() {
        let e1 = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        let e2 = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        let e3 = Event {
            r#type: EventType::System(SystemEventType::Done(
                SystemEventScope::Job,
                SystemEventResult::Success,
            )),
            time_emitted: 0,
            metadata: vec![],
        };

        let e4 = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Failed,
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        let e5 = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_2")
        .build();

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
        assert_ne!(e2, e4);
        assert_ne!(e2, e5);
    }

    #[test]
    fn construct_log_event() {
        Event::new_builder(EventType::Log)
            .with_job_name("job_name")
            .with_pipeline_name("pipeline_name")
            .with_log_message("my log message")
            .build();
        assert!(true)
    }
}
