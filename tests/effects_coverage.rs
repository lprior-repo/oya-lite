use oya_lite::lifecycle::types::*;

#[test]
fn effect_workspace_prepare_debug() {
    let e = Effect::WorkspacePrepare {
        workspace: "w".into(),
        path: "/tmp".into(),
    };
    let s = format!("{:?}", e);
    assert!(s.contains("WorkspacePrepare"));
}

#[test]
fn effect_jj_debug() {
    let e = Effect::Jj {
        args: JjArgs(vec!["status".into()]),
        cwd: None,
    };
    let s = format!("{:?}", e);
    assert!(s.contains("Jj"));
}

#[test]
fn effect_moon_run_debug() {
    let e = Effect::MoonRun {
        task: "build".into(),
        cwd: Some("/p".into()),
    };
    let s = format!("{:?}", e);
    assert!(s.contains("MoonRun"));
}

#[test]
fn effect_moon_ci_debug() {
    let e = Effect::MoonCi { cwd: None };
    let s = format!("{:?}", e);
    assert!(s.contains("MoonCi"));
}

#[test]
fn effect_opencode_debug() {
    let e = Effect::Opencode {
        prompt: "p".into(),
        model: "m".into(),
        cwd: None,
    };
    let s = format!("{:?}", e);
    assert!(s.contains("Opencode"));
}

