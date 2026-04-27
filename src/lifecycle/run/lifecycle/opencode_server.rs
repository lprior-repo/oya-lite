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
