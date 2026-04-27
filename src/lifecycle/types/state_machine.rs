use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::{BeadData, BeadId, ErrorMessage, StepName, StepResult};
use crate::lifecycle::error::{FailureCategory, LifecycleError};

const SMALL_VEC_SIZE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    Planned {
        bead: BeadData,
    },
    WorkspaceReady {
        bead: BeadData,
    },
    Executing {
        bead: BeadData,
        step: StepName,
    },
    Completed {
        bead: BeadData,
        result: StepResult,
    },
    Failed {
        bead_id: BeadId,
        error: ErrorMessage,
    },
}

impl Phase {
    #[allow(dead_code)]
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Failed { .. })
    }

    #[must_use]
    pub fn bead_id(&self) -> &BeadId {
        match self {
            Self::Planned { bead }
            | Self::WorkspaceReady { bead }
            | Self::Executing { bead, .. }
            | Self::Completed { bead, .. } => &bead.bead_id,
            Self::Failed { bead_id, .. } => bead_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateEvent {
    WorkspaceReady,
    StepStarted(StepName),
    Completed(StepResult),
    Failed(ErrorMessage),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowState {
    pub phase: Phase,
    pub completed_steps: StepDeps,
}

pub type StepDeps = SmallVec<[StepName; SMALL_VEC_SIZE]>;

fn transition_phase(phase: &Phase, event: &StateEvent) -> Result<Phase, LifecycleError> {
    if let StateEvent::Failed(error) = event {
        return failed_transition(phase, error);
    }
    standard_transition(phase, event).ok_or_else(invalid_transition)
}

fn standard_transition(phase: &Phase, event: &StateEvent) -> Option<Phase> {
    planned_workspace_ready(phase, event)
        .or_else(|| step_started_transition(phase, event))
        .or_else(|| executing_workspace_ready(phase, event))
        .or_else(|| completed_transition(phase, event))
}

fn planned_workspace_ready(phase: &Phase, event: &StateEvent) -> Option<Phase> {
    match (phase, event) {
        (Phase::Planned { bead }, StateEvent::WorkspaceReady) => {
            Some(Phase::WorkspaceReady { bead: bead.clone() })
        }
        _ => None,
    }
}

fn step_started_transition(phase: &Phase, event: &StateEvent) -> Option<Phase> {
    match (phase, event) {
        (
            Phase::Planned { bead }
            | Phase::WorkspaceReady { bead }
            | Phase::Executing { bead, .. },
            StateEvent::StepStarted(step),
        ) => Some(Phase::Executing {
            bead: bead.clone(),
            step: step.clone(),
        }),
        _ => None,
    }
}

fn executing_workspace_ready(phase: &Phase, event: &StateEvent) -> Option<Phase> {
    match (phase, event) {
        (Phase::Executing { bead, .. }, StateEvent::WorkspaceReady) => {
            Some(Phase::WorkspaceReady { bead: bead.clone() })
        }
        _ => None,
    }
}

fn completed_transition(phase: &Phase, event: &StateEvent) -> Option<Phase> {
    match (phase, event) {
        (
            Phase::WorkspaceReady { bead } | Phase::Executing { bead, .. },
            StateEvent::Completed(result),
        ) => Some(Phase::Completed {
            bead: bead.clone(),
            result: *result,
        }),
        _ => None,
    }
}

fn invalid_transition() -> LifecycleError {
    LifecycleError::terminal(FailureCategory::Validation, "invalid state transition")
}

fn failed_transition(phase: &Phase, error: &ErrorMessage) -> Result<Phase, LifecycleError> {
    match phase {
        Phase::Planned { bead }
        | Phase::WorkspaceReady { bead }
        | Phase::Executing { bead, .. } => Ok(Phase::Failed {
            bead_id: bead.bead_id.clone(),
            error: error.clone(),
        }),
        Phase::Completed { .. } | Phase::Failed { .. } => Err(LifecycleError::terminal(
            FailureCategory::Validation,
            error.as_str(),
        )),
    }
}

impl WorkflowState {
    #[must_use]
    pub fn new(bead_id: BeadId) -> Self {
        let bead = BeadData::from_bead_id(bead_id);
        Self {
            phase: Phase::Planned { bead },
            completed_steps: StepDeps::new(),
        }
    }

    pub fn with_transition(self, event: StateEvent) -> Result<Self, LifecycleError> {
        let next_phase = transition_phase(&self.phase, &event)?;
        Ok(Self {
            phase: next_phase,
            completed_steps: self.completed_steps,
        })
    }

    #[must_use]
    pub fn with_advanced_step(self, step_name: StepName) -> Self {
        let mut completed = self.completed_steps;
        completed.push(step_name);
        Self {
            phase: self.phase,
            completed_steps: completed,
        }
    }
}
