//! Opt-in phase timing diagnostics for the standalone `warpctrl` client.
use std::sync::OnceLock;
use std::time::Duration;

use crate::protocol::ActionKind;

/// Environment variable that enables safe `warpctrl` timing output on stderr.
pub const DEBUG_TIMING_ENV: &str = "WARPCTRL_DEBUG_TIMING";

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Returns whether opt-in client timing diagnostics are enabled.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var(DEBUG_TIMING_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false)
    })
}

/// Emits a phase duration without request-specific fields.
pub fn emit(phase: &str, elapsed: Duration) {
    if enabled() {
        eprintln!(
            "warpctrl timing phase={phase} elapsed_ms={:.3}",
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}

/// Emits a phase duration and success state without request-specific fields.
pub fn emit_result(phase: &str, elapsed: Duration, success: bool) {
    if enabled() {
        eprintln!(
            "warpctrl timing phase={phase} elapsed_ms={:.3} success={success}",
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}

/// Emits a phase duration and safe discovered-record count.
pub fn emit_count(phase: &str, elapsed: Duration, count: usize) {
    if enabled() {
        eprintln!(
            "warpctrl timing phase={phase} elapsed_ms={:.3} count={count}",
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}

/// Emits a phase duration with a safe action name and success state.
pub fn emit_action_result(phase: &str, action: ActionKind, elapsed: Duration, success: bool) {
    if enabled() {
        eprintln!(
            "warpctrl timing phase={phase} action={} elapsed_ms={:.3} success={success}",
            action.as_str(),
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}

/// Emits an HTTP phase duration with a safe action name, status, and success state.
pub fn emit_http(
    phase: &str,
    action: ActionKind,
    elapsed: Duration,
    status: Option<u16>,
    success: bool,
) {
    if enabled() {
        let status = status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "none".to_owned());
        eprintln!(
            "warpctrl timing phase={phase} action={} elapsed_ms={:.3} status={status} success={success}",
            action.as_str(),
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}
