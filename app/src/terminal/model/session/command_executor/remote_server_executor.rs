use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::remote_server::client::RemoteServerClient;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use warp_completer::completer::{CommandExitStatus, CommandOutput};
use warp_core::command::ExitCode;
use warp_core::SessionId;

use crate::remote_server::proto::{run_command_response, RunCommandErrorCode};
use crate::terminal::model::session::command_executor::{CommandExecutor, ExecuteCommandOptions};
use crate::terminal::shell::Shell;

/// `CommandExecutor` implementation that executes commands via a persistent
/// `warp remote-server` process running on the remote host over SSH.
///
/// The executor is always constructed with a live `RemoteServerClient` that
/// was obtained from [`crate::remote_server::manager::RemoteServerManager`]
/// after the session reached the `Connected` state. The manager owns the
/// authoritative per-session client; this executor holds a cloned `Arc` to
/// the same underlying channels and transitively keeps them alive as long
/// as the `Session` is alive. If that connection later disconnects, the
/// client slot is cleared until the manager reconnects the session and
/// supplies a replacement client.
///
/// Commands issued while the client is disconnected will fail locally.
/// If the underlying SSH connection is torn down mid-session,
/// [`RemoteServerClient::run_command`] will fail naturally and
/// [`execute_command`] surfaces that as an `Err`. We deliberately do *not*
/// silently synthesize an empty `Ok(CommandOutput)` for the disconnected
/// case, because callers (notably the completions/syntax-highlighting
/// pipeline) treat `Ok(empty)` as "there are zero top-level commands" and
/// produce incorrect results.
pub struct RemoteServerCommandExecutor {
    session_id: SessionId,
    client: RwLock<Option<Arc<RemoteServerClient>>>,
}

impl std::fmt::Debug for RemoteServerCommandExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteServerCommandExecutor")
            .field("session_id", &self.session_id)
            .finish()
    }
}

impl RemoteServerCommandExecutor {
    /// Creates a new executor backed by an already-connected
    /// [`RemoteServerClient`].
    pub fn new(session_id: SessionId, client: Arc<RemoteServerClient>) -> Self {
        Self {
            session_id,
            client: RwLock::new(Some(client)),
        }
    }

    /// Reattaches this executor to the client created by a successful
    /// remote-server reconnect.
    pub(crate) fn set_client(&self, client: Arc<RemoteServerClient>) {
        *self.client.write() = Some(client);
    }

    /// Prevents new commands from being sent after the manager has observed
    /// that this session's remote-server connection is gone.
    pub(crate) fn clear_client(&self) {
        *self.client.write() = None;
    }
}

#[async_trait]
impl CommandExecutor for RemoteServerCommandExecutor {
    async fn execute_command(
        &self,
        command: &str,
        _shell: &Shell,
        current_directory_path: Option<&str>,
        environment_variables: Option<HashMap<String, String>>,
        _execute_command_options: ExecuteCommandOptions,
    ) -> Result<CommandOutput> {
        let Some(client) = self.client.read().clone() else {
            return Err(anyhow!(
                "Remote command skipped because the remote server client is not connected (session={:?})",
                self.session_id,
            ));
        };

        let response = client
            .run_command(
                self.session_id,
                command.to_owned(),
                current_directory_path.map(ToOwned::to_owned),
                environment_variables.unwrap_or_default(),
            )
            .await
            .map_err(|e| anyhow!("Remote command failed (session={:?}): {e}", self.session_id))?;

        match response.result {
            Some(run_command_response::Result::Success(success)) => {
                let status = match success.exit_code {
                    Some(0) => CommandExitStatus::Success,
                    _ => CommandExitStatus::Failure,
                };
                Ok(CommandOutput {
                    stdout: success.stdout,
                    stderr: success.stderr,
                    status,
                    exit_code: success.exit_code.map(ExitCode::from),
                })
            }
            Some(run_command_response::Result::Error(err)) => {
                if err.code() == RunCommandErrorCode::SessionNotFound {
                    warp_core::safe_error!(
                        safe: ("Remote command SESSION_NOT_FOUND — SessionBootstrapped notification likely lost"),
                        full: ("Remote command SESSION_NOT_FOUND (session={:?}): {} — the SessionBootstrapped notification was likely lost", self.session_id, err.message)
                    );
                }
                Err(anyhow!(
                    "Remote command error (session={:?}, code={:?}): {}",
                    self.session_id,
                    err.code(),
                    err.message,
                ))
            }
            None => {
                warp_core::safe_error!(
                    safe: ("Remote command returned empty response — proto-level bug"),
                    full: ("Remote command returned empty response (session={:?}) — proto-level bug", self.session_id)
                );
                Err(anyhow!(
                    "Remote command returned empty response (session={:?})",
                    self.session_id,
                ))
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// The remote server multiplexes commands over a single SSH connection,
    /// so parallel execution is safe (unlike `RemoteCommandExecutor` which
    /// opens a new SSH session per command and is limited by `MaxSessions`).
    fn supports_parallel_command_execution(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "remote_server_executor_tests.rs"]
mod tests;
