#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod opencode_server;
mod steps;

use crate::lifecycle::effects::TokioCommandExecutor;
use crate::lifecycle::error::LifecycleError;
use crate::lifecycle::state::state_db::StateDbError;
use crate::lifecycle::state::{load_state, persist_state, StateDb};
use crate::lifecycle::types::{
    BeadId, Effect, EffectJournalEntry, ErrorMessage, LifecycleProgress, LifecycleRequest,
    LifecycleStep, ModelId, OpencodeServerConfig, PromptString, StateEvent,
    StepName, StepResult, WorkflowState, WorkspaceName, WorkspacePath,
};
use std::path::PathBuf;
use steps::{execute_steps, StepFailure, StepRun};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-20250514";

pub struct LifecycleConfig {
    pub data_dir: crate::lifecycle::types::DataDirPath,
    pub opencode_server: Option<OpencodeServerConfig>,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            data_dir: crate::lifecycle::types::DataDirPath(".oya-lite".to_owned()),
            opencode_server: None,
        }
    }
}

pub struct LifecycleOrchestrator {
    db: StateDb,
    executor: TokioCommandExecutor,
    server_config: Option<OpencodeServerConfig>,
}

impl LifecycleOrchestrator {
    pub fn new(config: LifecycleConfig) -> std::result::Result<Self, StateDbError> {
        let db = StateDb::open(PathBuf::from(&config.data_dir.0))?;
        Ok(Self {
            db,
            executor: TokioCommandExecutor::new(),
            server_config: config.opencode_server,
        })
    }

    pub async fn run_lifecycle(
        &self,
        request: LifecycleRequest,
    ) -> std::result::Result<mpsc::Receiver<LifecycleProgress>, LifecycleError> {
        let (tx, rx) = mpsc::channel(100);
        let db = self.db.clone();
        let executor = self.executor;
        let server_config = self.server_config.clone();
        tokio::spawn(async move {
            if let Err(e) = run_lifecycle_inner(db, executor, server_config, request, tx).await {
                error!("lifecycle error: {e}");
            }
        });
        Ok(rx)
    }

    #[allow(dead_code)]
    pub fn get_workflow_state(
        &self,
        bead_id: &BeadId,
    ) -> std::result::Result<Option<WorkflowState>, LifecycleError> {
        load_state(&self.db, bead_id)
            .map(|opt| opt.map(|(s, _)| s))
            .map_err(|e| {
                LifecycleError::terminal(
                    crate::lifecycle::error::FailureCategory::Workspace,
                    e.to_string(),
                )
            })
    }
}

pub(super) async fn send_progress(
    tx: &mpsc::Sender<LifecycleProgress>,
    progress: LifecycleProgress,
) {
    if tx.send(progress).await.is_err() {
        warn!("progress channel closed, receiver dropped");
    }
}

fn build_steps(request: &LifecycleRequest) -> Vec<LifecycleStep> {
    let workspace_step = LifecycleStep {
        name: StepName("workspace-prepare".to_owned()),
        effect: Effect::WorkspacePrepare {
            workspace: WorkspaceName(format!("workspace-{}", request.bead_id.as_str())),
            path: WorkspacePath(format!("../{}", request.bead_id.as_str())),
        },
    };
    match request.prompt.as_ref().filter(|p| !p.0.is_empty()) {
        Some(p) => vec![
            workspace_step,
            build_opencode_step(&request.bead_id, p, request.model.as_ref()),
        ],
        None => vec![workspace_step],
    }
}

fn build_opencode_step(
    bead_id: &BeadId,
    prompt: &PromptString,
    model: Option<&ModelId>,
) -> LifecycleStep {
    let resolved_model = match model {
        Some(m) => m.clone(),
        None => ModelId(DEFAULT_MODEL.to_owned()),
    };
    LifecycleStep {
        name: StepName("opencode-run".to_owned()),
        effect: Effect::Opencode {
            prompt: prompt.clone(),
            model: resolved_model,
            cwd: Some(WorkspacePath(format!("../{}", bead_id.as_str()))),
        },
    }
}

