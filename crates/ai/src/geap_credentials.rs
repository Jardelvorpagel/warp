use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local};
use warp_core::ui::Icon;
use warp_multi_agent_api as api;

/// Short-lived Google Cloud credentials for Gemini Enterprise (GEAP) inference.
///
/// The access token is minted by exchanging a Warp-issued OIDC identity token
/// through GCP Workload Identity Federation (STS token exchange, optionally
/// followed by service account impersonation). No long-lived Google credential
/// material is ever stored; only this short-lived access token is attached to
/// requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeapCredentials {
    access_token: String,
    expires_at: Option<SystemTime>,
}

impl GeapCredentials {
    pub fn new(access_token: String, expires_at: Option<SystemTime>) -> Self {
        Self {
            access_token,
            expires_at,
        }
    }

    pub fn expires_at(&self) -> Option<SystemTime> {
        self.expires_at
    }

    /// Whether the token is expired or about to expire (within a minute).
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            expires_at
                .duration_since(SystemTime::now())
                .map(|remaining| remaining <= Duration::from_secs(60))
                .unwrap_or(true)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeapCredentialsState {
    Missing,
    Disabled,
    Refreshing,
    Loaded {
        credentials: GeapCredentials,
        loaded_at: SystemTime,
    },
    Failed {
        message: String,
    },
}

impl From<GeapCredentials> for api::request::settings::api_keys::GoogleCloudCredentials {
    fn from(credentials: GeapCredentials) -> Self {
        Self {
            access_token: credentials.access_token,
        }
    }
}

fn format_status_timestamp(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    if datetime.date_naive() == Local::now().date_naive() {
        datetime.format("%-I:%M %p").to_string()
    } else {
        datetime.format("%b %-d at %-I:%M %p").to_string()
    }
}

impl GeapCredentialsState {
    pub fn user_facing_components(&self) -> (String, String, Icon) {
        match self {
            Self::Missing => (
                "Google Cloud federation not configured".to_string(),
                "Configure the Gemini Enterprise workload identity federation settings (project number, pool, and provider), then refresh."
                    .to_string(),
                Icon::Key,
            ),
            Self::Disabled => (
                "Gemini Enterprise disabled".to_string(),
                "Warp will not mint Google Cloud credentials until Gemini Enterprise is enabled by your workspace admin."
                    .to_string(),
                Icon::Key,
            ),
            Self::Refreshing => (
                "Refreshing credentials...".to_string(),
                "Exchanging your Warp identity for Google Cloud credentials".to_string(),
                Icon::RefreshCw04,
            ),
            Self::Loaded {
                credentials,
                loaded_at,
            } => (
                "Credentials loaded".to_string(),
                match credentials.expires_at() {
                    Some(expires_at) => format!(
                        "Loaded at {}, expires {}",
                        format_status_timestamp(*loaded_at),
                        format_status_timestamp(expires_at)
                    ),
                    None => format!("Loaded at {}", format_status_timestamp(*loaded_at)),
                },
                Icon::CheckCircleBroken,
            ),
            Self::Failed { message } => (
                "Unable to load Google Cloud credentials".to_string(),
                message.clone(),
                Icon::AlertTriangle,
            ),
        }
    }
}
