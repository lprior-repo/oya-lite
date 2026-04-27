#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::state_db::{Result, StateDb};
use crate::lifecycle::types::{BeadId, EffectJournalEntry, WorkflowState};

pub fn persist_state(
    db: &StateDb,
    state: &WorkflowState,
    journal: &[EffectJournalEntry],
) -> Result<()> {
    let bead_id = state.phase.bead_id();
    let state_json = serde_json::to_string(state)?;
    let journal_entries: Vec<(String, String)> = journal
        .iter()
        .map(|entry| {
            let key = db.next_journal_key(bead_id);
            let value = serde_json::to_string(entry)?;
            Ok((key, value))
        })
        .collect::<Result<Vec<_>>>()?;
    db.batch_persist_state(bead_id, &state_json, &journal_entries)?;
    db.flush()?;
    Ok(())
}

#[allow(dead_code)]
pub fn load_state(
    db: &StateDb,
    bead_id: &BeadId,
) -> Result<Option<(WorkflowState, Vec<EffectJournalEntry>)>> {
    let state_json = db.load_workflow(bead_id)?;
    let journal_entries = db.load_journal(bead_id)?;
    match state_json {
        Some(json) => {
            let state: WorkflowState = serde_json::from_str(&json)?;
            let journal: Vec<EffectJournalEntry> = journal_entries
                .iter()
                .filter_map(|j| match serde_json::from_str(j) {
                    Ok(entry) => Some(entry),
                    Err(e) => {
                        tracing::warn!("corrupted journal entry for {bead_id:?}: {e}");
                        None
                    }
                })
                .collect();
            Ok(Some((state, journal)))
        }
        None => Ok(None),
    }
}
