use super::opencode_server::run_opencode_server;
use super::send_progress;
use crate::lifecycle::effects::run::opencode_output_is_error;
use crate::lifecycle::effects::{run_effect, TokioCommandExecutor};
use crate::lifecycle::error::LifecycleError;
use crate::lifecycle::types::{
    EffectJournalEntry, ErrorMessage, LifecycleProgress, LifecycleStep, OpencodeServerConfig,
    StateEvent, StepName, Timestamp, WorkflowState,
};
use tokio::sync::mpsc;

pub(super) struct StepRun {
    pub(super) workflow_state: WorkflowState,
    pub(super) journal: Vec<EffectJournalEntry>,
}

pub(super) struct StepFailure {
    pub(super) partial_run: StepRun,
    pub(super) error: LifecycleError,
}

pub(super) async fn execute_steps(
    executor: TokioCommandExecutor,
    steps: &[LifecycleStep],
    initial_state: WorkflowState,
    tx: &mpsc::Sender<LifecycleProgress>,
    server_config: Option<&OpencodeServerConfig>,
) -> std::result::Result<StepRun, StepFailure> {
    let mut run = StepRun {
        workflow_state: initial_state,
        journal: Vec::new(),
    };
    for step in steps {
        run = execute_single_step(executor, step, run, tx, server_config).await?;
    }
    Ok(run)
}

async fn handle_step_transition(
    run: StepRun,
    step_name: &StepName,
    entry: &EffectJournalEntry,
) -> Result<StepRun, StepFailure> {
    if !entry.result.is_success() {
        return Ok(run);
    }
    let Some(event) = workspace_ready_event(step_name) else {
        return Ok(run);
    };
    let StepRun {
        workflow_state,
        journal,
    } = run;
    let prev_state = workflow_state.clone();
    match workflow_state.with_transition(event) {
        Ok(state) => Ok(StepRun {
            workflow_state: state,
            journal,
        }),
        Err(error) => Err(step_transition_failure(prev_state, journal, error)),
    }
}

fn workspace_ready_event(step_name: &StepName) -> Option<StateEvent> {
    match step_name.as_str() {
        "workspace-prepare" => Some(StateEvent::WorkspaceReady),
        _ => None,
    }
}

fn step_transition_failure(
    workflow_state: WorkflowState,
    journal: Vec<EffectJournalEntry>,
    error: LifecycleError,
) -> StepFailure {
    StepFailure {
        partial_run: StepRun {
            workflow_state,
            journal,
        },
        error,
    }
}

async fn execute_single_step(
    executor: TokioCommandExecutor,
    step: &LifecycleStep,
    mut run: StepRun,
    tx: &mpsc::Sender<LifecycleProgress>,
    server_config: Option<&OpencodeServerConfig>,
) -> std::result::Result<StepRun, StepFailure> {
    let step_name = step.name.clone();
    run = start_step(run, &step_name, tx).await?;
    let entry = match dispatch_effect(executor, &step.effect, server_config).await {
        Ok(entry) => entry,
        Err(error) => return Err(on_dispatch_failure(run, &step_name, tx, error).await),
    };
    finish_step(run, entry, &step_name, tx).await
}

async fn start_step(
    mut run: StepRun,
    step_name: &StepName,
    tx: &mpsc::Sender<LifecycleProgress>,
) -> std::result::Result<StepRun, StepFailure> {
    run.workflow_state = run
        .workflow_state
        .clone()
        .with_transition(StateEvent::StepStarted(step_name.clone()))
        .map_err(|e| to_step_failure(&run, e))?;
    send_step_started(tx, step_name).await;
    Ok(run)
}

async fn send_step_started(tx: &mpsc::Sender<LifecycleProgress>, step_name: &StepName) {
    send_progress(
        tx,
        LifecycleProgress::StepStarted {
            step: step_name.clone(),
            started_at: Timestamp(chrono::Utc::now().to_rfc3339()),
        },
    )
    .await;
}

async fn finish_step(
    run: StepRun,
    entry: EffectJournalEntry,
    step_name: &StepName,
    tx: &mpsc::Sender<LifecycleProgress>,
) -> std::result::Result<StepRun, StepFailure> {
    let transitioned_run = handle_step_transition(run, step_name, &entry).await?;
    if entry.result.is_success() {
        Ok(on_step_success(transitioned_run, entry, step_name, tx).await)
    } else {
        Err(on_step_failure(transitioned_run, entry, step_name, tx).await)
    }
}

fn to_step_failure(run: &StepRun, error: LifecycleError) -> StepFailure {
    StepFailure {
        partial_run: StepRun {
            workflow_state: run.workflow_state.clone(),
            journal: run.journal.clone(),
        },
        error,
    }
}

