#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::executor::{CommandExecutor, CommandFailure, CommandResult};
use crate::lifecycle::error::{FailureCategory, LifecycleError};
use crate::lifecycle::types::{Effect, EffectJournalEntry, StepResult};

pub fn effect_timeout_secs(effect: &Effect) -> u64 {
    match effect {
        Effect::WorkspacePrepare { .. } => 30,
        Effect::Jj { .. } => 120,
        Effect::MoonRun { .. } => 300,
        Effect::MoonCi { .. } => 600,
        Effect::Opencode { .. } => 3600,
    }
}

pub fn classify_command_failure(failure: &CommandFailure) -> LifecycleError {
    match failure {
        CommandFailure::Timeout(_) => {
            LifecycleError::transient(FailureCategory::Command, "command timed out")
        }
        CommandFailure::Io(_) => LifecycleError::transient(
            FailureCategory::Command,
            "io error during command execution",
        ),
    }
}

fn command_result_is_success(effect: &Effect, result: &CommandResult) -> bool {
    result.is_success()
        && match effect {
            Effect::Opencode { .. } => !opencode_output_is_error(&result.stdout, &result.stderr),
            _ => true,
        }
}

pub fn opencode_output_is_error(stdout: &str, stderr: &str) -> bool {
    stdout.contains("\"type\":\"error\"")
        || stderr.contains("ProviderModelNotFoundError")
        || stderr.contains("Model not found")
}

pub async fn run_effect<E: CommandExecutor>(
    executor: &E,
    effect: Effect,
    cwd: Option<String>,
) -> std::result::Result<EffectJournalEntry, LifecycleError> {
    let timeout_secs = effect_timeout_secs(&effect);
    let effect_clone = effect.clone();
    let exec_result = executor.execute(effect, cwd, timeout_secs).await;
    match exec_result {
        Ok(result) => Ok(build_journal_entry(effect_clone, timeout_secs, result)),
        Err(failure) => Err(classify_command_failure(&failure)),
    }
}

