#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_lite::lifecycle::types::*;
use oya_lite::lifecycle::error::LifecycleError;

fn parse_bead_id(s: &str) -> Result<BeadId, Box<dyn std::error::Error>> {
    BeadId::parse(s).map_err(|e: BeadIdParseError| e.into())
}

fn workflow_to_ws_ready(id: BeadId) -> Result<WorkflowState, LifecycleError> {
    WorkflowState::new(id)
        .with_transition(StateEvent::WorkspaceReady)
}

#[test]
fn planned_phase_has_bead_data() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("phase-test")?;
    let state = WorkflowState::new(id.clone());
    match state.phase {
        Phase::Planned { bead } => {
            assert_eq!(bead.bead_id, id);
        }
        _ => {
            return Err("expected Planned phase".into());
        }
    }
    Ok(())
}

#[test]
fn completed_phase_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("completed-test")?;
    let state = workflow_to_ws_ready(id)?
        .with_transition(StateEvent::StepStarted("s".into()))?
        .with_transition(StateEvent::Completed(StepResult::Success))?;
    assert!(state.phase.is_terminal());
    assert!(matches!(state.phase, Phase::Completed { result: StepResult::Success, .. }));
    Ok(())
}

#[test]
fn failed_phase_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("failed-test")?;
    let state = WorkflowState::new(id)
        .with_transition(StateEvent::Failed("error".into()))?;
    assert!(state.phase.is_terminal());
    assert!(matches!(state.phase, Phase::Failed { .. }));
    Ok(())
}

#[test]
fn workspace_ready_phase_bead_id() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("ws-ready-test")?;
    let state = workflow_to_ws_ready(id.clone())?;
    assert_eq!(state.phase.bead_id(), &id);
    Ok(())
}

#[test]
fn executing_phase_bead_id_and_step() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("executing-test")?;
    let step_name = StepName("my-step".into());
    let state = workflow_to_ws_ready(id.clone())?
        .with_transition(StateEvent::StepStarted(step_name.clone()))?;
    assert_eq!(state.phase.bead_id(), &id);
    match state.phase {
        Phase::Executing { step, .. } => assert_eq!(step, step_name),
        _ => {
            return Err("expected Executing phase".into());
        }
    }
    Ok(())
}

#[test]
fn completed_phase_bead_id_and_result() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("completed-result-test")?;
    let state = workflow_to_ws_ready(id.clone())?
        .with_transition(StateEvent::Completed(StepResult::Success))?;
    assert_eq!(state.phase.bead_id(), &id);
    match state.phase {
        Phase::Completed { result, .. } => assert_eq!(result, StepResult::Success),
        _ => {
            return Err("expected Completed phase".into());
        }
    }
    Ok(())
}

#[test]
fn failed_phase_bead_id_and_error() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("failed-result-test")?;
    let error_msg = ErrorMessage("something went wrong".into());
    let state = WorkflowState::new(id.clone())
        .with_transition(StateEvent::Failed(error_msg.clone()))?;
    assert_eq!(state.phase.bead_id(), &id);
    match state.phase {
        Phase::Failed { error, .. } => assert_eq!(error, error_msg),
        _ => {
            return Err("expected Failed phase".into());
        }
    }
    Ok(())
}

#[test]
fn step_result_is_success() {
    assert!(StepResult::Success.is_success());
    assert!(!StepResult::Failure.is_success());
}

#[test]
fn step_deps_smallvec_capacity() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("smallvec-cap")?;
    let state = WorkflowState::new(id);
    let capacity = state.completed_steps.capacity();
    assert!(capacity >= 4);
    Ok(())
}

#[test]
fn workflow_state_phase_json_serde() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("serde-test")?;
    let state = WorkflowState::new(id);
    let json = serde_json::to_string(&state.phase)?;
    assert!(json.contains("planned"));
    let back: Phase = serde_json::from_str(&json)?;
    assert!(matches!(back, Phase::Planned { .. }));
    Ok(())
}

#[test]
fn workflow_state_full_json_serde() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("full-serde-test")?;
    let state = WorkflowState::new(id);
    let json = serde_json::to_string(&state)?;
    let back: WorkflowState = serde_json::from_str(&json)?;
    assert_eq!(state.phase, back.phase);
    assert_eq!(state.completed_steps.len(), back.completed_steps.len());
    Ok(())
}

#[test]
fn state_event_debug_workspace_ready() {
    let event = StateEvent::WorkspaceReady;
    let s = format!("{:?}", event);
    assert!(s.contains("WorkspaceReady"));
}

#[test]
fn state_event_debug_step_started() {
    let event = StateEvent::StepStarted("my-step".into());
    let s = format!("{:?}", event);
    assert!(s.contains("StepStarted"));
    assert!(s.contains("my-step"));
}

