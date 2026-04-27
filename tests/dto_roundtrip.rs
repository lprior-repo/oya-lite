use oya_lite::lifecycle::types::{
    BeadId, Effect, EffectJournalEntry, LifecycleProgress, LifecycleRequest, LifecycleOutcome,
    ModelId, OpencodeServerConfig, OpencodeUrl, PromptString, RepoUrl, StepName, StepResult,
    Timestamp, Username, SensitiveString, WorkspaceName, WorkspacePath,
};

#[test]
fn lifecycle_request_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let req = LifecycleRequest {
        bead_id: BeadId::parse("test-bean")?,
        model: Some(ModelId("gpt-4".into())),
        repo: Some(RepoUrl("https://github.com/test/repo".into())),
        prompt: Some(PromptString("fix the bug".into())),
    };
    let json = serde_json::to_string(&req)?;
    let back: LifecycleRequest = serde_json::from_str(&json)?;
    assert_eq!(req.bead_id, back.bead_id);
    assert_eq!(req.prompt, back.prompt);
    assert_eq!(req.model, back.model);
    assert_eq!(req.repo, back.repo);
    Ok(())
}

#[test]
fn lifecycle_request_minimal_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let req = LifecycleRequest {
        bead_id: BeadId::parse("minimal")?,
        model: None,
        repo: None,
        prompt: None,
    };
    let json = serde_json::to_string(&req)?;
    let back: LifecycleRequest = serde_json::from_str(&json)?;
    assert!(back.model.is_none());
    assert!(back.repo.is_none());
    assert!(back.prompt.is_none());
    Ok(())
}

#[test]
fn lifecycle_progress_json_parse_workspace_prepare() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"kind":"step_started","step":"workspace-prepare","started_at":"2024-01-01T00:00:00Z"}"#;
    let p: LifecycleProgress = serde_json::from_str(json)?;
    assert!(matches!(p, LifecycleProgress::StepStarted { .. }));
    Ok(())
}

#[test]
fn lifecycle_progress_initialized_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "initialized",
        "bead_id": "test-bead",
        "steps": ["workspace-prepare"]
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    let serialized = serde_json::to_string(&progress)?;
    assert!(serialized.contains("initialized"));
    Ok(())
}

#[test]
fn lifecycle_progress_finished_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "kind": "finished",
        "result": "success",
        "message": "done"
    }"#;
    let progress: LifecycleProgress = serde_json::from_str(json)?;
    let serialized = serde_json::to_string(&progress)?;
    assert!(serialized.contains("finished"));
    Ok(())
}

#[test]
fn effect_journal_entry_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let entry = EffectJournalEntry {
        effect: Effect::WorkspacePrepare {
            workspace: WorkspaceName("ws".into()),
            path: WorkspacePath("/tmp".into()),
        },
        timeout_secs: 30,
        result: StepResult::Success,
        stdout: "done".into(),
        stderr: "".into(),
    };
    let json = serde_json::to_string(&entry)?;
    let back: EffectJournalEntry = serde_json::from_str(&json)?;
    assert_eq!(entry.timeout_secs, back.timeout_secs);
    assert_eq!(entry.result, back.result);
    Ok(())
}

#[test]
fn effect_journal_entry_opencode_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let entry = EffectJournalEntry {
        effect: Effect::Opencode {
            prompt: PromptString("hello".into()),
            model: ModelId("gpt-4".into()),
            cwd: Some(WorkspacePath("/workspace".into())),
        },
        timeout_secs: 3600,
        result: StepResult::Success,
        stdout: "response".into(),
        stderr: "".into(),
    };
    let json = serde_json::to_string(&entry)?;
    let back: EffectJournalEntry = serde_json::from_str(&json)?;
    assert_eq!(entry.timeout_secs, back.timeout_secs);
    Ok(())
}