fn build_journal_entry(
    effect: Effect,
    timeout_secs: u64,
    result: CommandResult,
) -> EffectJournalEntry {
    let result_val = if command_result_is_success(&effect, &result) {
        StepResult::Success
    } else {
        StepResult::Failure
    };
    EffectJournalEntry {
        effect,
        timeout_secs,
        result: result_val,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::effects::executor::{CommandExecutor, CommandResult};
    use std::future::Future;
    use std::pin::Pin;

    type BoxedCommandFuture<'a> = Pin<
        Box<dyn Future<Output = std::result::Result<CommandResult, CommandFailure>> + Send + 'a>,
    >;

    #[derive(Clone)]
    enum MockOutcome {
        Ok(CommandResult),
        Timeout(std::time::Duration),
        IoErr(String),
    }

    struct MockExecutor {
        outcome: MockOutcome,
    }

    impl MockExecutor {
        fn success(code: i32) -> Self {
            Self {
                outcome: MockOutcome::Ok(CommandResult {
                    status_code: Some(code),
                    stdout: "ok".into(),
                    stderr: String::new(),
                }),
            }
        }

        fn opencode_error() -> Self {
            Self {
                outcome: MockOutcome::Ok(CommandResult {
                    status_code: Some(0),
                    stdout: r#"{"type":"error","error":{"data":{"message":"Model not found"}}}"#
                        .into(),
                    stderr: "ProviderModelNotFoundError".into(),
                }),
            }
        }

        fn timeout() -> Self {
            Self {
                outcome: MockOutcome::Timeout(std::time::Duration::from_secs(30)),
            }
        }

        fn io_err(msg: &str) -> Self {
            Self {
                outcome: MockOutcome::IoErr(msg.to_owned()),
            }
        }
    }

    fn outcome_to_result(
        outcome: MockOutcome,
    ) -> std::result::Result<CommandResult, CommandFailure> {
        match outcome {
            MockOutcome::Ok(result) => Ok(result),
            MockOutcome::Timeout(duration) => Err(CommandFailure::Timeout(duration)),
            MockOutcome::IoErr(message) => Err(io_failure(&message)),
        }
    }

    fn io_failure(message: &str) -> CommandFailure {
        CommandFailure::Io(std::io::Error::new(std::io::ErrorKind::NotFound, message))
    }

    impl CommandExecutor for MockExecutor {
        fn execute(
            &self,
            _effect: Effect,
            _cwd: Option<String>,
            _timeout_secs: u64,
        ) -> BoxedCommandFuture<'_> {
            let outcome = self.outcome.clone();
            Box::pin(async move { outcome_to_result(outcome) })
        }
    }

    #[test]
    fn workspace_prepare_timeout_is_30_seconds() {
        let ws = Effect::WorkspacePrepare {
            workspace: "x".into(),
            path: "y".into(),
        };
        assert_eq!(effect_timeout_secs(&ws), 30);
    }

    #[test]
    fn jj_timeout_is_120_seconds() {
        let jj = Effect::Jj {
            args: crate::lifecycle::types::JjArgs(vec![]),
            cwd: None,
        };
        assert_eq!(effect_timeout_secs(&jj), 120);
    }

    #[test]
    fn moon_run_timeout_is_300_seconds() {
        let moon = Effect::MoonRun {
            task: "t".into(),
            cwd: None,
        };
        assert_eq!(effect_timeout_secs(&moon), 300);
    }

    #[test]
    fn moon_ci_timeout_is_600_seconds() {
        let moon_ci = Effect::MoonCi { cwd: None };
        assert_eq!(effect_timeout_secs(&moon_ci), 600);
    }

    #[test]
    fn opencode_timeout_is_3600_seconds() {
        let oc = Effect::Opencode {
            prompt: "p".into(),
            model: "m".into(),
            cwd: None,
        };
        assert_eq!(effect_timeout_secs(&oc), 3600);
    }

    // ── opencode_output_is_error ──

    #[test]
    fn opencode_output_is_error_true_when_type_error_in_stdout() {
        assert!(opencode_output_is_error(r#"{"type":"error"}"#, ""));
    }

    #[test]
    fn opencode_output_is_error_true_when_provider_error_in_stderr() {
        assert!(opencode_output_is_error("", "ProviderModelNotFoundError: ..."));
    }

    #[test]
    fn opencode_output_is_error_true_when_model_not_found_in_stderr() {
        assert!(opencode_output_is_error("", "Error: Model not found"));
    }

    #[test]
    fn opencode_output_is_error_false_when_no_error_markers() {
        assert!(!opencode_output_is_error(r#"{"type":"text","content":"hi"}"#, ""));
    }

    #[test]
    fn opencode_output_is_error_false_when_empty() {
        assert!(!opencode_output_is_error("", ""));
    }

    #[test]
    fn opencode_output_is_error_partial_json_not_matched() {
        // The string must be exactly the error marker, not just any JSON
        assert!(!opencode_output_is_error(r#"{"type":"errorrr"}"#, ""));
        assert!(!opencode_output_is_error(r#"{"type":"text","error":"..."}"#, ""));
    }

    #[test]
    fn opencode_output_is_error_model_not_found_partial_not_matched() {
        assert!(!opencode_output_is_error("", "Model was not found")); // different order
        assert!(!opencode_output_is_error("", "model not found error")); // extra words
    }

    #[test]
    fn opencode_output_is_error_provider_error_partial_not_matched() {
        assert!(!opencode_output_is_error("", "ProviderNotFoundError")); // different error name
        assert!(!opencode_output_is_error("", "ProviderModelError")); // partial
    }

    #[test]
    fn classify_timeout_is_transient() {
        let failure = CommandFailure::Timeout(std::time::Duration::from_secs(30));
        let err = classify_command_failure(&failure);
        assert!(!err.is_terminal());
        assert_eq!(err.category(), FailureCategory::Command);
    }

    #[test]
    fn classify_io_is_transient() {
        let failure = CommandFailure::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        let err = classify_command_failure(&failure);
        assert!(!err.is_terminal());
    }

    #[tokio::test]
    async fn run_effect_success_returns_entry() -> Result<(), Box<dyn std::error::Error>> {
        let exec = MockExecutor::success(0);
        let effect = Effect::WorkspacePrepare {
            workspace: "x".into(),
            path: "y".into(),
        };
        let entry = run_effect(&exec, effect, None).await?;
        assert!(entry.result.is_success());
        assert_eq!(entry.stdout, "ok");
        Ok(())
    }

    #[tokio::test]
    async fn run_effect_non_zero_returns_entry_with_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exec = MockExecutor::success(1);
        let effect = Effect::WorkspacePrepare {
            workspace: "x".into(),
            path: "y".into(),
        };
        let entry = run_effect(&exec, effect, None).await?;
        assert!(!entry.result.is_success());
        Ok(())
    }

    #[tokio::test]
    async fn run_effect_opencode_json_error_returns_failure_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exec = MockExecutor::opencode_error();
        let effect = Effect::Opencode {
            prompt: "p".into(),
            model: "bad".into(),
            cwd: None,
        };
        let entry = run_effect(&exec, effect, None).await?;
        assert!(!entry.result.is_success());
        assert!(entry.stdout.contains("\"type\":\"error\""));
        Ok(())
    }

    #[tokio::test]
    async fn run_effect_timeout_returns_err() -> Result<(), Box<dyn std::error::Error>> {
        let exec = MockExecutor::timeout();
        let effect = Effect::WorkspacePrepare {
            workspace: "x".into(),
            path: "y".into(),
        };
        let result = run_effect(&exec, effect, None).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(!err.is_terminal());
        }
        Ok(())
    }

    #[tokio::test]
    async fn run_effect_io_error_returns_err() -> Result<(), Box<dyn std::error::Error>> {
        let exec = MockExecutor::io_err("io error");
        let effect = Effect::WorkspacePrepare {
            workspace: "x".into(),
            path: "y".into(),
        };
        let result = run_effect(&exec, effect, None).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(!err.is_terminal());
        }
        Ok(())
    }
}
