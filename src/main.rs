#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod lifecycle;

use std::path::PathBuf;

use clap::Parser;
use lifecycle::run::{LifecycleConfig, LifecycleOrchestrator};
use lifecycle::types::{
    BeadId, LifecycleProgress, ModelId, OpencodeServerConfig, OpencodeUrl, PromptString, RepoUrl,
    SensitiveString, Username,
};
use lifecycle::LifecycleRequest;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "oya-lite")]
#[command(about = "Lightweight workflow orchestrator")]
#[command(version)]
struct Args {
    #[arg(long, default_value = ".oya-lite")]
    data_dir: PathBuf,
    #[arg(long)]
    bead_id: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    server: Option<String>,
    #[arg(long, default_value = "opencode")]
    server_user: String,
    #[arg(long, env = "OPENCODE_SERVER_PASSWORD")]
    server_password: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let args = Args::parse();
    let orchestrator = LifecycleOrchestrator::new(build_config(&args)?)
        .map_err(|e| anyhow::anyhow!("failed to create lifecycle orchestrator: {}", e))?;
    run_request(&orchestrator, args).await
}

fn build_config(args: &Args) -> anyhow::Result<LifecycleConfig> {
    Ok(LifecycleConfig {
        data_dir: crate::lifecycle::types::DataDirPath(
            args.data_dir.to_string_lossy().into_owned(),
        ),
        opencode_server: build_server_config(args)?,
    })
}

fn build_server_config(args: &Args) -> anyhow::Result<Option<OpencodeServerConfig>> {
    let opencode_server = match (&args.server, &args.server_password) {
        (Some(url), Some(password)) => Some(OpencodeServerConfig {
            url: OpencodeUrl(url.clone()),
            username: Username(args.server_user.clone()),
            password: SensitiveString(password.clone()),
        }),
        (Some(_), None) => {
            anyhow::bail!(
                "--server-password (or OPENCODE_SERVER_PASSWORD) required when using --server"
            )
        }
        _ => None,
    };
    Ok(opencode_server)
}

fn build_request(args: Args) -> anyhow::Result<LifecycleRequest> {
    let Some(bead_id_str) = args.bead_id else {
        anyhow::bail!("No bead_id specified. Use --bead-id to specify a bead.");
    };
    let bead_id =
        BeadId::parse(&bead_id_str).map_err(|e| anyhow::anyhow!("invalid bead id: {}", e))?;
    Ok(LifecycleRequest {
        bead_id,
        model: args.model.map(ModelId),
        repo: args.repo.map(RepoUrl),
        prompt: args.prompt.map(PromptString),
    })
}

async fn run_request(orchestrator: &LifecycleOrchestrator, args: Args) -> anyhow::Result<()> {
    let request = build_request(args)?;
    let mut progress_rx = orchestrator.run_lifecycle(request).await?;
    if !drain_progress(&mut progress_rx).await {
        anyhow::bail!("lifecycle failed");
    }
    Ok(())
}

async fn drain_progress(rx: &mut tokio::sync::mpsc::Receiver<LifecycleProgress>) -> bool {
    let mut succeeded = true;
    while let Some(progress) = rx.recv().await {
        succeeded = succeeded && progress_indicates_success(&progress);
        match serde_json::to_string_pretty(&progress) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("failed to serialize progress: {e}"),
        }
    }
    succeeded
}

fn progress_indicates_success(progress: &LifecycleProgress) -> bool {
    match progress {
        LifecycleProgress::StepFailed { .. } => false,
        LifecycleProgress::Finished { result, .. } => result.is_success(),
        _ => true,
    }
}

