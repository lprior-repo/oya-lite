use oya_lite::lifecycle::effects::executor::{CommandExecutor, CommandFailure, CommandResult};
use oya_lite::lifecycle::effects::run::run_effect;
use oya_lite::lifecycle::types::Effect;
use std::future::Future;
use std::pin::Pin;

type BoxedCommandFuture<'a> =
    Pin<Box<dyn Future<Output = Result<CommandResult, CommandFailure>> + Send + 'a>>;

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
    fn success_with(stdout: &str, stderr: &str, code: i32) -> Self {
        Self {
            outcome: MockOutcome::Ok(CommandResult {
                status_code: Some(code),
                stdout: stdout.into(),
                stderr: stderr.into(),
            }),
        }
    }

    fn timeout() -> Self {
        Self {
            outcome: MockOutcome::Timeout(std::time::Duration::from_secs(3600)),
        }
    }

    fn io_err(msg: &str) -> Self {
        Self {
            outcome: MockOutcome::IoErr(msg.to_owned()),
        }
    }
}

fn outcome_to_result(outcome: MockOutcome) -> Result<CommandResult, CommandFailure> {
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

#[tokio::test]
async fn run_effect_workspace_prepare_success() -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::success_with("done", "", 0);
    let effect = Effect::WorkspacePrepare {
        workspace: "ws".into(),
        path: "/tmp".into(),
    };
    let entry = run_effect(&exec, effect, None).await?;
    assert!(entry.result.is_success());
    assert_eq!(entry.stdout, "done");
    assert_eq!(entry.stderr, "");
    Ok(())
}

#[tokio::test]
async fn run_effect_command_nonzero_is_ok_with_success_false(
) -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::success_with("", "error output", 1);
    let effect = Effect::WorkspacePrepare {
        workspace: "ws".into(),
        path: "/tmp".into(),
    };
    let entry = run_effect(&exec, effect, None).await?;
    assert!(!entry.result.is_success());
    Ok(())
}

#[tokio::test]
async fn run_effect_timeout_returns_transient_error() -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::timeout();
    let effect = Effect::WorkspacePrepare {
        workspace: "ws".into(),
        path: "/tmp".into(),
    };
    let result = run_effect(&exec, effect, None).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(!err.is_terminal());
        assert_eq!(
            err.category(),
            oya_lite::lifecycle::error::FailureCategory::Command
        );
    }
    Ok(())
}

#[tokio::test]
async fn run_effect_io_error_returns_transient_error() -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::io_err("opencode not found");
    let effect = Effect::Opencode {
        prompt: "fix".into(),
        model: "gpt-4".into(),
        cwd: Some("..".into()),
    };
    let result = run_effect(&exec, effect, Some("..".into())).await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(!err.is_terminal());
    }
    Ok(())
}

#[tokio::test]
async fn run_effect_with_cwd_forwarded() -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::success_with("ok", "", 0);
    let effect = Effect::Jj {
        args: oya_lite::lifecycle::types::JjArgs(vec!["status".into()]),
        cwd: Some("/tmp".into()),
    };
    let result = run_effect(&exec, effect, Some("/tmp".into())).await;
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn run_effect_journal_entry_captures_effect() -> Result<(), Box<dyn std::error::Error>> {
    let exec = MockExecutor::success_with("out", "err", 0);
    let effect = Effect::Opencode {
        prompt: "hello".into(),
        model: "test-model".into(),
        cwd: None,
    };
    let entry = run_effect(&exec, effect, None).await?;
    assert!(entry.result.is_success());
    assert_eq!(entry.timeout_secs, 3600);
    assert_eq!(entry.stdout, "out");
    assert_eq!(entry.stderr, "err");
    Ok(())
}
