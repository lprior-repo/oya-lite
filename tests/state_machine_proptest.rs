#![allow(clippy::unwrap_used)]

use oya_lite::lifecycle::types::{BeadId, StateEvent, StepName, WorkflowState};
use proptest::prelude::*;

proptest! {
    #[test]
    fn workflow_state_handles_bounded_step_counts(step_count in 0..10usize) {
        let id = BeadId::parse("prop-test").unwrap();
        let state = (0..step_count).fold(
            WorkflowState::new(id.clone()).with_transition(StateEvent::WorkspaceReady).unwrap(),
            |s, _| s.with_advanced_step(StepName("step".into()))
        );
        prop_assert_eq!(state.completed_steps.len(), step_count);
    }

    #[test]
    fn workflow_state_clone_preserves_phase_and_completed_steps(id_str in "[a-z0-9-]{1,32}") {
        let id = BeadId::parse(&id_str).unwrap();
        let state = WorkflowState::new(id.clone());
        let cloned = state.clone();
        prop_assert_eq!(state.phase, cloned.phase);
        prop_assert_eq!(state.completed_steps.len(), cloned.completed_steps.len());
    }

    #[test]
    fn phase_bead_id_matches_original_parsed_id(id_str in "[a-z0-9-]{1,32}") {
        let id = BeadId::parse(&id_str).unwrap();
        let state = WorkflowState::new(id.clone());
        prop_assert_eq!(state.phase.bead_id(), &id);
    }
}