#[test]
fn effect_eq() {
    let a = Effect::WorkspacePrepare {
        workspace: "w".into(),
        path: "/tmp".into(),
    };
    let b = Effect::WorkspacePrepare {
        workspace: "w".into(),
        path: "/tmp".into(),
    };
    let c = Effect::WorkspacePrepare {
        workspace: "w".into(),
        path: "/other".into(),
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effect_jj_eq() {
    let a = Effect::Jj {
        args: JjArgs(vec!["status".into()]),
        cwd: None,
    };
    let b = Effect::Jj {
        args: JjArgs(vec!["status".into()]),
        cwd: None,
    };
    let c = Effect::Jj {
        args: JjArgs(vec!["log".into()]),
        cwd: None,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effect_moon_run_eq() {
    let a = Effect::MoonRun {
        task: "build".into(),
        cwd: Some("/p".into()),
    };
    let b = Effect::MoonRun {
        task: "build".into(),
        cwd: Some("/p".into()),
    };
    let c = Effect::MoonRun {
        task: "test".into(),
        cwd: Some("/p".into()),
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effect_moon_ci_eq() {
    let a = Effect::MoonCi { cwd: None };
    let b = Effect::MoonCi { cwd: None };
    let c = Effect::MoonCi { cwd: Some("/other".into()) };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effect_opencode_eq() {
    let a = Effect::Opencode {
        prompt: "p".into(),
        model: "m".into(),
        cwd: None,
    };
    let b = Effect::Opencode {
        prompt: "p".into(),
        model: "m".into(),
        cwd: None,
    };
    let c = Effect::Opencode {
        prompt: "different".into(),
        model: "m".into(),
        cwd: None,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effect_program() {
    assert_eq!(
        Effect::WorkspacePrepare {
            workspace: "w".into(),
            path: "/tmp".into()
        }
        .program(),
        "mkdir"
    );
    assert_eq!(
        Effect::Jj {
            args: JjArgs(vec![]),
            cwd: None,
        }
        .program(),
        "jj"
    );
    assert_eq!(
        Effect::MoonRun {
            task: "build".into(),
            cwd: None,
        }
        .program(),
        "moon"
    );
    assert_eq!(Effect::MoonCi { cwd: None }.program(), "moon");
    assert_eq!(
        Effect::Opencode {
            prompt: "p".into(),
            model: "m".into(),
            cwd: None,
        }
        .program(),
        "opencode"
    );
}

#[test]
fn effect_cwd() {
    assert!(Effect::WorkspacePrepare {
        workspace: "w".into(),
        path: "/tmp".into()
    }
    .cwd()
    .is_none());

    assert!(Effect::Jj {
        args: JjArgs(vec![]),
        cwd: Some("/home".into()),
    }
    .cwd()
    .is_some());

    assert!(Effect::MoonRun {
        task: "build".into(),
        cwd: None,
    }
    .cwd()
    .is_none());

    assert!(Effect::MoonCi { cwd: None }.cwd().is_none());

    assert!(Effect::Opencode {
        prompt: "p".into(),
        model: "m".into(),
        cwd: None,
    }
    .cwd()
    .is_none());
}

#[test]
fn effect_args_workspace_prepare() {
    let e = Effect::WorkspacePrepare {
        workspace: "ws".into(),
        path: "/tmp/workspace".into(),
    };
    let args = e.args();
    assert!(args.contains(&"-p".to_owned()));
    assert!(args.contains(&"/tmp/workspace".to_owned()));
}

#[test]
fn effect_args_jj() {
    let e = Effect::Jj {
        args: JjArgs(vec!["status".into(), "-r".into()]),
        cwd: None,
    };
    let args = e.args();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "status");
    assert_eq!(args[1], "-r");
}

#[test]
fn effect_args_moon_run() {
    let e = Effect::MoonRun {
        task: "build".into(),
        cwd: None,
    };
    let args = e.args();
    assert_eq!(args[0], "run");
    assert_eq!(args[1], "build");
}

#[test]
fn effect_args_moon_ci() {
    let e = Effect::MoonCi { cwd: None };
    let args = e.args();
    assert_eq!(args[0], "run");
    assert_eq!(args[1], ":ci");
}

#[test]
fn effect_args_opencode() {
    let e = Effect::Opencode {
        prompt: "fix the bug".into(),
        model: "gpt-4".into(),
        cwd: None,
    };
    let args = e.args();
    assert!(args.contains(&"run".to_owned()));
    assert!(args.contains(&"--format".to_owned()));
    assert!(args.contains(&"json".to_owned()));
    assert!(args.contains(&"--model".to_owned()));
    assert!(args.contains(&"gpt-4".to_owned()));
    assert!(args.contains(&"fix the bug".to_owned()));
}

#[test]
fn lifecycle_step_debug() {
    let step = LifecycleStep {
        name: StepName("test-step".into()),
        effect: Effect::WorkspacePrepare {
            workspace: "ws".into(),
            path: "/tmp".into(),
        },
    };
    let s = format!("{:?}", step);
    assert!(s.contains("test-step"));
    assert!(s.contains("WorkspacePrepare"));
}

#[test]
fn opencode_server_config_debug() {
    let config = OpencodeServerConfig {
        url: "http://localhost:4099".into(),
        username: "user".into(),
        password: SensitiveString("secret".into()),
    };
    let s = format!("{:?}", config);
    assert!(s.contains("localhost"));
    assert!(s.contains("user"));
    assert!(s.contains("***"));
}

#[test]
fn effect_clone() {
    let e = Effect::Opencode {
        prompt: "p".into(),
        model: "m".into(),
        cwd: None,
    };
    let cloned = e.clone();
    assert_eq!(e, cloned);
}

#[test]
fn effect_serialize() -> Result<(), Box<dyn std::error::Error>> {
    let e = Effect::WorkspacePrepare {
        workspace: "ws".into(),
        path: "/tmp".into(),
    };
    let json = serde_json::to_string(&e)?;
    let back: Effect = serde_json::from_str(&json)?;
    assert_eq!(e, back);
    Ok(())
}

#[test]
fn effect_serialize_opencode() -> Result<(), Box<dyn std::error::Error>> {
    let e = Effect::Opencode {
        prompt: "hello".into(),
        model: "gpt-4".into(),
        cwd: Some("/workspace".into()),
    };
    let json = serde_json::to_string(&e)?;
    // Effect is tagged, so it contains "Opencode" (capitalized)
    assert!(json.contains("Opencode"));
    let back: Effect = serde_json::from_str(&json)?;
    assert_eq!(e, back);
    Ok(())
}

#[test]
fn sensitive_string_debug_hides_value() {
    let pw = SensitiveString("my-secret".into());
    let s = format!("{:?}", pw);
    assert!(s.contains("***"));
    assert!(!s.contains("my-secret"));
}

#[test]
fn sensitive_string_as_str() {
    let pw = SensitiveString("secret123".into());
    assert_eq!(pw.as_str(), "secret123");
}

#[test]
fn jj_args_newtype() {
    let args = JjArgs(vec!["status".into(), "log".into()]);
    assert_eq!(args.0.len(), 2);
    assert_eq!(args.0[0], "status");
}

#[test]
fn step_result_is_success() {
    assert!(StepResult::Success.is_success());
    assert!(!StepResult::Failure.is_success());
}

#[test]
fn step_result_eq() {
    assert_eq!(StepResult::Success, StepResult::Success);
    assert_eq!(StepResult::Failure, StepResult::Failure);
    assert_ne!(StepResult::Success, StepResult::Failure);
}

#[test]
fn step_result_serde() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&StepResult::Success)?;
    assert!(json.contains("success"));
    let back: StepResult = serde_json::from_str(&json)?;
    assert_eq!(back, StepResult::Success);
    Ok(())
}