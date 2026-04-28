use crate::lifecycle::effects::run::opencode_output_is_error;
use crate::lifecycle::error::LifecycleError;
use crate::lifecycle::types::{
    Effect, EffectJournalEntry, ModelId, OpencodeServerConfig, PromptString, SensitiveString,
    StepResult, Username, WorkspacePath, OPENCODE_TIMEOUT_SECS,
};

#[derive(serde::Serialize)]
struct OpencodeMessageBody<'a> {
    parts: Vec<serde_json::Value>,
    #[serde(rename = "providerID")]
    provider_id: &'a str,
    #[serde(rename = "modelID")]
    model_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<&'a str>,
}

async fn server_create_session(
    config: &OpencodeServerConfig,
    prompt: &PromptString,
    client: &reqwest::Client,
) -> std::result::Result<String, LifecycleError> {
    let session_url = format!("{}/session", config.url.as_str().trim_end_matches('/'));
    let session_resp = client
        .post(&session_url)
        .basic_auth(config.username.as_str(), Some(config.password.as_str()))
        .json(&create_session_body(prompt))
        .send()
        .await
        .map_err(|e| {
            LifecycleError::transient(
                crate::lifecycle::error::FailureCategory::Command,
                format!("opencode server session create failed: {e}"),
            )
        })?;
    parse_session_response(session_resp).await
}

fn create_session_body(prompt: &PromptString) -> serde_json::Value {
    serde_json::json!({
        "title": format!("oya-lite: {}", prompt.as_str().len().min(40))
    })
}

async fn parse_session_response(
    session_resp: reqwest::Response,
) -> std::result::Result<String, LifecycleError> {
    if !session_resp.status().is_success() {
        let error_msg = read_error_body(session_resp).await;
        return Err(LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Command,
            error_msg,
        ));
    }
    extract_session_id(session_resp).await
}

async fn read_error_body(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = match resp.text().await {
        Ok(text) => text,
        Err(e) => format!("unable to read response body: {e}"),
    };
    let body = body.chars().take(200).collect::<String>();
    format!("opencode server returned {status}: {body}")
}

async fn extract_session_id(
    session_resp: reqwest::Response,
) -> std::result::Result<String, LifecycleError> {
    let session_data: serde_json::Value = session_resp.json().await.map_err(|e| {
        LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Command,
            format!("failed to parse session response: {e}"),
        )
    })?;
    session_data
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            LifecycleError::terminal(
                crate::lifecycle::error::FailureCategory::Command,
                "opencode server response missing session id",
            )
        })
}

fn build_message_body<'a>(
    prompt: &'a PromptString,
    model: &'a ModelId,
    cwd: Option<&'a WorkspacePath>,
) -> OpencodeMessageBody<'a> {
    let parts = model.as_str().split('/').collect::<Vec<_>>();
    let (provider_id, model_id) = match parts.as_slice() {
        [provider, model, ..] => (*provider, *model),
        [single] => ("anthropic", *single),
        [] => ("anthropic", "unknown"),
    };
    OpencodeMessageBody {
        parts: vec![serde_json::json!({"type": "text", "text": prompt.as_str()})],
        provider_id,
        model_id,
        cwd: cwd.map(|p| p.as_str()),
    }
}

fn parse_opencode_effect(
    effect: &Effect,
) -> std::result::Result<(&PromptString, &ModelId, Option<&WorkspacePath>), LifecycleError> {
    match effect {
        Effect::Opencode { prompt, model, cwd } => Ok((prompt, model, cwd.as_ref())),
        _ => Err(LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Validation,
            "invalid effect for send message",
        )),
    }
}

async fn send_opencode_request(
    client: &reqwest::Client,
    url: &str,
    username: &Username,
    password: &SensitiveString,
    body: &OpencodeMessageBody<'_>,
) -> std::result::Result<reqwest::Response, LifecycleError> {
    client
        .post(url)
        .basic_auth(username.as_str(), Some(password.as_str()))
        .json(body)
        .send()
        .await
        .map_err(|e| {
            LifecycleError::transient(
                crate::lifecycle::error::FailureCategory::Command,
                format!("opencode server message send failed: {e}"),
            )
        })
}

fn parse_message_response(status: reqwest::StatusCode, body: String) -> (StepResult, String) {
    let result = if status.is_success() && !opencode_output_is_error(&body, "") {
        StepResult::Success
    } else {
        StepResult::Failure
    };
    (result, body.chars().take(4096).collect())
}

