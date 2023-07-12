use std::fmt::Display;
use std::fmt::{Debug, Formatter, Result};

use strum_macros::{EnumIter, EnumString};

use crate::metadata::Metadata;
use crate::select::Select::*;
use crate::{Event, EventType, Select, SystemEventResult, SystemEventScope, SystemEventType};

pub type ListenForEvents = Vec<ListenForEvent>;

#[derive(Debug, Clone)]
pub struct ListenForEvent {
    r#type: ListenForEventType,
    metadata: Vec<Metadata>,
}

impl ListenForEvent {
    pub fn new(r#type: ListenForEventType) -> ListenForEventBuilder {
        ListenForEventBuilder {
            event: Self {
                r#type,
                metadata: vec![],
            },
        }
    }

    pub fn r#type(&self) -> &ListenForEventType {
        &self.r#type
    }

    pub fn metadata(&self) -> &[Metadata] {
        self.metadata.as_ref()
    }

    pub fn matches_event(&self, other: &Event) -> bool {
        match (self.r#type, other.r#type()) {
            (ListenForEventType::System(sel), EventType::System(ser)) => match (sel, ser) {
                (Any, _) => true,
                (Only(ListenForSystemEventType::Trigger(sesl)), SystemEventType::Trigger(sesr)) => {
                    match (sesl, sesr) {
                        (Only(ListenForSystemEventScope::Pipeline), SystemEventScope::Pipeline)
                        | (Only(ListenForSystemEventScope::Job), SystemEventScope::Job)
                        | (Only(ListenForSystemEventScope::Task), SystemEventScope::Task) => {
                            matching_banner_metadata(&self.metadata, &other.metadata())
                        }
                        (Any, _) => true,
                        (_, _) => false,
                    }
                }
                (
                    Only(ListenForSystemEventType::Starting(sesl)),
                    SystemEventType::Starting(sesr),
                ) => match (sesl, sesr) {
                    (Only(ListenForSystemEventScope::Pipeline), SystemEventScope::Pipeline)
                    | (Only(ListenForSystemEventScope::Job), SystemEventScope::Job)
                    | (Only(ListenForSystemEventScope::Task), SystemEventScope::Task)
                    | (
                        Only(ListenForSystemEventScope::EventHandler),
                        SystemEventScope::EventHandler,
                    ) => matching_banner_metadata(&self.metadata, &other.metadata()),
                    (Any, _) => true,
                    (_, _) => false,
                },
                (
                    Only(ListenForSystemEventType::Done(sesl, serl)),
                    SystemEventType::Done(sesr, serr),
                ) => match (sesl, sesr) {
                    (Only(ListenForSystemEventScope::Pipeline), SystemEventScope::Pipeline)
                    | (Only(ListenForSystemEventScope::Job), SystemEventScope::Job)
                    | (Only(ListenForSystemEventScope::Task), SystemEventScope::Task)
                    | (
                        Only(ListenForSystemEventScope::EventHandler),
                        SystemEventScope::EventHandler,
                    )
                    | (Any, _) => match (serl, serr) {
                        (Only(ListenForSystemEventResult::Success), SystemEventResult::Success)
                        | (Only(ListenForSystemEventResult::Failed), SystemEventResult::Failed)
                        | (Only(ListenForSystemEventResult::Aborted), SystemEventResult::Aborted)
                        | (Only(ListenForSystemEventResult::Errored), SystemEventResult::Errored)
                        | (Any, _) => matching_banner_metadata(&self.metadata, &other.metadata()),
                        (_, _) => false,
                    },
                    (_, _) => false,
                },
                (_, _) => false,
            },
            (ListenForEventType::External, EventType::External)
            | (ListenForEventType::Metric, EventType::Metric)
            | (ListenForEventType::Log, EventType::Log)
            | (ListenForEventType::Notification, EventType::Notification) => {
                matching_banner_metadata(&self.metadata, &other.metadata())
            }
            (ListenForEventType::UserDefined, EventType::UserDefined) => {
                matching_banner_metadata(&self.metadata, &other.metadata())
            }
            (_, _) => false,
        }
    }
}

