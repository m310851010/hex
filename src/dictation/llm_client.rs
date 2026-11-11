//! Direct LLM providers for the dictation rewrite. When a person configures
//! an OpenAI-compatible or Anthropic endpoint, `processor::generate_cancellable`
//! calls here instead of discovering the OpenCode service. Both protocols
//! receive the same assembled prompt (`processor::prompt`), so the mode's
//! Instructions decide whether this is "only remove fillers" or "polish".
//!
//! Requests go through `net` (platform TLS, the computer's proxy, a shared
//! deadline), never through a child process. Logs name only the host: no
//! key, body, or dictated text leaves through tracing here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;

use crate::app_settings::{LlmProvider, LlmSettings};

const MAX_RESPONSE_BYTES: u64 = 16 << 20;
const ANTHROPIC_VERSION: &str = "2023-06-01";
const SYSTEM_PROMPT: &str = "You rewrite dictated speech. Return only the replacement text, with no explanation or Markdown fence.";

/// Sends the assembled prompt and returns the model's text.
pub fn generate(
    prompt: &str,
    settings: &LlmSettings,
    deadline: Duration,
    cancelled: &AtomicBool,
) -> Result<String> {
    if cancelled.load(Ordering::Acquire) {
        bail!("LLM generation was cancelled");
    }
    let started = Instant::now();
    let url = endpoint(settings, "/chat/completions", "/messages");
    let (auth_name, auth_value) = match settings.provider {
        LlmProvider::OpenAi => ("Authorization", format!("Bearer {}", settings.api_key)),
        LlmProvider::Anthropic => ("x-api-key", settings.api_key.clone()),
        LlmProvider::OpenCode => bail!("the OpenCode provider has no direct endpoint"),
    };
    let body = match settings.provider {
        LlmProvider::OpenAi => serde_json::json!({
            "model": settings.model,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": prompt },
            ],
            "stream": false,
        }),
        LlmProvider::Anthropic => serde_json::json!({
            "model": settings.model,
            "system": SYSTEM_PROMPT,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 4096,
            "stream": false,
        }),
        LlmProvider::OpenCode => unreachable!("checked above"),
    };
    let mut request = ureq::http::Request::builder()
        .method("POST")
        .uri(url.as_str())
        .header("Content-Type", "application/json")
        .header(auth_name, auth_value.as_str());
    if settings.provider == LlmProvider::Anthropic {
        request = request.header("anthropic-version", ANTHROPIC_VERSION);
    }
    let bytes = serde_json::to_vec(&body)?;
    let response = send(request, Some(bytes), &url, deadline, started, cancelled)?;
    parse_generation(settings.provider, &response)
}

/// The model ids a direct endpoint lists. A failure is a normal outcome:
/// the window keeps the hand-typed model field available.
pub fn list_models(settings: &LlmSettings, deadline: Duration) -> Result<Vec<String>> {
    if !settings.provider.direct() || settings.base_url.trim().is_empty() {
        bail!("configure a direct endpoint first");
    }
    let started = Instant::now();
    let url = format!("{}/models", settings.base_url.trim().trim_end_matches('/'));
    let mut request = ureq::http::Request::builder()
        .method("GET")
        .uri(url.as_str());
    match settings.provider {
        LlmProvider::OpenAi => {
            request = request.header("Authorization", format!("Bearer {}", settings.api_key));
        }
        LlmProvider::Anthropic => {
            request = request.header("x-api-key", settings.api_key.as_str());
            request = request.header("anthropic-version", ANTHROPIC_VERSION);
        }
        LlmProvider::OpenCode => unreachable!("checked above"),
    }
    let response = send(
        request,
        None,
        &url,
        deadline,
        started,
        &AtomicBool::new(false),
    )?;
    parse_models(settings.provider, &response)
}

fn endpoint(settings: &LlmSettings, open_ai: &str, anthropic: &str) -> String {
    let base = settings.base_url.trim().trim_end_matches('/');
    let path = match settings.provider {
        LlmProvider::OpenAi => open_ai,
        LlmProvider::Anthropic => anthropic,
        LlmProvider::OpenCode => unreachable!("checked by the caller"),
    };
    format!("{base}{path}")
}