async fn server_send_message(
    config: &OpencodeServerConfig,
    session_id: &str,
    effect: &Effect,
    client: &reqwest::Client,
) -> std::result::Result<(StepResult, String), LifecycleError> {
    let (prompt, model, cwd) = parse_opencode_effect(effect)?;
    let message_body = build_message_body(prompt, model, cwd);
    let msg_url = build_message_url(config, session_id);
    let msg_resp = send_message(config, client, &msg_url, &message_body).await?;
    read_message_response(msg_resp).await
}

fn build_message_url(config: &OpencodeServerConfig, session_id: &str) -> String {
    format!(
        "{}/session/{}/message",
        config.url.as_str().trim_end_matches('/'),
        session_id
    )
}

async fn send_message(
    config: &OpencodeServerConfig,
    client: &reqwest::Client,
    msg_url: &str,
    message_body: &OpencodeMessageBody<'_>,
) -> std::result::Result<reqwest::Response, LifecycleError> {
    send_opencode_request(
        client,
        msg_url,
        &config.username,
        &config.password,
        message_body,
    )
    .await
}

async fn read_message_response(
    msg_resp: reqwest::Response,
) -> std::result::Result<(StepResult, String), LifecycleError> {
    let msg_status = msg_resp.status();
    let msg_body = msg_resp.text().await.map_err(|e| {
        LifecycleError::transient(
            crate::lifecycle::error::FailureCategory::Command,
            format!("opencode server response read failed: {e}"),
        )
    })?;
    Ok(parse_message_response(msg_status, msg_body))
}

fn create_opencode_journal_entry(
    prompt: &PromptString,
    model: &ModelId,
    cwd: &Option<WorkspacePath>,
    result: StepResult,
    msg_body: String,
) -> EffectJournalEntry {
    EffectJournalEntry {
        effect: Effect::Opencode {
            prompt: prompt.clone(),
            model: model.clone(),
            cwd: cwd.clone(),
        },
        timeout_secs: OPENCODE_TIMEOUT_SECS,
        result,
        stdout: journal_stdout(&result, &msg_body),
        stderr: journal_stderr(&result, msg_body),
    }
}

fn journal_stdout(result: &StepResult, msg_body: &str) -> String {
    if result.is_success() {
        msg_body.to_owned()
    } else {
        String::new()
    }
}

fn journal_stderr(result: &StepResult, msg_body: String) -> String {
    if result.is_success() {
        String::new()
    } else {
        msg_body
    }
}

pub(super) async fn run_opencode_server(
    config: &OpencodeServerConfig,
    effect: &Effect,
) -> std::result::Result<EffectJournalEntry, LifecycleError> {
    let Effect::Opencode { prompt, model, cwd } = effect else {
        return Err(LifecycleError::terminal(
            crate::lifecycle::error::FailureCategory::Validation,
            "invalid effect",
        ));
    };
    let client = reqwest::Client::new();
    let session_id = server_create_session(config, prompt, &client).await?;
    let (result, msg_body) = server_send_message(config, &session_id, effect, &client).await?;
    Ok(create_opencode_journal_entry(
        prompt, model, cwd, result, msg_body,
    ))
}

