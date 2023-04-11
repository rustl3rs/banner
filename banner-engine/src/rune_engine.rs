use std::sync::Arc;

use log::{error, info};
use rune::Any;
use tokio::sync::mpsc::Sender;

use crate::{Engine, Event, EventType, SystemEventResult, SystemEventScope, SystemEventType};

#[derive(Any)]
pub struct RuneEngineWrapper {
    pub engine: Arc<dyn Engine + Send + Sync>,
    pub tx: Sender<Event>,
}

impl RuneEngineWrapper {
    pub async fn trigger_pipeline(&self, pipeline: &str) {
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
        Event::new(EventType::Log)
            .with_log_message(message)
            .send_from(&self.tx)
            .await;
        info!(target: "task_log", "{message}");
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
            SystemEventResult::Errored,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .send_from(&self.tx)
        .await;
    }

    pub async fn task_success(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Job,
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
            SystemEventScope::Job,
            SystemEventResult::Errored,
        )))
        .with_pipeline_name(pipeline)
        .with_job_name(job)
        .with_task_name(task)
        .send_from(&self.tx)
        .await;
    }

    pub async fn pipeline_success(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Pipeline,
            SystemEventResult::Success,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx)
        .await;
    }

    pub async fn pipeline_fail(&self, pipeline: &str, job: &str, task: &str) {
        Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Pipeline,
            SystemEventResult::Errored,
        )))
        .with_pipeline_name(pipeline)
        .send_from(&self.tx)
        .await;
    }

    pub async fn execute_task_name_in_scope(
        &self,
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
            Ok(er) => {
                info!(target: "task_log", "Task completed successfully: {:?}", er);
                self.task_success(pipeline, job, task).await;
            }
            Err(e) => {
                error!(target: "task_log", "Task execution failed: {:?}", e);
                self.task_fail(pipeline, job, task).await;
            }
        };
    }
}