impl Display for ListenForEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} | [", self.r#type())?;
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

// metadata from the left is checked for existence in the right. If all are present; TRUE; otherwise; FALSE
pub(crate) fn matching_banner_metadata(lhs: &[Metadata], rhs: &[Metadata]) -> bool {
    lhs.iter().all(|tag| rhs.iter().any(|t| t == tag))
}

pub struct ListenForEventBuilder {
    event: ListenForEvent,
}

// TODO: Make impossible states impossible; where possible.
//   eg: log messages should only be attachable to Log event types.
impl ListenForEventBuilder {
    pub fn with_pipeline_name(mut self, pipeline_name: &str) -> ListenForEventBuilder {
        let metadata = Metadata::new_banner_pipeline(pipeline_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_job_name(mut self, job_name: &str) -> ListenForEventBuilder {
        let metadata = Metadata::new_banner_job(job_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_task_name(mut self, task_name: &str) -> ListenForEventBuilder {
        let metadata = Metadata::new_banner_task(task_name);
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> ListenForEventBuilder {
        self.event.metadata.push(metadata);
        self
    }

    pub fn with_listen_for_event(mut self, event: &ListenForEvent) -> ListenForEventBuilder {
        let metadata = Metadata::new_listen_for_event(event);
        self.event.metadata.push(metadata);
        self
    }

    pub(crate) fn build(&self) -> ListenForEvent {
        self.event.clone()
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
pub enum ListenForEventType {
    External,
    System(Select<ListenForSystemEventType>),
    Metric,
    Log,
    Notification,
    UserDefined,
    Error,
}

impl Debug for ListenForEventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ListenForEventType::External => write!(f, "External"),
            ListenForEventType::System(et) => write!(f, "System({et:?})"),
            ListenForEventType::Metric => write!(f, "Metric"),
            ListenForEventType::Log => write!(f, "Log"),
            ListenForEventType::Notification => write!(f, "Notification"),
            ListenForEventType::UserDefined => write!(f, "UserDefined"),
            ListenForEventType::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, rune::Any)]
pub enum ListenForSystemEventType {
    Trigger(Select<ListenForSystemEventScope>),
    Starting(Select<ListenForSystemEventScope>),
    Done(
        Select<ListenForSystemEventScope>,
        Select<ListenForSystemEventResult>,
    ),
}

#[derive(Debug, Clone, Copy, PartialEq, rune::Any)]
pub enum ListenForSystemEventScope {
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
pub enum ListenForSystemEventResult {
    Success,
    Failed,
    Aborted,
    Errored,
    // Incomplete,
}

impl Display for ListenForSystemEventResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListenForSystemEventResult::Success => write!(f, "success"),
            ListenForSystemEventResult::Failed => write!(f, "failed"),
            ListenForSystemEventResult::Aborted => write!(f, "aborted"),
            ListenForSystemEventResult::Errored => write!(f, "errored"),
            // ListenForSystemEventResult::Incomplete => write!(f, "incomplete"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_events_like_each_other() {
        let e1 = Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        // exact match
        let lfe1 = ListenForEvent::new(ListenForEventType::System(Select::Only(
            ListenForSystemEventType::Done(
                Select::Only(ListenForSystemEventScope::Task),
                Select::Only(ListenForSystemEventResult::Success),
            ),
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        // any pipeline/job/task that completes successfully
        let lfe2 = ListenForEvent {
            r#type: ListenForEventType::System(Select::Only(ListenForSystemEventType::Done(
                Select::Any,
                Select::Only(ListenForSystemEventResult::Success),
            ))),
            metadata: vec![],
        };

        // failed task
        let lfe3 = ListenForEvent::new(ListenForEventType::System(Select::Only(
            ListenForSystemEventType::Done(
                Select::Only(ListenForSystemEventScope::Task),
                Select::Only(ListenForSystemEventResult::Failed),
            ),
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        // successful task2
        let lfe4 = ListenForEvent::new(ListenForEventType::System(Select::Only(
            ListenForSystemEventType::Done(
                Select::Only(ListenForSystemEventScope::Task),
                Select::Only(ListenForSystemEventResult::Success),
            ),
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_2")
        .build();

        // any result for task1
        let lfe5 = ListenForEvent::new(ListenForEventType::System(Select::Only(
            ListenForSystemEventType::Done(
                Select::Only(ListenForSystemEventScope::Task),
                Select::Any,
            ),
        )))
        .with_pipeline_name("pipeline_1")
        .with_job_name("job_1")
        .with_task_name("task_1")
        .build();

        assert!(lfe1.matches_event(&e1));
        assert!(lfe2.matches_event(&e1));
        assert!(!lfe3.matches_event(&e1));
        assert!(!lfe4.matches_event(&e1));
        assert!(lfe5.matches_event(&e1));
    }

    #[test]
    fn construct_log_event() {
        ListenForEvent::new(ListenForEventType::Log)
            .with_job_name("job_name")
            .with_pipeline_name("pipeline_name")
            .build();
        assert!(true)
    }
}
