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
