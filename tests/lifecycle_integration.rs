use oya_lite::lifecycle::run::LifecycleOrchestrator;
use oya_lite::lifecycle::types::{BeadId, LifecycleProgress, LifecycleRequest};
use tempfile::TempDir;

#[test]
fn lifecycle_orchestrator_new_opens_state_db() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let config = oya_lite::lifecycle::run::LifecycleConfig {
        data_dir: oya_lite::lifecycle::types::DataDirPath(
            dir.path().to_string_lossy().into_owned(),
        ),
        opencode_server: None,
    };
    let orch = LifecycleOrchestrator::new(config)?;
    assert!(orch.get_workflow_state(&BeadId::parse("new-bean")?).is_ok());
    Ok(())
}

#[test]
fn lifecycle_orchestrator_get_workflow_state_returns_none_for_new_bead(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let config = oya_lite::lifecycle::run::LifecycleConfig {
        data_dir: oya_lite::lifecycle::types::DataDirPath(
            dir.path().to_string_lossy().into_owned(),
        ),
        opencode_server: None,
    };
    let orch = LifecycleOrchestrator::new(config)?;
    let id = BeadId::parse("my-bean")?;
    let state = orch.get_workflow_state(&id)?;
    assert!(state.is_none());
    Ok(())
}

#[test]
fn lifecycle_request_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "bead_id": "test-bead",
        "model": "gpt-4",
        "repo": "https://github.com/test/repo",
        "prompt": "fix the bug"
    }"#;
    let req: LifecycleRequest = serde_json::from_str(json)?;
    assert_eq!(req.bead_id.as_str(), "test-bead");
    assert_eq!(req.model.as_ref().map(|m| m.as_str()), Some("gpt-4"));
    assert_eq!(req.repo.as_ref().map(|r| r.as_str()), Some("https://github.com/test/repo"));
    assert_eq!(req.prompt.as_ref().map(|p| p.as_str()), Some("fix the bug"));
    Ok(())
}

#[test]
fn lifecycle_request_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"bead_id": "minimal-bead"}"#;
    let req: LifecycleRequest = serde_json::from_str(json)?;
    assert_eq!(req.bead_id.as_str(), "minimal-bead");
    assert!(req.model.is_none());
    assert!(req.repo.is_none());
    assert!(req.prompt.is_none());
    Ok(())
}

#[test]
fn lifecycle_progress_initialized_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "initialized",
        "bead_id": "test-bead",
        "steps": ["workspace-prepare", "opencode-run"]
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::Initialized {
            bead_id,
            steps
        } if bead_id.as_str() == "test-bead" && steps.len() == 2
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_step_started_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "step_started",
        "step": "workspace-prepare",
        "started_at": "2024-01-01T00:00:00Z"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::StepStarted {
            step,
            started_at,
        } if step.as_str() == "workspace-prepare" && started_at.as_str() == "2024-01-01T00:00:00Z"
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_step_completed_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "step_completed",
        "step": "workspace-prepare",
        "duration_ms": 150
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::StepCompleted {
            step,
            duration_ms: 150,
        } if step.as_str() == "workspace-prepare"
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_step_failed_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "step_failed",
        "step": "opencode-run",
        "error": "model not found"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::StepFailed {
            step,
            error,
        } if step.as_str() == "opencode-run" && error.as_str() == "model not found"
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_finished_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "finished",
        "result": "success",
        "message": "all steps completed"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::Finished {
            result: oya_lite::lifecycle::types::StepResult::Success,
            message: Some(msg),
        } if msg.as_str() == "all steps completed"
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_finished_failure_parse() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "finished",
        "result": "failure",
        "message": "step failed"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(
        progress,
        LifecycleProgress::Finished {
            result: oya_lite::lifecycle::types::StepResult::Failure,
            message: Some(msg),
        } if msg.as_str() == "step failed"
    ));
    Ok(())
}

#[test]
fn lifecycle_progress_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "step_started",
        "step": "workspace-prepare",
        "started_at": "2024-01-01T00:00:00Z"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    let serialized = serde_json::to_string(&progress)?;
    assert!(serialized.contains("step_started"));
    assert!(serialized.contains("workspace-prepare"));
    Ok(())
}

#[test]
fn lifecycle_config_default() {
    let config = oya_lite::lifecycle::run::LifecycleConfig::default();
    assert!(config.opencode_server.is_none());
    assert_eq!(config.data_dir.as_str(), ".oya-lite");
}

#[test]
fn lifecycle_orchestrator_with_real_db() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let config = oya_lite::lifecycle::run::LifecycleConfig {
        data_dir: oya_lite::lifecycle::types::DataDirPath(
            dir.path().to_string_lossy().into_owned(),
        ),
        opencode_server: None,
    };
    let orch = LifecycleOrchestrator::new(config)?;
    let id = BeadId::parse("ghost-bean")?;
    let result = orch.get_workflow_state(&id)?;
    assert!(result.is_none());
    Ok(())
}
