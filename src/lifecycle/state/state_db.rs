#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::BeadId;
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

const KEYSPACE_WORKFLOWS: &str = "workflows";
const KEYSPACE_JOURNAL: &str = "journal";

#[derive(Debug, Error)]
pub enum StateDbError {
    #[error("fjall error: {0}")]
    Fjall(#[from] fjall::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StateDbError>;

#[derive(Clone)]
pub struct StateDb {
    db: Arc<Database>,
    workflows: Keyspace,
    journal: Keyspace,
}

impl StateDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::builder(path)
            .cache_size(64 * 1024 * 1024)
            .journal_compression(fjall::CompressionType::Lz4)
            .open()?;
        let workflows = db.keyspace(KEYSPACE_WORKFLOWS, KeyspaceCreateOptions::default)?;
        let journal = db.keyspace(KEYSPACE_JOURNAL, KeyspaceCreateOptions::default)?;
        Ok(Self {
            db: Arc::new(db),
            workflows,
            journal,
        })
    }

    #[allow(dead_code)]
    pub fn persist_workflow(&self, bead_id: &BeadId, state_json: &str) -> Result<()> {
        self.workflows.insert(bead_id.as_str(), state_json)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_workflow(&self, bead_id: &BeadId) -> Result<Option<String>> {
        self.workflows
            .get(bead_id.as_str())
            .map_err(StateDbError::Fjall)
            .map(|opt| opt.and_then(|v| String::from_utf8(v.to_vec()).ok()))
    }

    #[allow(dead_code)]
    pub fn append_journal(&self, bead_id: &BeadId, entry_json: &str) -> Result<()> {
        let key = next_journal_key(bead_id);
        self.journal.insert(&key, entry_json)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_journal(&self, bead_id: &BeadId) -> Result<Vec<String>> {
        let prefix = format!("{}_", bead_id.as_str());
        Ok(self
            .journal
            .prefix(&prefix)
            .filter_map(|guard| {
                guard
                    .value()
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            })
            .collect())
    }

    #[allow(dead_code)]
    pub fn list_workflow_ids(&self) -> Result<Vec<String>> {
        Ok(self
            .workflows
            .iter()
            .filter_map(|guard| {
                guard
                    .key()
                    .ok()
                    .and_then(|k| String::from_utf8(k.to_vec()).ok())
            })
            .collect())
    }

    #[allow(dead_code)]
    pub fn delete_workflow(&self, bead_id: &BeadId) -> Result<()> {
        self.workflows.remove(bead_id.as_str())?;
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.db.persist(PersistMode::SyncAll)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn snapshot(&self) -> fjall::Snapshot {
        self.db.snapshot()
    }

    pub fn batch_persist_state(
        &self,
        bead_id: &BeadId,
        state_json: &str,
        journal_entries: &[(String, String)],
    ) -> Result<()> {
        let mut batch = self.db.batch();
        batch.insert(&self.workflows, bead_id.as_str(), state_json);
        for (key, value) in journal_entries {
            batch.insert(&self.journal, key.as_str(), value.as_str());
        }
        batch.commit()?;
        Ok(())
    }

    pub fn next_journal_key(&self, bead_id: &BeadId) -> String {
        next_journal_key(bead_id)
    }
}

static JOURNAL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn timestamp_now() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(_) => 0,
    }
}

fn next_journal_key(bead_id: &BeadId) -> String {
    let ts = timestamp_now();
    let seq = JOURNAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{ts:020}_{seq:010}", bead_id.as_str())
}

// ─── TESTS ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{BeadId, WorkflowState};

    #[test]
    fn timestamp_now_returns_real_value() {
        let ts = timestamp_now();
        assert!(ts > 1, "timestamp_now must return epoch ms, not a constant");
    }

    #[test]
    fn persist_workflow_writes_data() {
        // Target: StateDb::persist_workflow → Ok(()) mutant
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("wf-persist").unwrap();
        let state_json = serde_json::to_string(&WorkflowState::new(bead_id.clone())).unwrap();
        db.persist_workflow(&bead_id, &state_json).unwrap();
        let loaded = db.load_workflow(&bead_id).unwrap();
        assert!(loaded.is_some(), "persist_workflow must actually write to Fjall");
    }

    #[test]
    fn append_journal_writes_data() {
        // Target: StateDb::append_journal → Ok(()) mutant
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("j-persist").unwrap();
        db.append_journal(&bead_id, "j1").unwrap();
        db.append_journal(&bead_id, "j2").unwrap();
        let entries = db.load_journal(&bead_id).unwrap();
        assert_eq!(entries.len(), 2, "append_journal must actually write entries");
    }

    #[test]
    fn flush_persists_data() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("flush-test").unwrap();
        db.persist_workflow(&bead_id, "{\"phase\":\"planned\"}").unwrap();
        db.flush().unwrap();
        drop(db);
        let db2 = StateDb::open(dir.path().join("db")).unwrap();
        let loaded = db2.load_workflow(&bead_id).unwrap();
        assert!(loaded.is_some(), "flush must persist data so it survives DB close/reopen");
    }

    #[test]
    fn flush_without_persist_still_closes_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        db.flush().unwrap();
    }
}