async fn persist_failure(db: &StateDb, failure: &StepFailure) -> LifecycleError {
    let failed_state = failure
        .partial_run
        .workflow_state
        .clone()
        .with_transition(StateEvent::Failed(ErrorMessage(failure.error.to_string())));
    match failed_state {
        Ok(state) => {
            if let Err(e) = persist_state(db, &state, &failure.partial_run.journal) {
                error!("failed to persist failed state: {e}");
            }
        }
        Err(e) => error!("failed to transition to Failed phase: {e}"),
    }
    failure.error.clone()
}

async fn run_lifecycle_inner(
    db: StateDb,
    executor: TokioCommandExecutor,
    server_config: Option<OpencodeServerConfig>,
    request: LifecycleRequest,
    tx: mpsc::Sender<LifecycleProgress>,
) -> std::result::Result<(), LifecycleError> {
    let bead_id = request.bead_id.clone();
    let steps = build_steps(&request);
    let state = WorkflowState::new(bead_id.clone());
    send_initialized(&tx, &bead_id, &steps).await;
    let run = match execute_steps(executor, &steps, state, &tx, server_config.as_ref()).await {
        Ok(r) => r,
        Err(failure) => return Err(persist_failure(&db, &failure).await),
    };
    finish_lifecycle(&db, bead_id, run, &tx).await
}

async fn send_initialized(
    tx: &mpsc::Sender<LifecycleProgress>,
    bead_id: &BeadId,
    steps: &[LifecycleStep],
) {
    send_progress(
        tx,
        LifecycleProgress::Initialized {
            bead_id: bead_id.clone(),
            steps: steps.iter().map(|s| s.name.clone()).collect(),
        },
    )
    .await;
}

async fn finish_lifecycle(
    db: &StateDb,
    bead_id: BeadId,
    run: StepRun,
    tx: &mpsc::Sender<LifecycleProgress>,
) -> std::result::Result<(), LifecycleError> {
    send_finished_success(tx).await;
    let completed_state = transition_completed(run.workflow_state)?;
    persist_completed_state(db, &completed_state, &run.journal)?;
    info!("lifecycle completed for {:?}", bead_id);
    Ok(())
}

async fn send_finished_success(tx: &mpsc::Sender<LifecycleProgress>) {
    send_progress(
        tx,
        LifecycleProgress::Finished {
            result: StepResult::Success,
            message: Some(ErrorMessage::from("all steps completed")),
        },
    )
    .await;
}

fn transition_completed(
    state: WorkflowState,
) -> std::result::Result<WorkflowState, LifecycleError> {
    state
        .with_transition(StateEvent::Completed(StepResult::Success))
        .map_err(|e| {
            LifecycleError::terminal(
                crate::lifecycle::error::FailureCategory::Validation,
                format!("failed to transition to Completed: {e}"),
            )
        })
}

fn persist_completed_state(
    db: &StateDb,
    state: &WorkflowState,
    journal: &[EffectJournalEntry],
) -> std::result::Result<(), LifecycleError> {
    persist_state(db, state, journal).map_err(|e| {
        LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Workspace,
            e.to_string(),
        )
    })
}