async fn on_dispatch_failure(
    run: StepRun,
    step_name: &StepName,
    tx: &mpsc::Sender<LifecycleProgress>,
    error: LifecycleError,
) -> StepFailure {
    let message = ErrorMessage(error.to_string());
    send_progress(
        tx,
        LifecycleProgress::StepFailed {
            step: step_name.clone(),
            error: message,
        },
    )
    .await;
    StepFailure {
        partial_run: run,
        error,
    }
}

async fn dispatch_effect(
    executor: TokioCommandExecutor,
    effect: &crate::lifecycle::types::Effect,
    server_config: Option<&OpencodeServerConfig>,
) -> std::result::Result<EffectJournalEntry, LifecycleError> {
    let cwd = effect.cwd().map(|p| p.0.clone());
    match (effect, server_config) {
        (crate::lifecycle::types::Effect::Opencode { .. }, Some(cfg)) => {
            run_opencode_server(cfg, effect).await
        }
        _ => run_effect(&executor, effect.clone(), cwd).await,
    }
}

fn append_to_journal(
    journal: Vec<EffectJournalEntry>,
    entry: EffectJournalEntry,
) -> Vec<EffectJournalEntry> {
    journal.into_iter().chain(std::iter::once(entry)).collect()
}

async fn on_step_success(
    run: StepRun,
    entry: EffectJournalEntry,
    step_name: &StepName,
    tx: &mpsc::Sender<LifecycleProgress>,
) -> StepRun {
    let new_state = run.workflow_state.with_advanced_step(step_name.clone());
    send_progress(
        tx,
        LifecycleProgress::StepCompleted {
            step: step_name.clone(),
            duration_ms: 0,
        },
    )
    .await;
    StepRun {
        workflow_state: new_state,
        journal: append_to_journal(run.journal, entry),
    }
}

async fn on_step_failure(
    run: StepRun,
    entry: EffectJournalEntry,
    step_name: &StepName,
    tx: &mpsc::Sender<LifecycleProgress>,
) -> StepFailure {
    let failure_message = sanitize_failure_message(&entry);
    let journal = append_to_journal(run.journal, entry);
    send_step_failed(tx, step_name, &failure_message).await;
    build_step_failure(run.workflow_state, journal, failure_message)
}

async fn send_step_failed(
    tx: &mpsc::Sender<LifecycleProgress>,
    step_name: &StepName,
    message: &str,
) {
    send_progress(
        tx,
        LifecycleProgress::StepFailed {
            step: step_name.clone(),
            error: ErrorMessage(message.to_owned()),
        },
    )
    .await;
}

fn build_step_failure(
    workflow_state: WorkflowState,
    journal: Vec<EffectJournalEntry>,
    failure_message: String,
) -> StepFailure {
    StepFailure {
        partial_run: StepRun {
            workflow_state,
            journal,
        },
        error: LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Command,
            failure_message,
        ),
    }
}

fn sanitize_failure_message(entry: &EffectJournalEntry) -> String {
    if opencode_output_is_error(&entry.stdout, &entry.stderr) {
        return "opencode model not found or unavailable".to_owned();
    }
    match first_nonempty_line(&entry.stderr).or_else(|| first_nonempty_line(&entry.stdout)) {
        Some(line) => line.to_owned(),
        None => "command failed without diagnostic output".to_owned(),
    }
}

fn first_nonempty_line(text: &str) -> Option<&str> {
    text.lines().find(|line| !line.trim().is_empty())
}

