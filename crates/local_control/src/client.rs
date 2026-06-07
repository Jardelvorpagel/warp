//! Blocking client helpers used by the standalone `warpctrl` CLI.
//!
//! Authentication is a two-transport flow:
//!
//! 1. Discovery supplies instance metadata, an exact `127.0.0.1` control
//!    endpoint, and an instance-bound credential-broker socket reference. It
//!    never supplies a bearer credential.
//! 2. Before using either reference, the client validates that the endpoint is
//!    loopback and that the broker filename is derived from the selected
//!    instance ID.
//! 3. The client requests a credential for one action over the owner-only
//!    broker socket. On Unix, the server authenticates the
//!    connecting process through kernel-reported peer credentials before
//!    issuing a short-lived, action-scoped credential.
//! 4. The client keeps that credential in memory and presents it as a bearer
//!    token only to the selected instance's loopback HTTP endpoint. The running
//!    Warp app revalidates the credential, current settings, action scope, and
//!    request before dispatch.
//!
//! Client-side validation prevents accidental use of inconsistent discovery
//! authority, but it is not the authorization boundary. The broker and running
//! app enforce authorization, and credentials must never be written to
//! discovery records, logs, or command output.
#[cfg(unix)]
use std::io::{Read as _, Write as _};
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::path::Path;

use crate::auth::{CredentialRequest, ScopedCredential};
use crate::discovery::InstanceRecord;
use crate::protocol::{
    Action, ActionKind, ControlError, ControlResponse, ErrorCode, ErrorResponseEnvelope,
    RequestEnvelope, ResponseEnvelope,
};
use instant::Instant;

/// Requests an action-scoped credential and sends one authenticated control request.
#[cfg(not(target_family = "wasm"))]
pub fn send_request(
    instance: &InstanceRecord,
    request: &RequestEnvelope,
) -> Result<ResponseEnvelope, ControlError> {
    let total_started = Instant::now();
    let action = request.action.kind;
    let authority_started = Instant::now();
    let authority = instance.validate_local_control_authority();
    crate::timing::emit_action_result(
        "client.authority_validation",
        action,
        authority_started.elapsed(),
        authority.is_ok(),
    );
    authority?;
    let credential_started = Instant::now();
    let credential = request_credential(instance, action);
    crate::timing::emit_action_result(
        "client.credential",
        action,
        credential_started.elapsed(),
        credential.is_ok(),
    );
    let credential = credential?;
    let endpoint = instance.endpoint.as_ref().ok_or_else(|| {
        ControlError::new(
            ErrorCode::LocalControlDisabled,
            "local control endpoint is disabled for this instance",
        )
    })?;
    let client_started = Instant::now();
    let client = reqwest::blocking::Client::new();
    crate::timing::emit_action_result("client.http_client", action, client_started.elapsed(), true);
    let send_started = Instant::now();
    let response = client
        .post(endpoint.url())
        .header("Authorization", credential.authorization_value())
        .json(request)
        .send()
        .map_err(|err| {
            ControlError::with_details(
                ErrorCode::TransportUnavailable,
                "failed to send local-control request",
                err.to_string(),
            )
        });
    crate::timing::emit_http(
        "client.http_send",
        action,
        send_started.elapsed(),
        response
            .as_ref()
            .ok()
            .map(|response| response.status().as_u16()),
        response.is_ok(),
    );
    let response = response?;
    let status = response.status();
    let body_started = Instant::now();
    let text = response.text().map_err(|err| {
        ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "failed to read local-control response",
            err.to_string(),
        )
    });
    crate::timing::emit_http(
        "client.http_body",
        action,
        body_started.elapsed(),
        Some(status.as_u16()),
        text.is_ok(),
    );
    let text = text?;
    let decode_started = Instant::now();
    let result = if let Ok(envelope) = serde_json::from_str::<ResponseEnvelope>(&text) {
        if let ControlResponse::Error { error } = &envelope.response {
            Err(error.clone())
        } else {
            Ok(envelope)
        }
    } else if let Ok(envelope) = serde_json::from_str::<ErrorResponseEnvelope>(&text) {
        Err(envelope.error)
    } else {
        Err(ControlError::with_details(
            ErrorCode::TransportUnavailable,
            format!("local-control request failed with HTTP {status}"),
            text,
        ))
    };
    crate::timing::emit_action_result(
        "client.response_decode",
        action,
        decode_started.elapsed(),
        result.is_ok(),
    );
    crate::timing::emit_action_result(
        "client.request_total",
        action,
        total_started.elapsed(),
        result.is_ok(),
    );
    result
}

