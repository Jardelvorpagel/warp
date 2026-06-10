//! Gemini Enterprise (GEAP) credential minting via GCP Workload Identity Federation.
//!
//! Unlike AWS Bedrock's local-credential-chain path, GEAP credentials never come from
//! local cloud config: Warp exchanges a Warp-issued OIDC identity token for a
//! short-lived Google Cloud access token. The exchange is the standard WIF flow:
//!
//! 1. Mint a Warp OIDC JWT scoped to the customer's workload identity pool provider
//!    (`ManagedSecretManager::issue_task_identity_token`).
//! 2. Exchange it for a federated access token at the GCP Security Token Service
//!    (RFC 8693 token exchange).
//! 3. Optionally impersonate a customer service account via the IAM Credentials API
//!    to obtain the final access token.
//!
//! Only the resulting short-lived access token is kept (in memory) and attached to
//! agent requests; no refresh tokens, ADC files, or service account keys are involved.

use std::time::{Duration, SystemTime};

pub use ai::api_keys::GeapCredentials;
use ai::api_keys::{ApiKeyManager, GeapCredentialsState};
use futures::channel::oneshot::channel;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use vec1::vec1;
use warp_managed_secrets::client::IdentityTokenOptions;
use warp_managed_secrets::{ManagedSecretManager, TaskIdentityToken};
use warpui::{ModelContext, SingletonEntity};

use crate::workspaces::user_workspaces::{UserWorkspaces, UserWorkspacesEvent};
use crate::workspaces::workspace::LlmHostSettings;

pub(crate) const GEAP_CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// Requested lifetime for the Warp OIDC identity token.
const GEAP_IDENTITY_TOKEN_DURATION: Duration = Duration::from_secs(60 * 60);

/// Lifetime requested for the impersonated service account access token.
const GEAP_IMPERSONATED_TOKEN_LIFETIME_SECS: u64 = 3600;

/// Re-mint credentials when the cached token expires within this window.
const GEAP_REFRESH_EXPIRY_SKEW: Duration = Duration::from_secs(5 * 60);

const GCP_STS_TOKEN_URL: &str = "https://sts.googleapis.com/v1/token";
const TOKEN_EXCHANGE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const REQUESTED_TOKEN_TYPE_ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";
const SUBJECT_TOKEN_TYPE_ID_TOKEN: &str = "urn:ietf:params:oauth:token-type:id_token";

/// Workload identity federation parameters for minting GEAP credentials.
///
/// These are admin-configured per team and synced to the client through the
/// `GEMINI_ENTERPRISE` workspace host settings. The `audience` is the full workload
/// identity provider resource name, which already encodes the customer's project
/// number, pool, and provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeapWifConfig {
    /// Full workload identity federation audience
    /// (`//iam.googleapis.com/projects/.../providers/...`).
    audience: String,
    /// Service account email for impersonation. When `None`, the federated STS
    /// token is used directly.
    service_account_email: Option<String>,
}

impl GeapWifConfig {
    /// Builds a config from raw values, trimming whitespace. Returns `None` when the
    /// audience is empty. An empty service account email means "no impersonation".
    pub fn from_parts(audience: &str, service_account_email: &str) -> Option<Self> {
        let audience = audience.trim();
        let service_account_email = service_account_email.trim();
        if audience.is_empty() {
            return None;
        }
        Some(Self {
            audience: audience.to_string(),
            service_account_email: (!service_account_email.is_empty())
                .then(|| service_account_email.to_string()),
        })
    }

    fn from_host_settings(host_settings: &LlmHostSettings) -> Option<Self> {
        Self::from_parts(
            host_settings.gcp_audience.as_deref().unwrap_or_default(),
            host_settings.gcp_sa_email.as_deref().unwrap_or_default(),
        )
    }

    /// The workload identity federation audience. This is both the `aud` claim of the
    /// Warp OIDC token and the `audience` of the STS exchange, so the GCP provider must
    /// list it as an allowed audience.
    pub fn workload_identity_audience(&self) -> String {
        self.audience.clone()
    }

