use std::{str::FromStr, sync::Arc};

use rune::Any;
use tokio::sync::broadcast::Sender;

use crate::{
    Engine, Event, EventType, ExecutionStatus, SystemEventResult, SystemEventScope,
    SystemEventType, JOB_TAG, PIPELINE_TAG, TASK_TAG,
};

#[derive(Any)]
pub struct EventHandlerEngine {
    pub engine: Arc<dyn Engine + Send + Sync>,
    pub tx: Sender<Event>,
}

impl<'a> EventHandlerEngine {
    pub async fn trigger_pipeline(&self, pipeline: &str) {
        log::trace!("trigger_pipeline");
        Event::new_builder(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Pipeline,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx);

        let _ = self
            .engine
            .set_state_for_id(
                &format!("{}/{}", "", pipeline),
                ExecutionStatus::Running.to_string(),
            )
            .await;
    }

    pub async fn trigger_job(&self, pipeline: &str, job: &str) {
        Event::new_builder(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Job,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx);

        let _ = self
            .engine
            .set_state_for_id(
                &format!("{}/{}/{}", "", pipeline, job),
                ExecutionStatus::Running.to_string(),
            )
            .await;
    }

    pub async fn trigger_task(&self, pipeline: &str, job: &str, task: &str) {
        Event::new_builder(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Task,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx);

        let _ = self
            .engine
            .set_state_for_id(
                &format!("{}/{}/{}/{}", "", pipeline, job, task),
                ExecutionStatus::Running.to_string(),
            )
            .await;
    }

    pub fn log_message(&self, message: &str) {
        log::info!(target: "task_log", "LOGMESSAGE: {message}");
        Event::new_builder(EventType::Log)
            .with_log_message(message)
            .send_from(&self.tx);
    }

    pub async fn pipeline_complete(&self, event: &Event) {
        let (pipeline, _, _) = Self::get_pipeline_metadata_from_event_tags(event);
        let key = format!("{scope}/{pipeline}", scope = "");
        let result = match event.get_type() {
            EventType::System(SystemEventType::Done(SystemEventScope::Job, result)) => {
                match result {
                    SystemEventResult::Success => {
                        let _ = self
                            .engine
                            .set_state_for_id(&key, ExecutionStatus::Success.to_string())
                            .await;
                    }
                    _ => {
                        let _ = self
                            .engine
                            .set_state_for_id(&key, ExecutionStatus::Failed.to_string())
                            .await;
                    }
                }
                result
            }
            _ => {
                panic!("Invalid event type for pipeline complete")
            }
        };

        Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Pipeline,
            result,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx);
    }

    pub async fn job_complete(&self, event: &Event) {
        let (pipeline, job, _) = Self::get_pipeline_metadata_from_event(event);
        let key = format!("{scope}/{pipeline}/{job}", scope = "");
        let result = match event.get_type() {
            EventType::System(SystemEventType::Done(SystemEventScope::Task, result)) => {
                match result {
                    SystemEventResult::Success => {
                        let _ = self
                            .engine
                            .set_state_for_id(&key, ExecutionStatus::Success.to_string())
                            .await;
                    }
                    _ => {
                        let _ = self
                            .engine
                            .set_state_for_id(&key, ExecutionStatus::Failed.to_string())
                            .await;
                    }
                }
                result
            }
            _ => {
                panic!("Invalid event type for job complete")
            }
        };

        Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Job,
            result,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx);
    }

    pub async fn task_success(&self, pipeline: &str, job: &str, task: &str) {
        Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx);

        let _ = self
            .engine
            .set_state_for_id(
                &format!("{}/{}/{}/{}", "", pipeline, job, task),
                ExecutionStatus::Success.to_string(),
            )
            .await;
    }

    pub async fn task_fail(&self, pipeline: &str, job: &str, task: &str) {
        Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Failed,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx);

        let _ = self
            .engine
            .set_state_for_id(
                &format!("{}/{}/{}/{}", "", pipeline, job, task),
                ExecutionStatus::Failed.to_string(),
            )
            .await;
    }

    pub async fn execute_task_name_in_scope(
        &mut self,
        scope: &str,
        pipeline: &str,
        job: &str,
        task: &str,
    ) {
        let result = self
            .engine
            .execute_task_name_in_scope(scope, pipeline, job, task)
            .await;
        match result {
            Ok(er) => match er {
                crate::ExecutionResult::Success(_) => {
                    log::info!(target: "task_log", "Task [{}/{}/{}] completed successfully: {:?}", pipeline, job, task, er);
                    self.task_success(pipeline, job, task).await;
                }
                crate::ExecutionResult::Failed(e) => {
                    // TODO: iterate over and log the events raised by the execution target.
                    log::error!(target: "task_log", "Task [{}/{}/{}] execution failed: {:?}", pipeline, job, task, e);
                    self.task_fail(pipeline, job, task).await;
                }
            },
            Err(e) => {
                log::error!(target: "task_log", "Task [{}/{}/{}] execution failed: {:?}", pipeline, job, task, e);
                self.task_fail(pipeline, job, task).await;
            }
        };
    }

    pub async fn get_from_state(&self, _scope: &str, key: &str) -> Option<SystemEventResult> {
        let value_from_state = self.engine.get_state_for_id(key).await;
        if let Some(value) = value_from_state {
            let result = SystemEventResult::from_str(&value);
            log::warn!(target: "task_log", "get_from_state called on {key} with value {value}, leaving result: {result:?}");
            if let Ok(r) = result {
                log::debug!(target: "task_log", "Some(r): {:?}", Some(r));
                Some(r)
            } else {
                log::error!(target: "task_log", "get_from_state called on {key} with value {value:?} which is not a valid SystemEventResult");
                None
            }
        } else {
            log::warn!(target: "task_log", "get_from_state called on {key} which does not exist");
            None
        }
    }

    pub async fn set_state_for_task(&self, _scope: &str, key: &str, value: &SystemEventResult) {
        let val = value.to_string();
        let result = self.engine.set_state_for_id(key, val).await;
        match result {
            Ok(()) => {
                log::debug!(target: "task_log", "set_state_for_task called on {key} with value {value}");
            }
            Err(e) => {
                log::error!(target: "task_log", "set_state_for_task called on {key} but persistence failed: {e:?}");
            }
        }
    }

    pub fn get_pipeline_metadata_from_event(event: &'a Event) -> (&'a str, &'a str, &'a str) {
        let pipeline = event
            .metadata()
            .iter()
            .find(|md| md.key() == PIPELINE_TAG)
            .unwrap()
            .value();
        let job = event
            .metadata()
            .iter()
            .find(|md| md.key() == JOB_TAG)
            .unwrap()
            .value();
        match event.metadata().iter().find(|md| md.key() == TASK_TAG) {
            Some(task) => (pipeline, job, task.value()),
            None => (pipeline, job, ""),
        }
    }

    pub fn get_pipeline_metadata_from_event_tags(
        event: &'a Event,
    ) -> (&'a str, Option<&'a str>, Option<&'a str>) {
        let pipeline = event
            .metadata()
            .iter()
            .find(|md| md.key() == PIPELINE_TAG)
            .unwrap()
            .value();
        match event.metadata().iter().find(|md| md.key() == JOB_TAG) {
            Some(job) => match event.metadata().iter().find(|md| md.key() == TASK_TAG) {
                Some(task) => (pipeline, Some(job.value()), Some(task.value())),
                None => (pipeline, Some(job.value()), None),
            },
            None => (pipeline, None, None),
        }
    }
}
