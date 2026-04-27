use serde::{Deserialize, Serialize};

use super::{
    JjArgs, ModelId, MoonTaskName, OpencodeUrl, PromptString, SensitiveString, StepName, Username,
    WorkspaceName, WorkspacePath,
};

pub const OPENCODE_TIMEOUT_SECS: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    WorkspacePrepare {
        workspace: WorkspaceName,
        path: WorkspacePath,
    },
    Jj {
        args: JjArgs,
        cwd: Option<WorkspacePath>,
    },
    MoonRun {
        task: MoonTaskName,
        cwd: Option<WorkspacePath>,
    },
    MoonCi {
        cwd: Option<WorkspacePath>,
    },
    Opencode {
        prompt: PromptString,
        model: ModelId,
        cwd: Option<WorkspacePath>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpencodeServerConfig {
    pub url: OpencodeUrl,
    pub username: Username,
    pub password: SensitiveString,
}

impl Effect {
    pub fn program(&self) -> &'static str {
        match self {
            Self::WorkspacePrepare { .. } => "mkdir",
            Self::Jj { .. } => "jj",
            Self::MoonRun { .. } | Self::MoonCi { .. } => "moon",
            Self::Opencode { .. } => "opencode",
        }
    }

    pub fn cwd(&self) -> Option<&WorkspacePath> {
        match self {
            Self::WorkspacePrepare { .. } => None,
            Self::Jj { cwd, .. }
            | Self::MoonRun { cwd, .. }
            | Self::MoonCi { cwd, .. }
            | Self::Opencode { cwd, .. } => cwd.as_ref(),
        }
    }

    pub fn args(&self) -> Vec<String> {
        match self {
            Self::WorkspacePrepare { path, .. } => vec!["-p".to_owned(), path.0.clone()],
            Self::Jj { args, .. } => args.0.clone(),
            Self::MoonRun { task, .. } => vec!["run".to_owned(), task.0.clone()],
            Self::MoonCi { .. } => vec!["run".to_owned(), ":ci".to_owned()],
            Self::Opencode { prompt, model, .. } => opencode_args(prompt, model),
        }
    }
}

fn opencode_args(prompt: &PromptString, model: &ModelId) -> Vec<String> {
    vec![
        "run".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        "--model".to_owned(),
        model.0.clone(),
        prompt.0.clone(),
    ]
}

#[derive(Debug, Clone)]
pub struct LifecycleStep {
    pub name: StepName,
    pub effect: Effect,
}