    fn impersonation_url(&self) -> Option<String> {
        self.service_account_email.as_ref().map(|email| {
            format!(
                "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{email}:generateAccessToken"
            )
        })
    }
}

/// Errors that can occur when minting GEAP credentials through workload identity federation.
#[derive(Debug, Clone)]
pub enum LoadGeapCredentialsError {
    /// The Warp server refused to mint an OIDC identity token.
    MintIdentityToken(String),
    /// The GCP STS token exchange failed.
    ExchangeToken(String),
    /// Service account impersonation failed.
    ImpersonateServiceAccount(String),
}

impl std::fmt::Display for LoadGeapCredentialsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MintIdentityToken(message) => {
                write!(f, "Failed to mint a Warp identity token: {message}")
            }
            Self::ExchangeToken(message) => write!(
                f,
                "Google Cloud STS token exchange failed: {message}. Check that the workload \
                 identity pool provider exists, its issuer matches Warp's OIDC issuer, and its \
                 allowed audiences include the provider resource name."
            ),
            Self::ImpersonateServiceAccount(message) => write!(
                f,
                "Service account impersonation failed: {message}. Check that the service account \
                 email is correct and that the workload identity pool has the \
                 `roles/iam.workloadIdentityUser` binding on it."
            ),
        }
    }
}

impl std::error::Error for LoadGeapCredentialsError {}

/// Request body for the GCP STS token exchange (RFC 8693).
/// See <https://cloud.google.com/iam/docs/reference/sts/rest/v1/TopLevel/token>.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StsTokenExchangeRequest<'a> {
    grant_type: &'a str,
    audience: &'a str,
    scope: &'a str,
    requested_token_type: &'a str,
    subject_token: &'a str,
    subject_token_type: &'a str,
}

/// Response body for the GCP STS token exchange. OAuth-style snake_case fields.
#[derive(Debug, Deserialize)]
pub(crate) struct StsTokenExchangeResponse {
    pub(crate) access_token: String,
    /// Lifetime in seconds. Absent in some configurations, in which case we fall
    /// back to the Warp identity token's expiry as a conservative bound.
    #[serde(default)]
    pub(crate) expires_in: Option<u64>,
}

/// Request body for the IAM Credentials `generateAccessToken` call.
#[derive(Debug, Serialize)]
struct GenerateAccessTokenRequest<'a> {
    scope: [&'a str; 1],
    lifetime: String,
}

/// Response body for the IAM Credentials `generateAccessToken` call.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateAccessTokenResponse {
    pub(crate) access_token: String,
    /// RFC 3339 timestamp at which the token expires.
    pub(crate) expire_time: String,
}

pub(crate) fn parse_rfc3339_to_system_time(value: &str) -> Option<SystemTime> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(SystemTime::from)
}

/// Reads an HTTP error response body for an actionable message without ever
/// including request credentials.
async fn error_detail(response: http_client::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let body = body.trim();
    if body.is_empty() {
        format!("HTTP {status}")
    } else {
        // Cap the body length so a misbehaving proxy can't flood logs/UI.
        let body: String = body.chars().take(512).collect();
        format!("HTTP {status}: {body}")
    }
}