#[test]
fn state_event_debug_completed() {
    let event = StateEvent::Completed(StepResult::Success);
    let s = format!("{:?}", event);
    assert!(s.contains("Completed"));
}

#[test]
fn state_event_debug_failed() {
    let event = StateEvent::Failed("error msg".into());
    let s = format!("{:?}", event);
    assert!(s.contains("Failed"));
    assert!(s.contains("error msg"));
}

#[test]
fn phase_debug_planned() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("debug-test")?;
    let phase = Phase::Planned { bead: BeadData::from_bead_id(id) };
    let s = format!("{:?}", phase);
    assert!(s.contains("Planned"));
    Ok(())
}

#[test]
fn phase_debug_workspace_ready() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("debug-test")?;
    let phase = Phase::WorkspaceReady { bead: BeadData::from_bead_id(id) };
    let s = format!("{:?}", phase);
    assert!(s.contains("WorkspaceReady"));
    Ok(())
}

#[test]
fn phase_debug_executing() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("debug-test")?;
    let phase = Phase::Executing { bead: BeadData::from_bead_id(id), step: "step-1".into() };
    let s = format!("{:?}", phase);
    assert!(s.contains("Executing"));
    assert!(s.contains("step-1"));
    Ok(())
}

#[test]
fn phase_debug_completed() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("debug-test")?;
    let phase = Phase::Completed { bead: BeadData::from_bead_id(id), result: StepResult::Success };
    let s = format!("{:?}", phase);
    assert!(s.contains("Completed"));
    assert!(s.contains("Success"));
    Ok(())
}

#[test]
fn phase_debug_failed() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("debug-test")?;
    let phase = Phase::Failed { bead_id: id, error: "failed".into() };
    let s = format!("{:?}", phase);
    assert!(s.contains("Failed"));
    assert!(s.contains("failed"));
    Ok(())
}

#[test]
fn effect_journal_entry_debug() -> Result<(), Box<dyn std::error::Error>> {
    let entry = EffectJournalEntry {
        effect: Effect::WorkspacePrepare { workspace: "ws".into(), path: "/tmp".into() },
        timeout_secs: 3600,
        result: StepResult::Success,
        stdout: "out".into(),
        stderr: "err".into(),
    };
    let s = format!("{:?}", entry);
    assert!(s.contains("WorkspacePrepare"));
    assert!(s.contains("3600"));
    Ok(())
}

#[test]
fn effect_journal_entry_serde() -> Result<(), Box<dyn std::error::Error>> {
    let entry = EffectJournalEntry {
        effect: Effect::MoonRun { task: "build".into(), cwd: None },
        timeout_secs: 1800,
        result: StepResult::Failure,
        stdout: "test output".into(),
        stderr: "".into(),
    };
    let json = serde_json::to_string(&entry)?;
    let back: EffectJournalEntry = serde_json::from_str(&json)?;
    assert_eq!(entry.timeout_secs, back.timeout_secs);
    Ok(())
}

// ── Tests to kill mutation survivors ──

#[test]
fn non_terminal_phases_are_not_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("not-term")?;
    let planned = Phase::Planned { bead: BeadData::from_bead_id(id.clone()) };
    assert!(!planned.is_terminal());
    let ws_ready = Phase::WorkspaceReady { bead: BeadData::from_bead_id(id.clone()) };
    assert!(!ws_ready.is_terminal());
    let executing = Phase::Executing { bead: BeadData::from_bead_id(id), step: "s".into() };
    assert!(!executing.is_terminal());
    Ok(())
}

#[test]
fn executing_plus_workspace_ready_transitions_to_workspace_ready() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("exec-ws")?;
    let state = WorkflowState::new(id.clone())
        .with_transition(StateEvent::WorkspaceReady)?
        .with_transition(StateEvent::StepStarted("s".into()))?;
    assert!(matches!(state.phase, Phase::Executing { .. }));
    let next = state.with_transition(StateEvent::WorkspaceReady)?;
    assert!(matches!(next.phase, Phase::WorkspaceReady { .. }));
    Ok(())
}

#[test]
fn invalid_transition_from_completed_returns_terminal_error() -> Result<(), Box<dyn std::error::Error>> {
    let id = parse_bead_id("comp-invalid")?;
    let state = WorkflowState::new(id)
        .with_transition(StateEvent::WorkspaceReady)?
        .with_transition(StateEvent::Completed(StepResult::Success))?;
    assert!(state.phase.is_terminal());
    let result = state.with_transition(StateEvent::StepStarted("extra".into()));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalid state transition"));
    Ok(())
}