// ─── TESTS ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{LifecycleProgress, StepResult};

    // ── build_server_config ──

    #[test]
    fn build_server_config_returns_none_when_no_server() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("test".into()),
            model: None,
            repo: None,
            prompt: None,
            server: None,
            server_user: "opencode".into(),
            server_password: None,
        };
        let result = build_server_config(&args).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn build_server_config_returns_config_with_auth() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("test".into()),
            model: None,
            repo: None,
            prompt: None,
            server: Some("http://localhost:4099".into()),
            server_user: "user1".into(),
            server_password: Some("secret".into()),
        };
        let result = build_server_config(&args).unwrap().unwrap();
        assert_eq!(result.url.as_str(), "http://localhost:4099");
        assert_eq!(result.username.as_str(), "user1");
    }

    #[test]
    fn build_server_config_requires_password_with_server() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("test".into()),
            model: None,
            repo: None,
            prompt: None,
            server: Some("http://localhost:4099".into()),
            server_user: "user1".into(),
            server_password: None,
        };
        let result = build_server_config(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("password"));
    }

    // ── build_request ──

    #[test]
    fn build_request_parses_valid_bead_id() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("my-test-bean-123".into()),
            model: Some("claude-3".into()),
            repo: Some("https://github.com/test/repo".into()),
            prompt: Some("fix bug".into()),
            server: None,
            server_user: "opencode".into(),
            server_password: None,
        };
        let req = build_request(args).unwrap();
        assert_eq!(req.bead_id.as_str(), "my-test-bean-123");
        assert_eq!(req.model.as_ref().map(|m| m.as_str()), Some("claude-3"));
        assert_eq!(req.prompt.as_ref().map(|p| p.as_str()), Some("fix bug"));
    }

    #[test]
    fn build_request_fails_without_bead_id() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: None,
            model: None,
            repo: None,
            prompt: None,
            server: None,
            server_user: "opencode".into(),
            server_password: None,
        };
        let result = build_request(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bead_id"));
    }

    #[test]
    fn build_request_fails_with_invalid_bead_id() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("bad/id".into()),
            model: None,
            repo: None,
            prompt: None,
            server: None,
            server_user: "opencode".into(),
            server_password: None,
        };
        let result = build_request(args);
        assert!(result.is_err());
    }

    #[test]
    fn build_request_optional_fields_optional() {
        let args = Args {
            data_dir: std::path::PathBuf::from("/tmp"),
            bead_id: Some("minimal".into()),
            model: None,
            repo: None,
            prompt: None,
            server: None,
            server_user: "opencode".into(),
            server_password: None,
        };
        let req = build_request(args).unwrap();
        assert!(req.model.is_none());
        assert!(req.repo.is_none());
        assert!(req.prompt.is_none());
    }

    // ── progress_indicates_success ──

    #[test]
    fn progress_indicates_success_step_failed_false() {
        let prog = LifecycleProgress::StepFailed {
            step: "workspace-prepare".into(),
            error: "oops".into(),
        };
        assert!(!progress_indicates_success(&prog));
    }

    #[test]
    fn progress_indicates_success_finished_success_true() {
        let prog = LifecycleProgress::Finished {
            result: StepResult::Success,
            message: None,
        };
        assert!(progress_indicates_success(&prog));
    }

    #[test]
    fn progress_indicates_success_finished_failure_false() {
        let prog = LifecycleProgress::Finished {
            result: StepResult::Failure,
            message: None,
        };
        assert!(!progress_indicates_success(&prog));
    }

    #[test]
    fn progress_indicates_success_step_started_true() {
        let prog = LifecycleProgress::StepStarted {
            step: "workspace-prepare".into(),
            started_at: "2024-01-01T00:00:00Z".into(),
        };
        assert!(progress_indicates_success(&prog));
    }

    // ── Tests to kill mutation survivors ──

    #[tokio::test]
    async fn drain_progress_returns_false_on_step_failure() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        tx.send(LifecycleProgress::Initialized {
            bead_id: BeadId::parse("dp-fail").unwrap(),
            steps: vec![],
        }).await.unwrap();
        tx.send(LifecycleProgress::StepFailed {
            step: "workspace-prepare".into(),
            error: "boom".into(),
        }).await.unwrap();
        tx.send(LifecycleProgress::Finished {
            result: StepResult::Success,
            message: None,
        }).await.unwrap();
        drop(tx);
        let result = drain_progress(&mut rx).await;
        assert!(!result, "drain_progress should return false when a StepFailed occurs, even if Finished(Success) comes later");
    }

    #[tokio::test]
    async fn drain_progress_and_vs_or_operator() {
        // With &&: StepFailed flips succeeded to false, subsequent Success can't flip it back
        // With || mutant: StepFailed flips succeeded to false, then Success || false = true
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        tx.send(LifecycleProgress::Initialized {
            bead_id: BeadId::parse("dp-and").unwrap(),
            steps: vec![],
        }).await.unwrap();
        tx.send(LifecycleProgress::StepFailed {
            step: "s1".into(),
            error: "fail".into(),
        }).await.unwrap();
        tx.send(LifecycleProgress::Finished {
            result: StepResult::Success,
            message: None,
        }).await.unwrap();
        drop(tx);
        let result = drain_progress(&mut rx).await;
        assert!(!result, "&& operator: StepFailed should permanently fail the result");
    }

    #[tokio::test]
    async fn drain_progress_returns_true_on_all_success() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        tx.send(LifecycleProgress::Initialized {
            bead_id: BeadId::parse("dp-ok").unwrap(),
            steps: vec![],
        }).await.unwrap();
        tx.send(LifecycleProgress::StepStarted {
            step: "s1".into(),
            started_at: "now".into(),
        }).await.unwrap();
        tx.send(LifecycleProgress::Finished {
            result: StepResult::Success,
            message: None,
        }).await.unwrap();
        drop(tx);
        let result = drain_progress(&mut rx).await;
        assert!(result);
    }
}
