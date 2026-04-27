use oya_lite::lifecycle::types::*;

#[test]
fn lifecycle_config_default() {
    let config = oya_lite::lifecycle::run::LifecycleConfig::default();
    assert_eq!(config.data_dir.0, ".oya-lite");
    assert!(config.opencode_server.is_none());
}

#[test]
fn lifecycle_config_with_data_dir() {
    let config = oya_lite::lifecycle::run::LifecycleConfig {
        data_dir: DataDirPath("/custom/path".into()),
        opencode_server: None,
    };
    assert_eq!(config.data_dir.0, "/custom/path");
}

#[test]
fn lifecycle_request_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let request = LifecycleRequest {
        bead_id: BeadId::parse("test-bean")?,
        model: Some(ModelId("gpt-4".into())),
        repo: Some(RepoUrl("https://github.com/test/repo".into())),
        prompt: Some(PromptString("hello world".into())),
    };
    let json = serde_json::to_string(&request)?;
    assert!(json.contains("test-bean"));
    let back: LifecycleRequest = serde_json::from_str(&json)?;
    assert_eq!(back.bead_id, request.bead_id);
    Ok(())
}

#[test]
fn lifecycle_request_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let request = LifecycleRequest {
        bead_id: BeadId::parse("minimal")?,
        model: None,
        repo: None,
        prompt: None,
    };
    let json = serde_json::to_string(&request)?;
    let back: LifecycleRequest = serde_json::from_str(&json)?;
    assert_eq!(back.bead_id, request.bead_id);
    Ok(())
}

#[test]
fn lifecycle_progress_initialized_serde() -> Result<(), Box<dyn std::error::Error>> {
    let progress = LifecycleProgress::Initialized {
        bead_id: BeadId::parse("init-test")?,
        steps: vec![StepName("step1".into()), StepName("step2".into())],
    };
    let json = serde_json::to_string(&progress)?;
    assert!(json.contains("initialized"));
    assert!(json.contains("init-test"));
    Ok(())
}

#[test]
fn lifecycle_progress_step_started_serde() -> Result<(), Box<dyn std::error::Error>> {
    let progress = LifecycleProgress::StepStarted {
        step: StepName("my-step".into()),
        started_at: Timestamp("2024-01-01T00:00:00Z".into()),
    };
    let json = serde_json::to_string(&progress)?;
    assert!(json.contains("step_started"));
    assert!(json.contains("my-step"));
    Ok(())
}

#[test]
fn lifecycle_progress_finished_serde() -> Result<(), Box<dyn std::error::Error>> {
    let progress = LifecycleProgress::Finished {
        result: StepResult::Success,
        message: Some(ErrorMessage("done".into())),
    };
    let json = serde_json::to_string(&progress)?;
    assert!(json.contains("finished"));
    assert!(json.contains("success"));
    Ok(())
}

#[test]
fn lifecycle_progress_step_failed_serde() -> Result<(), Box<dyn std::error::Error>> {
    let progress = LifecycleProgress::StepFailed {
        step: StepName("failing-step".into()),
        error: ErrorMessage("it broke".into()),
    };
    let json = serde_json::to_string(&progress)?;
    assert!(json.contains("step_failed"));
    assert!(json.contains("failing-step"));
    Ok(())
}