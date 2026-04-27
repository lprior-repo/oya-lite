#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::Effect;
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status_code == Some(0)
    }
}

#[derive(Debug, Error)]
pub enum CommandFailure {
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait CommandExecutor: Send + Sync {
    fn execute(
        &self,
        effect: Effect,
        cwd: Option<String>,
        timeout_secs: u64,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<CommandResult, CommandFailure>>
                + Send
                + '_,
        >,
    >;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokioCommandExecutor;

impl TokioCommandExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    async fn execute_impl(
        effect: Effect,
        cwd: Option<String>,
        timeout_secs: u64,
    ) -> std::result::Result<CommandResult, CommandFailure> {
        let program = effect.program();
        let args = effect.args();
        let timeout_duration = Duration::from_secs(timeout_secs);

        let output = run_command(program, &args, cwd.as_deref(), timeout_duration).await?;
        Ok(command_output_to_result(output))
    }
}

async fn run_command(
    program: &str,
    args: &[String],
    cwd: Option<&str>,
    timeout_duration: Duration,
) -> std::result::Result<std::process::Output, CommandFailure> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    wait_for_output(cmd, timeout_duration).await
}

async fn wait_for_output(
    mut cmd: Command,
    timeout_duration: Duration,
) -> std::result::Result<std::process::Output, CommandFailure> {
    let child = cmd.spawn()?;
    match timeout(timeout_duration, child.wait_with_output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(CommandFailure::Io(e)),
        Err(_) => Err(CommandFailure::Timeout(timeout_duration)),
    }
}

fn command_output_to_result(output: std::process::Output) -> CommandResult {
    CommandResult {
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

impl CommandExecutor for TokioCommandExecutor {
    fn execute(
        &self,
        effect: Effect,
        cwd: Option<String>,
        timeout_secs: u64,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<CommandResult, CommandFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(Self::execute_impl(effect, cwd, timeout_secs))
    }
}

impl Default for TokioCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_result_success_zero() {
        let r = CommandResult {
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(r.is_success());
    }

    #[test]
    fn command_result_failure_nonzero() {
        let r = CommandResult {
            status_code: Some(1),
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(!r.is_success());
    }

    #[test]
    fn command_result_none_status_not_success() {
        let r = CommandResult {
            status_code: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(!r.is_success());
    }

    #[test]
    fn tokio_command_executor_is_copy() {
        let a = TokioCommandExecutor::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn command_failure_timeout_display() {
        let f = CommandFailure::Timeout(Duration::from_secs(42));
        let s = format!("{f}");
        assert!(s.contains("42"));
    }

    #[test]
    fn command_failure_io_from_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let f = CommandFailure::Io(io_err);
        let s = format!("{f}");
        assert!(s.contains("missing"));
    }

    #[test]
    fn command_output_to_result_with_output() {
        use std::process::Output;
        let output = Output {
            status: std::process::ExitStatus::default(),
            stdout: b"hello".to_vec(),
            stderr: b"world".to_vec(),
        };
        let result = command_output_to_result(output);
        assert_eq!(result.stdout, "hello");
        assert_eq!(result.stderr, "world");
    }

    #[test]
    fn command_output_to_result_with_utf8_lossy() {
        use std::process::Output;
        let output = Output {
            status: std::process::ExitStatus::default(),
            stdout: vec![0xf0, 0x9f, 0x98, 0x80],
            stderr: vec![],
        };
        let result = command_output_to_result(output);
        assert_eq!(result.stdout, "😀");
    }
}
