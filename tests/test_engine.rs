use banner_engine::{start_engine, Engine};
use test_engine::TestEngine;
use tokio::sync::broadcast;

#[test]
pub fn test() {
    assert!(true);
}

#[tokio::test]
pub async fn test2() {
    // get all banner files in the test-pipelines directory
    let files = std::fs::read_dir("../test-pipelines").unwrap();

    let (tx, rx) = broadcast::channel(100);

    // for each file, create a TestEngine and run it
    for file in files {
        let file = file.unwrap();
        let filepath = file.path();
        let filename = filepath.to_str().unwrap();
        println!("Testing pipeline {}", filename);
        let engine = TestEngine::new(filepath).await;
        engine.confirm_requirements().await.unwrap();
        engine.initialise().await.unwrap();
        let pipelines = engine.get_pipelines();
        for pipeline in pipelines {
            let pipeline_name = pipeline.name();
            let jobs = pipeline.get_jobs();
            for job in jobs {
                let job_name = job.name();
                let tasks = job.get_tasks();
                for task in tasks {
                    let task_name = task.name();
                    let result = start_engine(engine, rx, tx).await.unwrap();
                    println!("Result: {:?}", result);
                }
            }
        }
    }
}
