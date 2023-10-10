use std::path::PathBuf;

use rstest::rstest;
use test_engine::TestEngine;

#[rstest]
#[tokio::test]
pub async fn test_all_pipelines(#[files("../test-pipelines/*.ban")] path: PathBuf) {
    let filename = path.to_str().unwrap();
    println!("Testing pipeline {}", filename);
    let mut engine = TestEngine::new(path).await;
    engine.start();
    // engine.verify().await;
}
