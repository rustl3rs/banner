use banner_engine::{Engine, Image, TaskDefinition};
use local_engine::LocalEngine;

#[tokio::test]
#[cfg(feature = "docker")]
async fn passes_requirments_check() {
    let engine = LocalEngine::new();
    engine.confirm_requirements().await.unwrap();
}

#[tokio::test]
#[cfg(feature = "docker")]
async fn starts_task_with_required_tags() {
    use banner_engine::{ExecutionResult, Tag};
    use std::error::Error;

    async fn execute(tags: Vec<Tag>) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
        let engine = LocalEngine::new();
        let image = Image::new(String::from("alpine:latest"), None);
        let command = vec![
            String::from("sh"),
            String::from("-ce"),
            String::from("echo rustl3rs;"),
        ];
        let inputs = vec![];
        let outputs = vec![];
        let task = TaskDefinition::new(tags, image, command, inputs, outputs);
        engine.execute(&task).await
    }

    async fn check_succeeds(tags: Vec<Tag>) {
        let result = execute(tags).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    async fn check_fails(expected_error: &str, tags: Vec<Tag>) {
        let result = execute(tags).await;
        assert!(result.is_err());
        assert_eq!(&format!("{}", result.err().unwrap()), expected_error);
    }

    check_succeeds(vec![]).await;

    let pipeline_tag = Tag::new("banner.dev/pipeline", "test-pipeline");
    check_succeeds(vec![pipeline_tag.clone()]).await;

    let job_tag = Tag::new("banner.dev/job", "test-job");
    check_succeeds(vec![pipeline_tag.clone(), job_tag.clone()]).await;

    let task_tag = Tag::new("banner.dev/task", "test-task");
    check_succeeds(vec![pipeline_tag.clone(), job_tag.clone(), task_tag]).await;
}
