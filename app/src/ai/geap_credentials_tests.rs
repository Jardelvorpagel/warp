use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{TimeZone, Utc};

use super::*;

fn identity_token(expires_at: chrono::DateTime<Utc>) -> TaskIdentityToken {
    TaskIdentityToken {
        token: "warp-oidc-jwt".to_string(),
        expires_at,
        issuer: "https://app.warp.dev".to_string(),
    }
}

#[test]
fn config_from_parts_uses_audience_and_service_account() {
    let audience = "//iam.googleapis.com/projects/123456789/locations/global/workloadIdentityPools/warp-team-pool/providers/warp-oidc-provider";
    let config =
        GeapWifConfig::from_parts(audience, "warp-vertex@my-project.iam.gserviceaccount.com")
            .unwrap();

    assert_eq!(config.workload_identity_audience(), audience);
    assert_eq!(
        config.impersonation_url().unwrap(),
        "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/warp-vertex@my-project.iam.gserviceaccount.com:generateAccessToken"
    );
}

#[test]
fn config_from_parts_trims_whitespace_and_omits_empty_sa() {
    let config = GeapWifConfig::from_parts("  //iam.googleapis.com/aud  ", "  ").unwrap();

    assert_eq!(
        config.workload_identity_audience(),
        "//iam.googleapis.com/aud"
    );
    // A whitespace-only service account email means no impersonation.
    assert_eq!(config.impersonation_url(), None);
}

#[test]
fn config_from_parts_requires_audience() {
    assert_eq!(
        GeapWifConfig::from_parts("", "warp-vertex@x.iam.gserviceaccount.com"),
        None
    );
    assert_eq!(GeapWifConfig::from_parts("   ", ""), None);
}
#[test]
fn sts_response_parses_with_expiry() {
    let response: StsTokenExchangeResponse = serde_json::from_str(
        r#"{
            "access_token": "federated-token",
            "issued_token_type": "urn:ietf:params:oauth:token-type:access_token",
            "token_type": "Bearer",
            "expires_in": 3600
        }"#,
    )
    .unwrap();

    assert_eq!(response.access_token, "federated-token");
    assert_eq!(response.expires_in, Some(3600));
}

#[test]
fn sts_response_parses_without_expiry() {
    let response: StsTokenExchangeResponse = serde_json::from_str(
        r#"{
            "access_token": "federated-token",
            "token_type": "Bearer"
        }"#,
    )
    .unwrap();

    assert_eq!(response.access_token, "federated-token");
    assert_eq!(response.expires_in, None);
}

#[test]
fn sts_token_expiry_uses_expires_in_when_present() {
    let response = StsTokenExchangeResponse {
        access_token: "federated-token".to_string(),
        expires_in: Some(3600),
    };
    let token = identity_token(Utc::now());

    let expires_at = sts_token_expiry(&response, &token).unwrap();
    let remaining = expires_at.duration_since(SystemTime::now()).unwrap();
    assert!(remaining > Duration::from_secs(3590));
    assert!(remaining <= Duration::from_secs(3600));
}

#[test]
fn sts_token_expiry_falls_back_to_identity_token_expiry() {
    let jwt_expiry = Utc.with_ymd_and_hms(2026, 6, 9, 23, 30, 0).unwrap();
    let response = StsTokenExchangeResponse {
        access_token: "federated-token".to_string(),
        expires_in: None,
    };
    let token = identity_token(jwt_expiry);

    assert_eq!(
        sts_token_expiry(&response, &token),
        Some(SystemTime::from(jwt_expiry))
    );
}

#[test]
fn generate_access_token_response_parses_camel_case() {
    let response: GenerateAccessTokenResponse = serde_json::from_str(
        r#"{
            "accessToken": "impersonated-token",
            "expireTime": "2026-06-09T23:30:00Z"
        }"#,
    )
    .unwrap();

    assert_eq!(response.access_token, "impersonated-token");
    let expires_at = parse_rfc3339_to_system_time(&response.expire_time).unwrap();
    let expected = UNIX_EPOCH
        + Duration::from_secs(
            Utc.with_ymd_and_hms(2026, 6, 9, 23, 30, 0)
                .unwrap()
                .timestamp() as u64,
        );
    assert_eq!(expires_at, expected);
}

#[test]
fn parse_rfc3339_rejects_invalid_timestamps() {
    assert_eq!(parse_rfc3339_to_system_time("not-a-timestamp"), None);
    assert_eq!(parse_rfc3339_to_system_time(""), None);
}