/// Exchanges a Warp OIDC identity token for a short-lived Google Cloud access token.
///
/// `identity_token` is the Warp-signed OIDC JWT whose audience is
/// [`GeapWifConfig::workload_identity_audience`].
async fn exchange_identity_token_for_geap_credentials(
    client: &http_client::Client,
    config: &GeapWifConfig,
    identity_token: TaskIdentityToken,
) -> Result<GeapCredentials, LoadGeapCredentialsError> {
    // Step 1: STS token exchange (Warp OIDC JWT -> federated access token).
    let audience = config.workload_identity_audience();
    let exchange_request = StsTokenExchangeRequest {
        grant_type: TOKEN_EXCHANGE_GRANT_TYPE,
        audience: &audience,
        scope: GEAP_CLOUD_PLATFORM_SCOPE,
        requested_token_type: REQUESTED_TOKEN_TYPE_ACCESS_TOKEN,
        subject_token: &identity_token.token,
        subject_token_type: SUBJECT_TOKEN_TYPE_ID_TOKEN,
    };
    let response = client
        .post(GCP_STS_TOKEN_URL)
        .json(&exchange_request)
        .send()
        .await
        .map_err(|err| LoadGeapCredentialsError::ExchangeToken(err.to_string()))?;
    if !response.status().is_success() {
        return Err(LoadGeapCredentialsError::ExchangeToken(
            error_detail(response).await,
        ));
    }
    let exchange_response: StsTokenExchangeResponse = response
        .json()
        .await
        .map_err(|err| LoadGeapCredentialsError::ExchangeToken(err.to_string()))?;

    let Some(impersonation_url) = config.impersonation_url() else {
        // No impersonation configured: use the federated token directly.
        let expires_at = sts_token_expiry(&exchange_response, &identity_token);
        log::info!("GEAP OIDC: federated credentials loaded (no impersonation)");
        return Ok(GeapCredentials::new(
            exchange_response.access_token,
            expires_at,
        ));
    };

    // Step 2: impersonate the configured service account.
    let impersonation_request = GenerateAccessTokenRequest {
        scope: [GEAP_CLOUD_PLATFORM_SCOPE],
        lifetime: format!("{GEAP_IMPERSONATED_TOKEN_LIFETIME_SECS}s"),
    };
    let response = client
        .post(&impersonation_url)
        .bearer_auth(&exchange_response.access_token)
        .json(&impersonation_request)
        .send()
        .await
        .map_err(|err| LoadGeapCredentialsError::ImpersonateServiceAccount(err.to_string()))?;
    if !response.status().is_success() {
        return Err(LoadGeapCredentialsError::ImpersonateServiceAccount(
            error_detail(response).await,
        ));
    }
    let impersonation_response: GenerateAccessTokenResponse = response
        .json()
        .await
        .map_err(|err| LoadGeapCredentialsError::ImpersonateServiceAccount(err.to_string()))?;

    let expires_at = parse_rfc3339_to_system_time(&impersonation_response.expire_time);
    log::info!("GEAP OIDC: impersonated service account credentials loaded");
    Ok(GeapCredentials::new(
        impersonation_response.access_token,
        expires_at,
    ))
}

/// Expiry for a federated (non-impersonated) STS token: `expires_in` when present,
/// otherwise the Warp identity token's expiry as a conservative bound.
pub(crate) fn sts_token_expiry(
    response: &StsTokenExchangeResponse,
    identity_token: &TaskIdentityToken,
) -> Option<SystemTime> {
    match response.expires_in {
        Some(seconds) => Some(SystemTime::now() + Duration::from_secs(seconds)),
        None => Some(SystemTime::from(identity_token.expires_at)),
    }
}

/// Extension trait for `ApiKeyManager` to handle GEAP credential refresh.
pub trait GeapCredentialRefresher {
    /// Sets up subscriptions to `UserWorkspaces` so GEAP credentials are (re-)minted on
    /// startup and whenever the admin-configured workspace host settings change.
    fn subscribe_to_geap_settings_changes(&mut self, ctx: &mut ModelContext<Self>)
    where
        Self: Sized;
}

impl GeapCredentialRefresher for ApiKeyManager {
    fn subscribe_to_geap_settings_changes(&mut self, ctx: &mut ModelContext<Self>) {
        // The GEAP workload identity federation config is admin-configured per team and synced
        // through the workspace host settings, so all (re-)mint triggers come from `UserWorkspaces`:
        // `TeamsChanged` initializes credentials on app startup, while `UpdateWorkspaceSettingsSuccess`
        // forces a re-mint because the audience or service account may have changed.
        ctx.subscribe_to_model(
            &UserWorkspaces::handle(ctx),
            |manager, event, ctx| match event {
                UserWorkspacesEvent::UpdateWorkspaceSettingsSuccess => {
                    drop(force_refresh_geap_credentials(manager, ctx));
                }
                UserWorkspacesEvent::TeamsChanged => {
                    drop(refresh_geap_credentials(manager, ctx));
                }
                _ => {}
            },
        );
    }
}