/// Fails closed on platforms without a native local-control HTTP transport.
#[cfg(target_family = "wasm")]
pub fn send_request(
    instance: &InstanceRecord,
    request: &RequestEnvelope,
) -> Result<ResponseEnvelope, ControlError> {
    request_credential(instance, request.action.kind)?;
    Err(ControlError::new(
        ErrorCode::LocalControlDisabled,
        "local control requires a native HTTP transport",
    ))
}
#[cfg(unix)]
/// Resolves the selected instance's validated broker path and requests a credential.
fn request_credential_over_owner_ipc(
    instance: &InstanceRecord,
    request: &CredentialRequest,
) -> Result<String, ControlError> {
    let path = instance.broker_socket_path()?;
    let started = Instant::now();
    let response = request_credential_over_socket(&path, request);
    crate::timing::emit_action_result(
        "client.broker_round_trip",
        request.action,
        started.elapsed(),
        response.is_ok(),
    );
    response
}

#[cfg(unix)]
/// Exchanges one credential request and response over an owner-authenticated socket.
///
/// Shutting down the write half delimits the JSON request so the broker can
/// read it to EOF before returning either a scoped credential or a structured
/// error response.
fn request_credential_over_socket(
    path: &Path,
    request: &CredentialRequest,
) -> Result<String, ControlError> {
    let mut stream = UnixStream::connect(path).map_err(|err| {
        ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "failed to connect to the owner-authenticated local-control credential broker",
            err.to_string(),
        )
    })?;
    let request = serde_json::to_vec(request).map_err(|err| {
        ControlError::with_details(
            ErrorCode::InvalidRequest,
            "failed to serialize local-control credential request",
            err.to_string(),
        )
    })?;
    stream.write_all(&request).map_err(|err| {
        ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "failed to write local-control credential request",
            err.to_string(),
        )
    })?;
    stream.shutdown(Shutdown::Write).map_err(|err| {
        ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "failed to finish local-control credential request",
            err.to_string(),
        )
    })?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|err| {
        ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "failed to read local-control credential response",
            err.to_string(),
        )
    })?;
    Ok(response)
}

#[cfg(not(unix))]
/// Fails closed on platforms without an owner-authenticated broker transport.
fn request_credential_over_owner_ipc(
    _instance: &InstanceRecord,
    _request: &CredentialRequest,
) -> Result<String, ControlError> {
    Err(ControlError::new(
        ErrorCode::LocalControlDisabled,
        "local control requires an owner-authenticated credential broker",
    ))
}

/// Requests and decodes a short-lived credential for one exact action.
pub fn request_credential(
    instance: &InstanceRecord,
    action: crate::protocol::ActionKind,
) -> Result<ScopedCredential, ControlError> {
    let started = Instant::now();
    instance.validate_local_control_authority()?;
    let request = CredentialRequest::new(action);
    let text = request_credential_over_owner_ipc(instance, &request)?;
    let result = if let Ok(credential) = serde_json::from_str::<ScopedCredential>(&text) {
        Ok(credential)
    } else if let Ok(envelope) = serde_json::from_str::<ErrorResponseEnvelope>(&text) {
        Err(envelope.error)
    } else {
        Err(ControlError::with_details(
            ErrorCode::TransportUnavailable,
            "local-control credential broker returned an invalid response",
            text,
        ))
    };
    crate::timing::emit_action_result(
        "client.credential_total",
        action,
        started.elapsed(),
        result.is_ok(),
    );
    result
}

/// Authenticates an app-ping request and verifies the selected instance is live.
pub fn probe_instance(instance: &InstanceRecord) -> Result<(), ControlError> {
    let response = send_request(
        instance,
        &RequestEnvelope::new(Action::new(ActionKind::AppPing)),
    )?;
    validate_probe_response(instance, response)
}

/// Rejects a health response that does not prove the selected instance identity.
fn validate_probe_response(
    instance: &InstanceRecord,
    response: ResponseEnvelope,
) -> Result<(), ControlError> {
    let ControlResponse::Ok { data } = response.response else {
        return Err(ControlError::new(
            ErrorCode::TransportUnavailable,
            "local-control health probe returned an error response",
        ));
    };
    if data.get("instance_id").and_then(serde_json::Value::as_str)
        != Some(instance.instance_id.0.as_str())
    {
        return Err(ControlError::new(
            ErrorCode::TransportUnavailable,
            "local-control health probe returned a different instance identity",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