fn send(
    request: ureq::http::request::Builder,
    body: Option<Vec<u8>>,
    url: &str,
    deadline: Duration,
    started: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>> {
    let left = remaining(deadline, started)?;
    let configure = |config: crate::net::Config| {
        config
            .http_status_as_error(false)
            .max_redirects(5)
            .timeout_global(Some(left))
    };
    let agent = crate::net::agent(url, configure);
    let bytes = body.unwrap_or_default();
    let built = request
        .body(bytes)
        .map_err(|error| eyre!("invalid LLM request: {error}"))?;
    let mut response = agent
        .run(built)
        .map_err(|error| eyre!("{}", crate::net::unreachable(url, &error)))?;
    if cancelled.load(Ordering::Acquire) {
        bail!("LLM generation was cancelled");
    }
    remaining(deadline, started)?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_vec()
        .map_err(|error| eyre!("could not read the LLM response: {error}"))?;
    if !(200..300).contains(&status) {
        let detail = String::from_utf8_lossy(&text);
        let detail = detail.chars().take(300).collect::<String>();
        bail!("LLM endpoint answered HTTP {status}: {detail}");
    }
    tracing::info!(host = %crate::net::host(url), status, "direct LLM request completed");
    Ok(text)
}

fn remaining(deadline: Duration, started: Instant) -> Result<Duration> {
    let left = deadline.saturating_sub(started.elapsed());
    if left.is_zero() {
        bail!("LLM generation exceeded {} seconds", deadline.as_secs());
    }
    Ok(left)
}

#[derive(Deserialize)]
struct OpenAiGeneration {
    choices: Vec<OpenAiChoice>,
}
#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}
#[derive(Deserialize)]
struct OpenAiMessage {
    content: String,
}
#[derive(Deserialize)]
struct AnthropicGeneration {
    content: Vec<AnthropicContent>,
}
#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(default)]
    text: String,
}
#[derive(Deserialize)]
struct OpenAiModels {
    data: Vec<OpenAiModel>,
}
#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}
#[derive(Deserialize)]
struct AnthropicModels {
    data: Vec<AnthropicModel>,
}
#[derive(Deserialize)]
struct AnthropicModel {
    id: String,
}

fn parse_generation(provider: LlmProvider, bytes: &[u8]) -> Result<String> {
    let text = match provider {
        LlmProvider::OpenAi => {
            let response: OpenAiGeneration = serde_json::from_slice(bytes)
                .map_err(|error| eyre!("invalid OpenAI-compatible response: {error}"))?;
            response
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message.content)
                .unwrap_or_default()
        }
        LlmProvider::Anthropic => {
            let response: AnthropicGeneration = serde_json::from_slice(bytes)
                .map_err(|error| eyre!("invalid Anthropic response: {error}"))?;
            response
                .content
                .into_iter()
                .map(|content| content.text)
                .collect::<String>()
        }
        LlmProvider::OpenCode => unreachable!("checked by the caller"),
    };
    if text.trim().is_empty() {
        bail!("the LLM returned empty text");
    }
    Ok(text)
}

fn parse_models(provider: LlmProvider, bytes: &[u8]) -> Result<Vec<String>> {
    let mut models: Vec<String> = match provider {
        LlmProvider::OpenAi => serde_json::from_slice::<OpenAiModels>(bytes)
            .map(|response| response.data.into_iter().map(|model| model.id).collect())
            .unwrap_or_default(),
        LlmProvider::Anthropic => serde_json::from_slice::<AnthropicModels>(bytes)
            .map(|response| response.data.into_iter().map(|model| model.id).collect())
            .unwrap_or_default(),
        LlmProvider::OpenCode => unreachable!("checked by the caller"),
    };
    models.sort();
    models.dedup();
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_parses_both_protocols() {
        assert_eq!(
            parse_generation(
                LlmProvider::OpenAi,
                br#"{"choices":[{"message":{"content":"hello"}}]}"#,
            )
            .unwrap(),
            "hello"
        );
        assert_eq!(
            parse_generation(
                LlmProvider::Anthropic,
                br#"{"content":[{"type":"text","text":"hello"}]}"#,
            )
            .unwrap(),
            "hello"
        );
        assert!(
            parse_generation(
                LlmProvider::OpenAi,
                br#"{"choices":[{"message":{"content":"  "}}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn model_lists_parse_sorted_and_unique() {
        assert_eq!(
            parse_models(
                LlmProvider::OpenAi,
                br#"{"data":[{"id":"b"},{"id":"a"},{"id":"b"}]}"#,
            )
            .unwrap(),
            ["a", "b"]
        );
        assert_eq!(
            parse_models(
                LlmProvider::Anthropic,
                br#"{"data":[{"id":"claude-3-5-sonnet"}]}"#,
            )
            .unwrap(),
            ["claude-3-5-sonnet"]
        );
    }

    #[test]
    fn a_cancelled_generation_does_not_send() {
        let settings = LlmSettings::for_provider(LlmProvider::OpenAi, "sk-test");
        let cancelled = AtomicBool::new(true);
        assert!(generate("x", &settings, Duration::from_secs(5), &cancelled).is_err());
    }

    #[test]
    fn endpoints_follow_the_provider() {
        let mut settings = LlmSettings::for_provider(LlmProvider::OpenAi, "k");
        assert_eq!(
            endpoint(&settings, "/chat/completions", "/messages"),
            "https://api.openai.com/v1/chat/completions"
        );
        settings.provider = LlmProvider::Anthropic;
        settings.base_url = "https://api.anthropic.com/v1/".into();
        assert_eq!(
            endpoint(&settings, "/chat/completions", "/messages"),
            "https://api.anthropic.com/v1/messages"
        );
    }
}
