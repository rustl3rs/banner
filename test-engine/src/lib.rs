use std::{error::Error, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use banner_engine::{
    start, Engine, Event, ExecutionResult, Pipeline, PipelineSpecification, PragmasBuilder,
    TaskDefinition,
};
use local_engine::LocalEngine;
use tokio::sync::broadcast::{self, Sender};

pub struct TestEngine {
    local_engine: Arc<dyn Engine + Send + Sync>,
    tx: Option<Sender<Event>>,
}

impl TestEngine {
    /// Create a new [`TestEngine`] from a pipeline file.
    ///
    /// # Panics
    ///
    /// Panics if the file can't be found or loaded.
    pub async fn new(filepath: PathBuf) -> Self {
        let mut engine = LocalEngine::new();
        engine
            .with_pipeline_from_file(
                filepath.clone(),
                PragmasBuilder::new().register_context("test"),
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to load pipeline from file - {filepath:?}"));
        Self {
            local_engine: Arc::new(engine),
            tx: None,
        }
    }

    /// Starts this [`TestEngine`].
    ///
    /// # Panics
    ///
    /// Panics if the local engine fails to confirm requirements or initialise.
    pub async fn start(&mut self) {
        self.local_engine.confirm_requirements().await.unwrap();
        self.local_engine.initialise().await.unwrap();

        let (tx, rx) = broadcast::channel(100);

        self.tx = Some(tx.clone());
        let engine = self.local_engine.clone();
        tokio::spawn(async move { start(&engine, rx, tx).await });
        // let _ = start(&engine, rx, tx).await;
        // build_and_send_events(&self.local_engine, &self.tx).await;
        // wait_complete(rx);
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

    async fn get_state_for_id(&self, key: &str) -> Option<String> {
        self.local_engine.get_state_for_id(key).await
    }

    async fn set_state_for_id(
        &self,
        key: &str,
        value: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.local_engine.set_state_for_id(key, value).await
    }
}