// ─── TESTS ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{
        BeadId, Effect, EffectJournalEntry, StateEvent, StepName, StepResult, WorkflowState,
        WorkspaceName, WorkspacePath,
    };

    fn make_entry(stdout: &str, stderr: &str, success: bool) -> EffectJournalEntry {
        EffectJournalEntry {
            effect: Effect::WorkspacePrepare {
                workspace: WorkspaceName("w".into()),
                path: WorkspacePath("/tmp".into()),
            },
            timeout_secs: 30,
            result: if success { StepResult::Success } else { StepResult::Failure },
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    // ── workspace_ready_event ──

    #[test]
    fn workspace_ready_event_workspace_prepare_returns_workspace_ready() {
        let step = StepName("workspace-prepare".into());
        let event = workspace_ready_event(&step);
        assert!(matches!(event, Some(StateEvent::WorkspaceReady)));
    }

    #[test]
    fn workspace_ready_event_other_step_returns_none() {
        for name in ["opencode-run", "moon-ci", "jj"] {
            let step = StepName(name.into());
            assert!(workspace_ready_event(&step).is_none(), "expected None for {name}");
        }
    }

    // ── step_transition_failure ──

    #[test]
    fn step_transition_failure_contains_state_and_error() {
        let state = WorkflowState::new(BeadId::parse("fail-test").unwrap());
        let journal = vec![];
        let err = LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Command,
            "boom",
        );
        let failure = step_transition_failure(state.clone(), journal.clone(), err.clone());
        assert!(failure.error.is_terminal());
        assert_eq!(failure.partial_run.workflow_state.phase, state.phase);
        assert_eq!(failure.partial_run.journal.len(), 0);
    }

    // ── append_to_journal ──

    #[test]
    fn append_to_journal_adds_entry() {
        let entry = make_entry("out", "", true);
        let journal: Vec<EffectJournalEntry> = vec![];
        let result = append_to_journal(journal, entry.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].stdout, "out");
    }

    #[test]
    fn append_to_journal_preserves_existing() {
        let e1 = make_entry("first", "", true);
        let e2 = make_entry("second", "", true);
        let journal = vec![e1];
        let result = append_to_journal(journal, e2);
        assert_eq!(result.len(), 2);
    }

    // ── sanitize_failure_message ──

    #[test]
    fn sanitize_failure_message_opencode_error_returns_sanitized() {
        let entry = make_entry(r#"{"type":"error","message":"model not found"}"#, "", false);
        let msg = sanitize_failure_message(&entry);
        assert_eq!(msg, "opencode model not found or unavailable");
    }

    #[test]
    fn sanitize_failure_message_stderr_line_returns_stderr() {
        let entry = make_entry("", "actual error\nline2\n", false);
        let msg = sanitize_failure_message(&entry);
        assert_eq!(msg, "actual error");
    }

    #[test]
    fn sanitize_failure_message_stdout_line_when_stderr_empty() {
        let entry = make_entry("stdout error\n", "", false);
        let msg = sanitize_failure_message(&entry);
        assert_eq!(msg, "stdout error");
    }

    #[test]
    fn sanitize_failure_message_empty_returns_default() {
        let entry = make_entry("", "", false);
        let msg = sanitize_failure_message(&entry);
        assert_eq!(msg, "command failed without diagnostic output");
    }

    #[test]
    fn sanitize_failure_message_first_real_line_returned() {
        let entry = make_entry("", "error line\nanother line\n", false);
        let msg = sanitize_failure_message(&entry);
        assert_eq!(msg, "error line");
    }

    // ── first_nonempty_line ──

    #[test]
    fn first_nonempty_line_finds_line() {
        assert_eq!(first_nonempty_line("hello\nworld\n"), Some("hello"));
        assert_eq!(first_nonempty_line("  spaced  \nnext\n"), Some("  spaced  "));
    }

    #[test]
    fn first_nonempty_line_skips_empty_lines() {
        assert_eq!(first_nonempty_line("\n  \nhello\n"), Some("hello"));
    }

    #[test]
    fn first_nonempty_line_none_when_all_empty() {
        assert!(first_nonempty_line("").is_none());
        assert!(first_nonempty_line("  \n  \n").is_none());
    }

    #[test]
    fn first_nonempty_line_only_whitespace() {
        assert!(first_nonempty_line("   ").is_none());
        assert!(first_nonempty_line(" \t\n ").is_none());
    }

    #[test]
    fn first_nonempty_line_newline_first() {
        assert_eq!(first_nonempty_line("\nhello"), Some("hello"));
    }

    #[test]
    fn first_nonempty_line_single_char() {
        assert_eq!(first_nonempty_line("x"), Some("x"));
    }

    // ── to_step_failure ──

    #[test]
    fn to_step_failure_copies_run_state() {
        let run = StepRun {
            workflow_state: WorkflowState::new(BeadId::parse("t").unwrap()),
            journal: vec![],
        };
        let err = LifecycleError::transient(
            crate::lifecycle::error::FailureCategory::Command,
            "transient error",
        );
        let failure = to_step_failure(&run, err.clone());
        assert!(!failure.error.is_terminal());
        assert_eq!(failure.partial_run.workflow_state.phase, run.workflow_state.phase);
    }

    // ── build_step_failure ──

    #[test]
    fn build_step_failure_returns_terminal_error() {
        let state = WorkflowState::new(BeadId::parse("t").unwrap());
        let failure = build_step_failure(state, vec![], "test failure".into());
        assert!(failure.error.is_terminal());
        assert_eq!(failure.error.category(), crate::lifecycle::error::FailureCategory::Command);
    }
}