/// Refreshes GEAP credentials, skipping the mint when a still-valid token is loaded.
///
/// Returns a future that resolves when the refresh completes. Subscription-triggered
/// callers that don't need to wait should drop the returned future — the underlying
/// work has already been scheduled on the executor by the time this returns.
pub(crate) fn refresh_geap_credentials(
    manager: &mut ApiKeyManager,
    ctx: &mut ModelContext<ApiKeyManager>,
) -> BoxFuture<'static, Result<(), String>> {
    refresh_geap_credentials_with_options(manager, false, ctx)
}

/// Refreshes GEAP credentials unconditionally, discarding any cached token.
pub(crate) fn force_refresh_geap_credentials(
    manager: &mut ApiKeyManager,
    ctx: &mut ModelContext<ApiKeyManager>,
) -> BoxFuture<'static, Result<(), String>> {
    refresh_geap_credentials_with_options(manager, true, ctx)
}

fn refresh_geap_credentials_with_options(
    manager: &mut ApiKeyManager,
    force: bool,
    ctx: &mut ModelContext<ApiKeyManager>,
) -> BoxFuture<'static, Result<(), String>> {
    if !UserWorkspaces::as_ref(ctx).is_gemini_enterprise_credentials_enabled() {
        manager.set_geap_credentials_state(GeapCredentialsState::Disabled, ctx);
        return Box::pin(async { Ok(()) });
    }

    let Some(config) = UserWorkspaces::as_ref(ctx)
        .gemini_enterprise_host_settings()
        .and_then(GeapWifConfig::from_host_settings)
    else {
        manager.set_geap_credentials_state(GeapCredentialsState::Missing, ctx);
        return Box::pin(async { Ok(()) });
    };

    // Skip if credentials are already loaded and not about to expire.
    if !force {
        if let GeapCredentialsState::Loaded { credentials, .. } = manager.geap_credentials_state() {
            let still_valid = credentials
                .expires_at()
                .and_then(|exp| {
                    exp.duration_since(SystemTime::now() + GEAP_REFRESH_EXPIRY_SKEW)
                        .ok()
                })
                .is_some();
            if still_valid {
                log::info!("GEAP OIDC: credentials still valid, skipping refresh");
                return Box::pin(async { Ok(()) });
            }
        }
    }

    log::info!(
        "GEAP OIDC: minting credentials via workload identity federation (audience {})",
        config.workload_identity_audience()
    );
    manager.set_geap_credentials_state(GeapCredentialsState::Refreshing, ctx);

    let token_future = ManagedSecretManager::handle(ctx)
        .as_ref(ctx)
        .issue_task_identity_token(IdentityTokenOptions {
            audience: config.workload_identity_audience(),
            requested_duration: GEAP_IDENTITY_TOKEN_DURATION,
            subject_template: vec1!["principal".to_string()],
        });

    let (tx, rx) = channel();
    let _ = ctx.spawn(
        async move {
            let identity_token = token_future
                .await
                .map_err(|err| LoadGeapCredentialsError::MintIdentityToken(err.to_string()))?;
            let client = http_client::Client::new();
            exchange_identity_token_for_geap_credentials(&client, &config, identity_token).await
        },
        move |manager, result, ctx| {
            let (new_state, tx_result) = match result {
                Ok(credentials) => {
                    log::info!("GEAP OIDC: credentials loaded successfully");
                    (
                        GeapCredentialsState::Loaded {
                            credentials,
                            loaded_at: SystemTime::now(),
                        },
                        Ok(()),
                    )
                }
                Err(err) => {
                    log::error!("GEAP OIDC: failed to load credentials: {err}");
                    let message = err.to_string();
                    (
                        GeapCredentialsState::Failed {
                            message: message.clone(),
                        },
                        Err(message),
                    )
                }
            };
            manager.set_geap_credentials_state(new_state, ctx);
            let _ = tx.send(tx_result);
        },
    );
    Box::pin(async move {
        rx.await
            .unwrap_or_else(|_| Err("Credential refresh was interrupted".to_string()))
    })
}

#[cfg(test)]
#[path = "geap_credentials_tests.rs"]
mod tests;