// ─── TESTS ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{
        BeadId, LifecycleRequest, ModelId, Phase, PromptString, RepoUrl,
    };

    // ── build_steps ──

    #[test]
    fn build_steps_no_prompt_returns_workspace_prepare_only() {
        let request = LifecycleRequest {
            bead_id: BeadId::parse("test").unwrap(),
            model: None,
            repo: None,
            prompt: None,
        };
        let steps = build_steps(&request);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].name.as_str(), "workspace-prepare");
    }

    #[test]
    fn build_steps_with_prompt_returns_workspace_prepare_and_opencode() {
        let request = LifecycleRequest {
            bead_id: BeadId::parse("test").unwrap(),
            model: Some(ModelId("gpt-4".into())),
            repo: Some(RepoUrl("https://github.com/test/repo".into())),
            prompt: Some(PromptString("fix it".into())),
        };
        let steps = build_steps(&request);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name.as_str(), "workspace-prepare");
        assert_eq!(steps[1].name.as_str(), "opencode-run");
    }

    #[test]
    fn build_steps_with_empty_prompt_returns_only_workspace_prepare() {
        let request = LifecycleRequest {
            bead_id: BeadId::parse("test").unwrap(),
            model: None,
            repo: None,
            prompt: Some(PromptString("".into())),
        };
        let steps = build_steps(&request);
        assert_eq!(steps.len(), 1);
    }

    // ── build_opencode_step ──

    #[test]
    fn build_opencode_step_uses_provided_model() {
        let bead_id = BeadId::parse("my-bean").unwrap();
        let prompt = PromptString("hello".into());
        let model = ModelId("claude-3.5".into());
        let step = build_opencode_step(&bead_id, &prompt, Some(&model));
        match step.effect {
            Effect::Opencode { prompt: p, model: m, cwd } => {
                assert_eq!(p.as_str(), "hello");
                assert_eq!(m.as_str(), "claude-3.5");
                assert!(cwd.is_some());
            }
            _ => panic!("expected Opencode effect"),
        }
    }

    #[test]
    fn build_opencode_step_defaults_model_when_none() {
        let bead_id = BeadId::parse("my-bean").unwrap();
        let step = build_opencode_step(&bead_id, &PromptString("hi".into()), None);
        match step.effect {
            Effect::Opencode { model, .. } => {
                assert_eq!(model.as_str(), DEFAULT_MODEL);
            }
            _ => panic!("expected Opencode effect"),
        }
    }

    #[test]
    fn build_opencode_step_cwd_contains_bead_id() {
        let bead_id = BeadId::parse("my-bean").unwrap();
        let step = build_opencode_step(&bead_id, &PromptString("hi".into()), None);
        match step.effect {
            Effect::Opencode { cwd, .. } => {
                assert!(cwd.unwrap().as_str().contains("my-bean"));
            }
            _ => panic!("expected Opencode effect"),
        }
    }

    // ── transition_completed ──

    #[test]
    fn transition_completed_transitions_to_completed_phase() {
        let id = BeadId::parse("comp-test").unwrap();
        let state = WorkflowState::new(id)
            .with_transition(StateEvent::WorkspaceReady)
            .unwrap()
            .with_transition(StateEvent::StepStarted("s".into()))
            .unwrap();
        let result = transition_completed(state);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().phase, Phase::Completed { .. }));
    }

    #[test]
    fn transition_completed_fails_from_planned_phase() {
        let id = BeadId::parse("fail-test").unwrap();
        let state = WorkflowState::new(id);
        let result = transition_completed(state);
        assert!(result.is_err());
    }

    // ── build_steps edge cases ──

    #[test]
    fn build_steps_empty_string_prompt_returns_only_workspace_prepare() {
        // Empty string should be treated as no prompt (filter removes empty)
        let request = LifecycleRequest {
            bead_id: BeadId::parse("empty-test").unwrap(),
            model: Some(ModelId("gpt-4".into())),
            repo: None,
            prompt: Some(PromptString("".into())),
        };
        let steps = build_steps(&request);
        assert_eq!(steps.len(), 1, "empty string prompt should produce only workspace-prepare");
        assert_eq!(steps[0].name.as_str(), "workspace-prepare");
    }

    #[test]
    fn build_steps_whitespace_prompt_returns_two_steps() {
        // Whitespace-only is NOT empty after trim... but the filter checks is_empty() on raw string
        // So "   " (spaces) is NOT empty, it has chars. This tests the is_empty() boundary.
        let request = LifecycleRequest {
            bead_id: BeadId::parse("ws-test").unwrap(),
            model: None,
            repo: None,
            prompt: Some(PromptString("   ".into())), // 3 spaces, not empty
        };
        let steps = build_steps(&request);
        // filter checks p.0.is_empty() which is false for "   ", so it includes opencode step
        assert_eq!(steps.len(), 2, "whitespace prompt should produce both steps");
    }

    // ── LifecycleConfig ──

    #[test]
    fn lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.data_dir.as_str(), ".oya-lite");
        assert!(config.opencode_server.is_none());
    }

    #[test]
    fn lifecycle_config_with_server() {
        use crate::lifecycle::types::{OpencodeServerConfig, OpencodeUrl, SensitiveString, Username};
        let config = LifecycleConfig {
            data_dir: crate::lifecycle::types::DataDirPath("/data".into()),
            opencode_server: Some(OpencodeServerConfig {
                url: OpencodeUrl("http://localhost:4099".into()),
                username: Username("user".into()),
                password: SensitiveString("pass".into()),
            }),
        };
        assert_eq!(config.data_dir.as_str(), "/data");
        assert!(config.opencode_server.is_some());
    }

    // ── LifecycleRequest ──

    #[test]
    fn lifecycle_request_all_fields() {
        let request = LifecycleRequest {
            bead_id: BeadId::parse("full-req").unwrap(),
            model: Some(ModelId("claude-3".into())),
            repo: Some(RepoUrl("https://github.com/test/repo".into())),
            prompt: Some(PromptString("do the thing".into())),
        };
        assert_eq!(request.bead_id.as_str(), "full-req");
        assert!(request.model.is_some());
        assert!(request.repo.is_some());
        assert!(request.prompt.is_some());
    }

    #[test]
    fn lifecycle_request_minimal() {
        let request = LifecycleRequest {
            bead_id: BeadId::parse("min-req").unwrap(),
            model: None,
            repo: None,
            prompt: None,
        };
        assert!(request.model.is_none());
        assert!(request.repo.is_none());
        assert!(request.prompt.is_none());
    }

    // ── Tests to kill mutation survivors ──

    #[tokio::test]
    async fn finish_lifecycle_persists_state() {
        // Targets: persist_completed_state → Ok(()) and finish_lifecycle → Ok(())
        // If either function returns Ok(()) without actually persisting,
        // load_state will return None and this test fails.
        use crate::lifecycle::state::load_state;

        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let id = BeadId::parse("fin-lf").unwrap();
        let state = WorkflowState::new(id.clone())
            .with_transition(StateEvent::WorkspaceReady)
            .unwrap()
            .with_advanced_step(StepName("s1".into()));

        let run = StepRun {
            workflow_state: state,
            journal: vec![],
        };

        let (tx, _rx) = tokio::sync::mpsc::channel(10);

        // finish_lifecycle calls persist_completed_state which calls persist_state
        let result = finish_lifecycle(&db, id.clone(), run, &tx).await;
        assert!(result.is_ok(), "finish_lifecycle should succeed");

        // Verify the state was actually persisted (not swallowed by Ok(()) mutation)
        let loaded = load_state(&db, &id).unwrap();
        assert!(loaded.is_some(), "finish_lifecycle must persist state to database");
        let (loaded_state, _) = loaded.unwrap();
        assert!(matches!(loaded_state.phase, Phase::Completed { .. }));
    }

    #[tokio::test]
    async fn persist_completed_state_writes_to_db() {
        // Targets: persist_completed_state → Ok(()) mutation
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();

        let id = BeadId::parse("pcs-test").unwrap();
        let state = WorkflowState::new(id.clone())
            .with_transition(StateEvent::WorkspaceReady)
            .unwrap()
            .with_transition(StateEvent::Completed(StepResult::Success))
            .unwrap();

        persist_completed_state(&db, &state, &[]).unwrap();

        let loaded = load_state(&db, &id).unwrap();
        assert!(loaded.is_some(), "persist_completed_state must write to database");
    }

    #[allow(clippy::manual_unwrap_or_default)]
    #[tokio::test]
    async fn send_initialized_sends_progress() {
        // Targets: send_initialized → () mutation
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let id = BeadId::parse("si-test").unwrap();
        send_initialized(&tx, &id, &[]).await;

        let progress = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            rx.recv(),
        )
        .await;
        let progress = match progress {
            Ok(p) => p,
            Err(_) => None,
        };
        assert!(progress.is_some(), "send_initialized must actually send progress");
        match progress.unwrap() {
            LifecycleProgress::Initialized { bead_id, steps } => {
                assert_eq!(bead_id, id);
                assert!(steps.is_empty());
            }
            _ => panic!("expected Initialized progress"),
        }
    }

    #[allow(clippy::manual_unwrap_or_default)]
    #[tokio::test]
    async fn send_progress_sends_message() {
        // Targets: send_progress → () mutation
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        send_progress(
            &tx,
            LifecycleProgress::Finished {
                result: StepResult::Success,
                message: None,
            },
        )
        .await;

        let progress = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            rx.recv(),
        )
        .await;
        let progress = match progress {
            Ok(p) => p,
            Err(_) => None,
        };
        assert!(progress.is_some(), "send_progress must actually send progress");
    }

    #[tokio::test]
    async fn get_workflow_state_returns_persisted_state() {
        // Targets: get_workflow_state → Ok(None) mutation
        use crate::lifecycle::state::persist_state;

        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let id = BeadId::parse("gws-test").unwrap();
        let state = WorkflowState::new(id.clone())
            .with_transition(StateEvent::WorkspaceReady)
            .unwrap()
            .with_advanced_step(StepName("s1".into()));

        persist_state(&db, &state, &[]).unwrap();

        let orchestrator = LifecycleOrchestrator {
            db,
            executor: TokioCommandExecutor::new(),
            server_config: None,
        };

        let result = orchestrator.get_workflow_state(&id);
        assert!(
            result.is_ok(),
            "get_workflow_state should return Ok when state exists"
        );
        let workflow_state = result.unwrap();
        assert!(
            workflow_state.is_some(),
            "get_workflow_state must return the persisted state, not Ok(None)"
        );
        assert_eq!(workflow_state.unwrap().completed_steps.len(), 1);
    }

    #[tokio::test]
    async fn run_lifecycle_inner_persists_state() {
        // Targets: run_lifecycle_inner → Ok(()) mutation
        // If the function returns Ok(()) without executing steps and finishing,
        // load_state will return None and this test fails.
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("db");

        let id = BeadId::parse("rli-test").unwrap();
        let request = LifecycleRequest {
            bead_id: id.clone(),
            model: None,
            repo: None,
            prompt: None,
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        // Spawn a background task to drain the channel so send operations don't block
        let drain = tokio::spawn(async move {
            while rx.recv().await.is_some() {}
        });

        let orchestrator = LifecycleOrchestrator::new(LifecycleConfig {
            data_dir: crate::lifecycle::types::DataDirPath(db_path.to_string_lossy().to_string()),
            opencode_server: None,
        })
        .unwrap();

        // Extract the fields for direct call to run_lifecycle_inner
        // We need to move them because run_lifecycle_inner takes ownership
        let db = orchestrator.db;
        let executor = orchestrator.executor;
        let server_config = orchestrator.server_config;

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            run_lifecycle_inner(db, executor, server_config, request, tx),
        )
        .await;

        // The test should complete within the timeout
        assert!(result.is_ok(), "run_lifecycle_inner should complete within 10s");

        // Re-open the DB to check persisted state
        let db2 = StateDb::open(db_path).unwrap();
        let workflow_state = load_state(&db2, &id)
            .unwrap()
            .map(|(s, _)| s)
            .expect("run_lifecycle_inner must persist state to database");

        assert!(
            matches!(workflow_state.phase, Phase::Completed { .. }),
            "state should be Completed after run_lifecycle_inner"
        );

        let _ = drain.await;
    }

    #[tokio::test]
    async fn send_finished_success_sends_finished_message() {
        // Targets: send_finished_success → () mutation
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        send_finished_success(&tx).await;

        let progress = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            rx.recv(),
        )
        .await;
        let progress = progress.unwrap_or_default();
        assert!(
            progress.is_some(),
            "send_finished_success must actually send a progress message"
        );
        match progress.unwrap() {
            LifecycleProgress::Finished { result, message } => {
                assert_eq!(result, StepResult::Success);
                assert!(
                    message.is_some(),
                    "Finished message must include a message field"
                );
            }
            _ => panic!("expected Finished progress from send_finished_success"),
        }
    }

    #[test]
    fn flush_persists_data_to_disk() {
        // Targets: flush → Ok(()) mutation
        // When flush is replaced with Ok(()), data stays in memory but is NOT written to disk.
        // Opening a fresh DB should NOT see the data.
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();

        let id = BeadId::parse("flush-test").unwrap();
        let state = WorkflowState::new(id.clone())
            .with_transition(StateEvent::WorkspaceReady)
            .unwrap();
        persist_state(&db, &state, &[]).unwrap();

        // Drop the DB to release any in-memory buffers
        drop(db);

        // Re-open the DB - flush should have written data to disk
        let db2 = StateDb::open(dir.path().join("db")).unwrap();
        let loaded = load_state(&db2, &id)
            .unwrap()
            .map(|(s, _)| s);

        assert!(
            loaded.is_some(),
            "flush must persist data to disk - re-opened DB should see the state"
        );
    }
}
