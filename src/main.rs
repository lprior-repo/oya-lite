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
