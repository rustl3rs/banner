use std::{error::Error, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use banner_engine::{Engine, ExecutionResult, Pipeline, PipelineSpecification, TaskDefinition};
use local_engine::LocalEngine;

pub struct TestEngine {
    local_engine: Arc<dyn Engine + Send + Sync>,
}

impl TestEngine {
    pub async fn new(filepath: PathBuf) -> Self {
        let mut engine = LocalEngine::new();
        engine
            .with_pipeline_from_file(filepath.clone())
            .await
            .expect(&format!("Failed to load pipeline from file - {filepath:?}"));
        Self {
            local_engine: Arc::new(engine),
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

    fn get_state_for_id(&self, key: &str) -> Option<String> {
        self.local_engine.get_state_for_id(key)
    }

    fn set_state_for_id(
        &self,
        key: &str,
        value: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.local_engine.set_state_for_id(key, value)
    }
}
