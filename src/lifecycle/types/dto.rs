use serde::{Deserialize, Serialize};

use super::{
    BeadId, Effect, ErrorMessage, ModelId, Phase, PromptString, RepoUrl, StepName, StepResult,
    Timestamp,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectJournalEntry {
    pub effect: Effect,
    pub timeout_secs: u64,
    pub result: StepResult,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LifecycleRequest {
    pub bead_id: BeadId,
    pub model: Option<ModelId>,
    pub repo: Option<RepoUrl>,
    pub prompt: Option<PromptString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LifecycleProgress {
    Initialized {
        bead_id: BeadId,
        steps: Vec<StepName>,
    },
    StepStarted {
        step: StepName,
        started_at: Timestamp,
    },
    StepCompleted {
        step: StepName,
        duration_ms: u64,
    },
    StepFailed {
        step: StepName,
        error: ErrorMessage,
    },
    Finished {
        result: StepResult,
        message: Option<ErrorMessage>,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleOutcome {
    pub bead_id: BeadId,
    pub result: StepResult,
    pub state: Phase,
    pub completed_steps: Vec<StepName>,
}
