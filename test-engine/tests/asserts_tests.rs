use test_engine::TestEngine;

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn get_only() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/get-only.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn jobs_before_tasks() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/jobs_before_tasks.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn failed_task() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/failed_task.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn multiple_jobs() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/multiple-jobs.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn minimal() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/minimal.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}

#[tokio::test]
#[tracing_test::traced_test]
#[cfg(feature = "docker")]
async fn multiple_jobs_sequential() {
    use std::path::PathBuf;

    let filepath = format!("../test-pipelines/multiple-jobs-sequential.ban",);
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.start().await;
}