// ─── TESTS ───────────────────────────────────────────────────────────────────

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{
        JjArgs, ModelId, OpencodeUrl, PromptString, SensitiveString, Username, OPENCODE_TIMEOUT_SECS,
    };
    use reqwest::StatusCode;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn config() -> OpencodeServerConfig {
        OpencodeServerConfig {
            url: OpencodeUrl("http://localhost:4099".into()),
            username: Username("user".into()),
            password: SensitiveString("pass".into()),
        }
    }

    // ── create_session_body ──

    #[test]
    fn create_session_body_contains_title() {
        let prompt = PromptString("fix the bug".into());
        let body = create_session_body(&prompt);
        let title = body.get("title").unwrap().as_str().unwrap();
        assert!(title.starts_with("oya-lite:"));
    }

    #[test]
    fn create_session_body_truncates_long_prompt() {
        let long_prompt = PromptString("x".repeat(100));
        let body = create_session_body(&long_prompt);
        let title = body.get("title").unwrap().as_str().unwrap();
        // title is "oya-lite: " (9 chars) + up to 40 chars of prompt
        assert!(title.len() <= 49);
    }

    #[test]
    fn create_session_body_with_empty_prompt() {
        let prompt = PromptString("".into());
        let body = create_session_body(&prompt);
        let title = body.get("title").unwrap().as_str().unwrap();
        assert_eq!(title, "oya-lite: 0");
    }

    #[test]
    fn create_session_body_exactly_40_chars() {
        let prompt = PromptString("a".repeat(40));
        let body = create_session_body(&prompt);
        let title = body.get("title").unwrap().as_str().unwrap();
        // "oya-lite: " (9 chars) + "40" (2 chars) = 11 total
        assert_eq!(title, "oya-lite: 40");
    }

    #[test]
    fn create_session_body_41_chars_shows_40() {
        let prompt = PromptString("a".repeat(41));
        let body = create_session_body(&prompt);
        let title = body.get("title").unwrap().as_str().unwrap();
        // Should show 40 since .min(40) caps at 40
        assert_eq!(title, "oya-lite: 40");
    }

    // ── build_message_body ──

    #[test]
    fn build_message_body_parses_model_with_provider() {
        let prompt = PromptString("hello".into());
        let model = ModelId("anthropic/claude-3".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "anthropic");
        assert_eq!(body.model_id, "claude-3");
        assert!(body.parts[0].as_object().unwrap().contains_key("text"));
    }

    #[test]
    fn build_message_body_single_part_defaults_to_anthropic() {
        let prompt = PromptString("hi".into());
        let model = ModelId("claude-3".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "anthropic");
        assert_eq!(body.model_id, "claude-3");
    }

    #[test]
    fn build_message_body_single_segment_defaults_anthropic() {
        let prompt = PromptString("".into());
        let model = ModelId("claude-3".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "anthropic");
        assert_eq!(body.model_id, "claude-3");
    }

    #[test]
    fn build_message_body_two_segment_parses_provider_and_model() {
        let prompt = PromptString("hi".into());
        let model = ModelId("anthropic/claude-3".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "anthropic");
        assert_eq!(body.model_id, "claude-3");
    }

    #[test]
    fn build_message_body_three_segment_uses_first_two() {
        let prompt = PromptString("hi".into());
        let model = ModelId("a/b/c".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "a");
        assert_eq!(body.model_id, "b");
    }

    #[test]
    fn build_message_body_with_cwd() {
        let prompt = PromptString("pwd".into());
        let model = ModelId("gpt-4".into());
        let cwd = Some(WorkspacePath("/tmp".into()));
        let body = build_message_body(&prompt, &model, cwd.as_ref());
        assert_eq!(body.cwd, Some("/tmp"));
    }

    #[test]
    fn build_message_body_with_empty_prompt() {
        let prompt = PromptString("".into());
        let model = ModelId("gpt-4".into());
        let body = build_message_body(&prompt, &model, None);
        assert!(body.parts[0].as_object().unwrap().contains_key("text"));
    }

    #[test]
    fn build_message_body_cwd_none() {
        let prompt = PromptString("hello".into());
        let model = ModelId("gpt-4".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.cwd, None);
    }

    #[test]
    fn build_message_body_four_segment_uses_first_two() {
        let prompt = PromptString("hi".into());
        let model = ModelId("a/b/c/d".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "a");
        assert_eq!(body.model_id, "b");
    }

    #[test]
    fn build_message_body_leading_slash_means_empty_provider() {
        // "/model" splits to ["", "model"] → matches [provider, model, ..] with provider=""
        let prompt = PromptString("test".into());
        let model = ModelId("/model".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "");
        assert_eq!(body.model_id, "model");
    }

    #[test]
    fn build_message_body_trailing_slash_means_empty_model() {
        // "provider/" splits to ["provider", ""] → matches [provider, model, ..] with model=""
        let prompt = PromptString("test".into());
        let model = ModelId("provider/".into());
        let body = build_message_body(&prompt, &model, None);
        assert_eq!(body.provider_id, "provider");
        assert_eq!(body.model_id, "");
    }

    #[test]
    fn build_message_body_with_cwd_none_when_not_provided() {
        let prompt = PromptString("test".into());
        let model = ModelId("gpt-4".into());
        let body = build_message_body(&prompt, &model, None);
        assert!(body.cwd.is_none());
    }

    // ── parse_opencode_effect ──

    #[test]
    fn parse_opencode_effect_accepts_opencode() {
        let effect = Effect::Opencode {
            prompt: PromptString("p".into()),
            model: ModelId("m".into()),
            cwd: None,
        };
        let (p, m, c) = parse_opencode_effect(&effect).unwrap();
        assert_eq!(p.as_str(), "p");
        assert_eq!(m.as_str(), "m");
        assert!(c.is_none());
    }

    #[test]
    fn parse_opencode_effect_accepts_opencode_with_cwd() {
        let effect = Effect::Opencode {
            prompt: PromptString("prompt".into()),
            model: ModelId("model".into()),
            cwd: Some(WorkspacePath("/workspace".into())),
        };
        let (p, m, c) = parse_opencode_effect(&effect).unwrap();
        assert_eq!(p.as_str(), "prompt");
        assert_eq!(m.as_str(), "model");
        assert!(c.is_some());
        assert_eq!(c.unwrap().as_str(), "/workspace");
    }

    #[test]
    fn parse_opencode_effect_rejects_workspace_prepare() {
        let effect = Effect::WorkspacePrepare { workspace: "w".into(), path: "/tmp".into() };
        let result = parse_opencode_effect(&effect);
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_effect_rejects_jj() {
        let effect = Effect::Jj { args: JjArgs(vec![]), cwd: None };
        let result = parse_opencode_effect(&effect);
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_effect_rejects_moon_run() {
        let effect = Effect::MoonRun { task: "t".into(), cwd: None };
        let result = parse_opencode_effect(&effect);
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_effect_rejects_moon_ci() {
        let effect = Effect::MoonCi { cwd: None };
        let result = parse_opencode_effect(&effect);
        assert!(result.is_err());
    }

    // ── parse_message_response ──

    #[test]
    fn parse_message_response_success_when_200_and_no_error() {
        let body = r#"{"type":"text","content":"hello"}"#;
        let (result, out) = parse_message_response(StatusCode::OK, body.into());
        assert!(matches!(result, StepResult::Success));
        assert!(out.contains("hello"));
    }

    #[test]
    fn parse_message_response_failure_when_500() {
        let (result, _) = parse_message_response(StatusCode::INTERNAL_SERVER_ERROR, "server error".into());
        assert!(matches!(result, StepResult::Failure));
    }

    #[test]
    fn parse_message_response_failure_when_error_json() {
        let body = r#"{"type":"error","message":"model not found"}"#;
        let (result, _) = parse_message_response(StatusCode::OK, body.into());
        assert!(matches!(result, StepResult::Failure));
    }

    #[test]
    fn parse_message_response_truncates_long_body() {
        let body = "x".repeat(5000);
        let (_, out) = parse_message_response(StatusCode::OK, body.clone());
        assert_eq!(out.len(), 4096);
    }

    #[test]
    fn parse_message_response_failure_when_400() {
        let (result, _) = parse_message_response(StatusCode::BAD_REQUEST, "bad request".into());
        assert!(matches!(result, StepResult::Failure));
    }

    #[test]
    fn parse_message_response_failure_when_401() {
        let (result, _) = parse_message_response(StatusCode::UNAUTHORIZED, "unauthorized".into());
        assert!(matches!(result, StepResult::Failure));
    }

    #[test]
    fn parse_message_response_success_empty_body() {
        let (result, out) = parse_message_response(StatusCode::OK, "".into());
        assert!(matches!(result, StepResult::Success));
        assert_eq!(out, "");
    }

    #[test]
    fn parse_message_response_non_json_body() {
        let body = "plain text response";
        let (result, _) = parse_message_response(StatusCode::OK, body.into());
        assert!(matches!(result, StepResult::Success));
    }

    // ── build_message_url ──

    #[test]
    fn build_message_url_formats_correctly() {
        let cfg = config();
        let url = build_message_url(&cfg, "abc123");
        assert_eq!(url, "http://localhost:4099/session/abc123/message");
    }

    #[test]
    fn build_message_url_strips_trailing_slash() {
        let mut cfg = config();
        cfg.url = OpencodeUrl("http://localhost:4099/".into());
        let url = build_message_url(&cfg, "sid");
        assert_eq!(url, "http://localhost:4099/session/sid/message");
    }

    // ── create_opencode_journal_entry ──

    #[test]
    fn create_opencode_journal_entry_success_stores_in_stdout() {
        let entry = create_opencode_journal_entry(
            &PromptString("hello".into()),
            &ModelId("gpt-4".into()),
            &None,
            StepResult::Success,
            "model response text".into(),
        );
        assert!(entry.result.is_success());
        assert_eq!(entry.stdout, "model response text");
        assert!(entry.stderr.is_empty());
        assert_eq!(entry.timeout_secs, OPENCODE_TIMEOUT_SECS);
    }

    #[test]
    fn create_opencode_journal_entry_failure_stores_in_stderr() {
        let entry = create_opencode_journal_entry(
            &PromptString("hi".into()),
            &ModelId("gpt-4".into()),
            &None,
            StepResult::Failure,
            "error message".into(),
        );
        assert!(!entry.result.is_success());
        assert!(entry.stdout.is_empty());
        assert_eq!(entry.stderr, "error message");
    }

    #[test]
    fn create_opencode_journal_entry_includes_effect() {
        let entry = create_opencode_journal_entry(
            &PromptString("p".into()),
            &ModelId("m".into()),
            &Some(WorkspacePath("/cwd".into())),
            StepResult::Success,
            "ok".into(),
        );
        match &entry.effect {
            Effect::Opencode { prompt, model, cwd } => {
                assert_eq!(prompt.as_str(), "p");
                assert_eq!(model.as_str(), "m");
                assert!(cwd.is_some());
            }
            _ => panic!("expected Opencode effect"),
        }
    }

    // ── journal_stdout / journal_stderr ──

    #[test]
    fn journal_stdout_returns_body_on_success() {
        assert_eq!(journal_stdout(&StepResult::Success, "response"), "response");
    }

    #[test]
    fn journal_stdout_empty_on_failure() {
        assert_eq!(journal_stdout(&StepResult::Failure, "response"), "");
    }

    #[test]
    fn journal_stderr_empty_on_success() {
        assert_eq!(journal_stderr(&StepResult::Success, "response".to_string()), "");
    }

    #[test]
    fn journal_stderr_returns_body_on_failure() {
        assert_eq!(journal_stderr(&StepResult::Failure, "error".to_string()), "error");
    }

    // ── Tests to kill mutation survivors ──

    #[tokio::test]
    async fn run_opencode_server_rejects_missing_session_id() {
        // Create a mock HTTP server that returns JSON without an "id" field
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            // Read the HTTP request (reqwest sends ~200 bytes for a POST)
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            // Write a valid HTTP response with JSON missing "id" field
            let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"no_id\": \"here\"}";
            socket.write_all(response).await.unwrap();
            // Close the socket to signal EOF to the client
            let _ = socket.shutdown().await;
        });

        let cfg = OpencodeServerConfig {
            url: OpencodeUrl(format!("http://{addr}")),
            username: Username("u".into()),
            password: SensitiveString("p".into()),
        };
        let effect = Effect::Opencode {
            prompt: PromptString("test".into()),
            model: ModelId("gpt-4".into()),
            cwd: None,
        };
        let result = run_opencode_server(&cfg, &effect).await;
        assert!(result.is_err(), "should reject response missing session id");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("session id") || err.to_string().contains("parse session response"));
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn run_opencode_server_rejects_non_success_session_response() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            let response = b"HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\nContent-Length: 13\r\n\r\nUnauthorized";
            socket.write_all(response).await.unwrap();
            let _ = socket.shutdown().await;
        });

        let cfg = OpencodeServerConfig {
            url: OpencodeUrl(format!("http://{addr}")),
            username: Username("u".into()),
            password: SensitiveString("p".into()),
        };
        let effect = Effect::Opencode {
            prompt: PromptString("test".into()),
            model: ModelId("gpt-4".into()),
            cwd: None,
        };
        let result = run_opencode_server(&cfg, &effect).await;
        assert!(result.is_err(), "should reject non-success session response");
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("401") || err_str.contains("Unauthorized"),
            "error message must contain HTTP status/body from read_error_body, got: {err_str}");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn run_opencode_server_rejects_500_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            let response = b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 15\r\n\r\nserver crashed!";
            socket.write_all(response).await.unwrap();
            let _ = socket.shutdown().await;
        });

        let cfg = OpencodeServerConfig {
            url: OpencodeUrl(format!("http://{addr}")),
            username: Username("u".into()),
            password: SensitiveString("p".into()),
        };
        let effect = Effect::Opencode {
            prompt: PromptString("test".into()),
            model: ModelId("gpt-4".into()),
            cwd: None,
        };
        let result = run_opencode_server(&cfg, &effect).await;
        assert!(result.is_err(), "should reject 500 server error");
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("500") && err_str.contains("server crashed"),
            "error must contain 500 status and body from read_error_body, got: {err_str}");
        server_handle.await.unwrap();
    }
}
