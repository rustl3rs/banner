use std::{str::FromStr, sync::Arc};

use rune::Any;
use tokio::sync::mpsc::Sender;

use crate::{
    Engine, Event, EventType, SystemEventResult, SystemEventScope, SystemEventType, JOB_TAG,
    PIPELINE_TAG, TASK_TAG,
};

#[derive(Any)]
pub struct RuneEngineWrapper {
    pub engine: Arc<dyn Engine + Send + Sync>,
    pub tx: Sender<Event>,
}

impl<'a> RuneEngineWrapper {
    pub async fn trigger_pipeline(&self, pipeline: &str) {
        log::trace!("trigger_pipeline");
        Event::new(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Pipeline,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx)
        .await;
    }

    pub async fn trigger_job(&self, pipeline: &str, job: &str) {
        Event::new(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Job,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx)
        .await;
    }

    pub async fn trigger_task(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Trigger(
            SystemEventScope::Task,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx)
        .await;
    }

    pub async fn log_message(&self, message: &str) {
        log::info!(target: "task_log", "LOGMESSAGE: {message}");
        Event::new(EventType::Log)
            .with_log_message(message)
            .send_from(&self.tx)
            .await;
    }

    pub async fn job_success(&self, pipeline: &str, job: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Job,
            SystemEventResult::Success,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx)
        .await;
    }

    pub async fn job_fail(&self, pipeline: &str, job: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Job,
            SystemEventResult::Failed,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx)
        .await;
    }

    pub async fn task_success(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx)
        .await;
    }

    pub async fn task_fail(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Failed,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx)
        .await;
    }

    pub async fn pipeline_success(&self, pipeline: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Pipeline,
            SystemEventResult::Success,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx)
        .await;
    }

    pub async fn pipeline_fail(&self, pipeline: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Pipeline,
            SystemEventResult::Failed,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx)
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
        let value_from_state = self.engine.get_state_for_id(key);
        match value_from_state {
            Some(value) => {
                let result = SystemEventResult::from_str(&value);
                log::warn!(target: "task_log", "get_from_state called on {key} with value {value}, leaving result: {result:?}");
                match result {
                    Ok(r) => {
                        log::warn!(target: "task_log", "Some(r): {:?}", Some(r));
                        Some(r)
                    }
                    Err(_) => {
                        log::error!(target: "task_log", "get_from_state called on {key} with value {value:?} which is not a valid SystemEventResult");
                        None
                    }
                }
            }
            None => {
                log::error!(target: "task_log", "get_from_state called on {key} which does not exist");
                None
            }
        }
    }

    pub async fn set_state_for_task(&self, _scope: &str, key: &str, value: SystemEventResult) {
        let val = value.to_string();
        let result = self.engine.set_state_for_id(key, val);
        match result {
            Ok(_) => {
                log::warn!(target: "task_log", "set_state_for_task called on {key} with value {value}");
            }
            Err(e) => {
                log::error!(target: "task_log", "set_state_for_task called on {key} but persistence failed: {e:?}");
            }
        }
    }

    pub async fn get_pipeline_metadata_from_event(
        &self,
        event: &'a Event,
    ) -> (&'a str, &'a str, &'a str) {
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
        let task = event
            .metadata()
            .iter()
            .find(|md| md.key() == TASK_TAG)
            .unwrap()
            .value();
        (pipeline, job, task)
    }
}
