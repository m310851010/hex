//! Where the dictation rewrite gets its model. OpenCode is the original
//! local-service path; the two direct providers let a person configure an
//! OpenAI-compatible or Anthropic endpoint themselves, with no service to
//! run. The API key lives in `settings.json` in plain text by choice: the
//! person asked for `hex settings get` to show it, and the UI masks it.

use serde::{Deserialize, Serialize};

/// Which service answers a rewrite request.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    /// The running OpenCode service found through `service.json`.
    #[default]
    OpenCode,
    /// Any OpenAI-compatible `/chat/completions` endpoint.
    OpenAi,
    /// Anthropic's `/messages` endpoint.
    Anthropic,
}

impl LlmProvider {
    pub const ALL: [Self; 3] = [Self::OpenCode, Self::OpenAi, Self::Anthropic];

    /// The English name the window shows; the caller translates it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenCode => "OpenCode",
            Self::OpenAi => "OpenAI compatible",
            Self::Anthropic => "Anthropic",
        }
    }

    /// A direct provider sends its own requests; OpenCode is discovered.
    pub const fn direct(self) -> bool {
        !matches!(self, Self::OpenCode)
    }

    /// Where requests start for a fresh config of this provider.
    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenCode => "",
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
        }
    }
}

/// The configured rewrite service. `#[serde(default)]` keeps older
/// settings.json files unchanged; the default is the original OpenCode path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct LlmSettings {
    pub provider: LlmProvider,
    /// Base URL of the direct endpoint, without a trailing slash.
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenCode,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
        }
    }
}

impl LlmSettings {
    /// Whether a direct request has everything it needs. An incomplete
    /// config is not an error: the rewrite simply falls back to the
    /// corrected transcript.
    pub fn ready(&self) -> bool {
        self.provider.direct()
            && !self.base_url.trim().is_empty()
            && !self.api_key.trim().is_empty()
            && !self.model.trim().is_empty()
    }

    /// A freshly selected provider's own defaults, keeping the key when
    /// the person already typed one.
    pub fn for_provider(provider: LlmProvider, key: &str) -> Self {
        Self {
            provider,
            base_url: provider.default_base_url().to_string(),
            api_key: key.to_string(),
            model: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_original_opencode_path() {
        let settings = LlmSettings::default();
        assert_eq!(settings.provider, LlmProvider::OpenCode);
        assert!(!settings.ready());
        let decoded: LlmSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn a_direct_provider_needs_all_three_fields() {
        let mut settings = LlmSettings::for_provider(LlmProvider::OpenAi, "sk-test");
        assert!(!settings.ready(), "no model yet");
        settings.model = "gpt-4o-mini".into();
        assert!(settings.ready());
        settings.api_key.clear();
        assert!(!settings.ready());
    }

    #[test]
    fn settings_round_trip_in_plain_text() {
        let settings = LlmSettings {
            provider: LlmProvider::Anthropic,
            base_url: "https://api.anthropic.com/v1".into(),
            api_key: "sk-ant-secret".into(),
            model: "claude-3-5-sonnet".into(),
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        assert!(encoded.contains("sk-ant-secret"));
        assert_eq!(
            serde_json::from_str::<LlmSettings>(&encoded).unwrap(),
            settings
        );
    }
}
