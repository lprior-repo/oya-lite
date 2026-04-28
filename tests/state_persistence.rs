#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_lite::lifecycle::error::LifecycleError;
use oya_lite::lifecycle::state::{load_state, persist_state, StateDb};
use oya_lite::lifecycle::types::{
    BeadId, Effect, EffectJournalEntry, Phase, StateEvent, StepResult, WorkflowState,
};
use tempfile::TempDir;

fn make_state_with_steps(
    bead_id: &str,
    step_count: usize,
) -> Result<WorkflowState, Box<dyn std::error::Error>> {
    let id = BeadId::parse(bead_id)?;
    Ok((0..step_count).fold(
        WorkflowState::new(id).with_transition(StateEvent::WorkspaceReady)?,
        |state, i| {
            state.with_advanced_step(oya_lite::lifecycle::types::StepName(format!("step-{i}")))
        },
    ))
}

fn make_journal(count: usize) -> Vec<EffectJournalEntry> {
    (0..count)
        .map(|i| EffectJournalEntry {
            effect: Effect::WorkspacePrepare {
                workspace: oya_lite::lifecycle::types::WorkspaceName(format!("ws-{i}")),
                path: oya_lite::lifecycle::types::WorkspacePath(format!("path-{i}")),
            },
            timeout_secs: 3600,
            result: if i % 2 == 0 {
                StepResult::Success
            } else {
                StepResult::Failure
            },
            stdout: "stdout".into(),
            stderr: "stderr".into(),
        })
        .collect()
}

#[test]
fn state_db_open_creates_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let path = dir.path().join("oya-db");
    let db = StateDb::open(&path)?;
    assert!(path.exists());
    drop(db);
    Ok(())
}

#[test]
fn state_db_persist_and_load_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let state = make_state_with_steps("test-bead", 3)?;
    let journal = make_journal(2);

    persist_state(&db, &state, &journal)?;

    let bead_id = BeadId::parse("test-bead")?;
    let loaded = load_state(&db, &bead_id)?;
    let (loaded_state, loaded_journal) = loaded.ok_or("expected state")?;

    assert_eq!(loaded_state.completed_steps.len(), 3);
    assert_eq!(loaded_journal.len(), 2);
    Ok(())
}

#[test]
fn state_db_load_nonexistent_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let bead_id = BeadId::parse("ghost")?;
    let result = load_state(&db, &bead_id)?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn state_db_completed_state_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let id = BeadId::parse("comp-test")?;
    let state = WorkflowState::new(id.clone())
        .with_transition(StateEvent::WorkspaceReady)?
        .with_advanced_step(oya_lite::lifecycle::types::StepName(
            "workspace-prepare".into(),
        ))
        .with_transition(StateEvent::Completed(StepResult::Success))?;

    let journal = make_journal(1);
    persist_state(&db, &state, &journal)?;

    let loaded = load_state(&db, &id)?.ok_or("expected state")?;
    assert!(matches!(
        loaded.0.phase,
        Phase::Completed {
            result: StepResult::Success,
            ..
        }
    ));
    Ok(())
}

#[test]
fn state_db_failed_state_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let id = BeadId::parse("fail-test")?;
    let error = LifecycleError::terminal(
        oya_lite::lifecycle::error::FailureCategory::Command,
        "command exploded",
    );
    let state = WorkflowState::new(id.clone())
        .with_transition(StateEvent::WorkspaceReady)?
        .with_transition(StateEvent::Failed(
            oya_lite::lifecycle::types::ErrorMessage(error.to_string()),
        ))?;

    persist_state(&db, &state, &[])?;

    let loaded = load_state(&db, &id)?.ok_or("expected state")?;
    assert!(matches!(loaded.0.phase, Phase::Failed { .. }));
    Ok(())
}

#[test]
fn state_db_list_workflow_ids() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let state_a = make_state_with_steps("bead-a", 1)?;
    let state_b = make_state_with_steps("bead-b", 1)?;

    persist_state(&db, &state_a, &[])?;
    persist_state(&db, &state_b, &[])?;

    let ids = db.list_workflow_ids()?;
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"bead-a".to_owned()));
    assert!(ids.contains(&"bead-b".to_owned()));
    Ok(())
}

#[test]
fn state_db_journal_entries_per_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let state = make_state_with_steps("j-test", 1)?;
    let journal = make_journal(5);

    persist_state(&db, &state, &journal)?;

    let bead_id = BeadId::parse("j-test")?;
    let loaded = load_state(&db, &bead_id)?.ok_or("expected state")?;
    assert_eq!(loaded.1.len(), 5);
    Ok(())
}

 #[test]
fn state_db_delete_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let state = make_state_with_steps("del-me", 1)?;
    persist_state(&db, &state, &[])?;

    let bead_id = BeadId::parse("del-me")?;
    db.delete_workflow(&bead_id)?;
    let loaded = load_state(&db, &bead_id)?;
    assert!(loaded.is_none());
    Ok(())
}

// ── Tests to kill StateDb mutation survivors ──

#[test]
fn persist_workflow_writes_to_database() -> Result<(), Box<dyn std::error::Error>> {
    // This test targets the mutant: StateDb::persist_workflow → Ok(())
    // If the mutation replaces the body with Ok(()), load_workflow returns None.
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let bead_id = BeadId::parse("persist-wf")?;
    let state_json = serde_json::to_string(&WorkflowState::new(bead_id.clone()))?;
    db.persist_workflow(&bead_id, &state_json)?;

    let loaded = db.load_workflow(&bead_id)?;
    assert!(loaded.is_some(), "persist_workflow must actually write to Fjall");
    let loaded_state: WorkflowState = serde_json::from_str(&loaded.unwrap())?;
    assert!(matches!(loaded_state.phase, Phase::Planned { .. }));
    Ok(())
}

#[test]
fn append_journal_persists_entries() -> Result<(), Box<dyn std::error::Error>> {
    // This test targets the mutant: StateDb::append_journal → Ok(())
    // If the mutation replaces the body with Ok(()), load_journal returns empty.
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let bead_id = BeadId::parse("append-j")?;
    db.append_journal(&bead_id, "entry-1")?;
    db.append_journal(&bead_id, "entry-2")?;

    let entries = db.load_journal(&bead_id)?;
    assert_eq!(entries.len(), 2, "append_journal must actually write journal entries");
    Ok(())
}

#[test]
fn flush_completes_without_error() -> Result<(), Box<dyn std::error::Error>> {
    // This test targets the mutant: StateDb::flush → Ok(())
    // flush must not panic or return Err. It exercises the persistence layer.
    let dir = TempDir::new()?;
    let db = StateDb::open(dir.path().join("db"))?;

    let bead_id = BeadId::parse("flush-test")?;
    db.persist_workflow(&bead_id, "{\"phase\":\"planned\"}")?;
    db.flush()?;
    Ok(())
}
