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
    LifecycleStep, ModelId, OpencodeServerConfig, PromptString, StateEvent, StepName, StepResult,
    WorkflowState, WorkspaceName, WorkspacePath,
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
