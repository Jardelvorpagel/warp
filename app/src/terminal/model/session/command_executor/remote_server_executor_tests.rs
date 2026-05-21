use std::sync::Arc;

use async_channel::TryRecvError;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use warp_core::SessionId;
use warpui::r#async::executor;

use super::*;
use crate::terminal::shell::ShellType;

#[tokio::test]
async fn disconnected_executor_skips_run_command_before_reaching_the_client() {
    let (client_stream, _server_stream) = tokio::io::duplex(4096);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let background_executor = executor::Background::default();
    let (client, _event_rx, failure_rx) = RemoteServerClient::new(
        client_read.compat(),
        client_write.compat_write(),
        &background_executor,
    );
    let client = Arc::new(client);
    let command_executor =
        RemoteServerCommandExecutor::new(SessionId::from(42u64), Arc::clone(&client));
    command_executor.clear_client();

    let shell = Shell::new(ShellType::Zsh, None, None, Default::default(), None);
    let error = command_executor
        .execute_command(
            "echo should-not-run",
            &shell,
            None,
            None,
            ExecuteCommandOptions::default(),
        )
        .await
        .expect_err("detached executors should fail locally");

    assert!(
        error.to_string().contains("not connected"),
        "unexpected detached executor error: {error:#}"
    );
    assert!(matches!(failure_rx.try_recv(), Err(TryRecvError::Empty)));
}
