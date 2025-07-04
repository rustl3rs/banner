use banner_engine::Engine;
use test_engine::TestEngine;

#[tokio::test]
#[cfg(feature = "docker")]
async fn passes_requirments_check() {
    use std::path::PathBuf;

    let filepath = format!(
        "{}/../test-pipelines/get-only.ban",
        env!("CARGO_MANIFEST_DIR")
    );
    let path = PathBuf::from(filepath);
    println!("{path:?}");
    let engine = TestEngine::new(path);
    engine.await.confirm_requirements().await.unwrap();
}