#[test]
fn effect_journal_entry_failure_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let entry = EffectJournalEntry {
        effect: Effect::Jj {
            args: oya_lite::lifecycle::types::JjArgs(vec!["status".into()]),
            cwd: None,
        },
        timeout_secs: 120,
        result: StepResult::Failure,
        stdout: "".into(),
        stderr: "command failed".into(),
    };
    let json = serde_json::to_string(&entry)?;
    let back: EffectJournalEntry = serde_json::from_str(&json)?;
    assert!(!back.result.is_success());
    Ok(())
}

#[test]
fn lifecycle_outcome_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = LifecycleOutcome {
        bead_id: BeadId::parse("outcome-bean")?,
        result: StepResult::Success,
        state: oya_lite::lifecycle::types::Phase::Completed {
            bead: oya_lite::lifecycle::types::BeadData::from_bead_id(BeadId::parse("outcome-bean")?),
            result: StepResult::Success,
        },
        completed_steps: vec![StepName("step-1".into())],
    };
    let json = serde_json::to_string(&outcome)?;
    assert!(json.contains("completed"));
    assert!(json.contains("outcome-bean"));
    Ok(())
}

#[test]
fn step_result_snake_case_serde() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&StepResult::Success)?;
    assert!(json.contains("success"));
    let back: StepResult = serde_json::from_str(&json)?;
    assert_eq!(back, StepResult::Success);
    Ok(())
}

#[test]
fn timestamp_serde() -> Result<(), Box<dyn std::error::Error>> {
    let ts = Timestamp("2024-01-01T00:00:00Z".into());
    let json = serde_json::to_string(&ts)?;
    let back: Timestamp = serde_json::from_str(&json)?;
    assert_eq!(ts.as_str(), back.as_str());
    Ok(())
}

#[test]
fn model_id_serde() -> Result<(), Box<dyn std::error::Error>> {
    let model = ModelId("anthropic/claude-sonnet-4-20250514".into());
    let json = serde_json::to_string(&model)?;
    let back: ModelId = serde_json::from_str(&json)?;
    assert_eq!(model.as_str(), back.as_str());
    Ok(())
}

#[test]
fn repo_url_serde() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoUrl("https://github.com/test/repo.git".into());
    let json = serde_json::to_string(&repo)?;
    let back: RepoUrl = serde_json::from_str(&json)?;
    assert_eq!(repo.as_str(), back.as_str());
    Ok(())
}

#[test]
fn prompt_string_serde() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = PromptString("Fix the critical bug in the auth module".into());
    let json = serde_json::to_string(&prompt)?;
    let back: PromptString = serde_json::from_str(&json)?;
    assert_eq!(prompt.as_str(), back.as_str());
    Ok(())
}

#[test]
fn opencode_server_config_serde() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpencodeServerConfig {
        url: OpencodeUrl("http://localhost:4099".into()),
        username: Username("admin".into()),
        password: SensitiveString("secret".into()),
    };
    let json = serde_json::to_string(&config)?;
    assert!(json.contains("password"));
    let back: OpencodeServerConfig = serde_json::from_str(&json)?;
    assert_eq!(config.url, back.url);
    Ok(())
}

#[test]
fn workspace_name_serde() -> Result<(), Box<dyn std::error::Error>> {
    let name = WorkspaceName("my-workspace".into());
    let json = serde_json::to_string(&name)?;
    let back: WorkspaceName = serde_json::from_str(&json)?;
    assert_eq!(name, back);
    Ok(())
}

#[test]
fn workspace_path_serde() -> Result<(), Box<dyn std::error::Error>> {
    let path = WorkspacePath("/data/workspace".into());
    let json = serde_json::to_string(&path)?;
    let back: WorkspacePath = serde_json::from_str(&json)?;
    assert_eq!(path, back);
    Ok(())
}

#[test]
fn step_name_serde() -> Result<(), Box<dyn std::error::Error>> {
    let name = StepName("workspace-prepare".into());
    let json = serde_json::to_string(&name)?;
    let back: StepName = serde_json::from_str(&json)?;
    assert_eq!(name, back);
    Ok(())
}
