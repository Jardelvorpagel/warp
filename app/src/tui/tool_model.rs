use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use ai::diff_validation::{AIRequestedCodeDiff, DiffType};
use anyhow::{anyhow, Result};
use chrono::Local;
use command::r#async::Command;
use itertools::Itertools;
use warp_core::command::ExitCode;
use warp_terminal::model::BlockId;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::{
    AIAgentAction, AIAgentActionId, AIAgentActionResult, AIAgentActionResultType,
    AIAgentActionType, AnyFileContent, CallMCPToolResult, CreateDocumentsResult,
    EditDocumentsResult, FetchConversationResult, FileContext, FileGlobResult, FileGlobV2Match,
    FileGlobV2Result, GrepFileMatch, GrepLineMatch, GrepResult, InsertReviewCommentsResult,
    ReadDocumentsResult, ReadFilesRequest, ReadFilesResult, ReadMCPResourceResult,
    ReadShellCommandOutputResult, ReadSkillResult, RequestCommandOutputResult,
    RequestComputerUseResult, RequestFileEditsResult, RunAgentsResult, SearchCodebaseFailureReason,
    SearchCodebaseResult, SendMessageToAgentResult, StartAgentResult, SuggestNewConversationResult,
    SuggestPromptResult, TransferShellCommandControlToUserResult, UpdatedFileContext,
    UploadArtifactResult, UseComputerResult, WaitForEventsResult,
    WriteToLongRunningShellCommandResult,
};
use crate::ai::blocklist::{apply_edits, read_local_file_context, FileReadResult, SessionContext};
use crate::ai::paths::host_native_absolute_path;
use crate::auth::AuthStateProvider;
use crate::terminal::shell::ShellType;
use crate::AuthState;

/// Minimal TUI-owned tool action model for v0 auto-executed client tools.
pub(crate) struct TuiToolActionModel {
    results_by_conversation: HashMap<AIConversationId, Vec<AIAgentActionResult>>,
    cards_by_conversation: HashMap<AIConversationId, Vec<TuiToolCard>>,
}

#[derive(Clone, Debug)]
pub(crate) struct TuiToolCard {
    pub action_id: AIAgentActionId,
    pub title: String,
    pub lines: Vec<String>,
}

pub(crate) enum TuiToolActionEvent {
    Updated { conversation_id: AIConversationId },
    ActionsFinished { conversation_id: AIConversationId },
}

impl TuiToolActionModel {
    pub fn new(_: &mut ModelContext<Self>) -> Self {
        Self {
            results_by_conversation: HashMap::new(),
            cards_by_conversation: HashMap::new(),
        }
    }

    pub fn card_for_action(
        &self,
        conversation_id: AIConversationId,
        action_id: &AIAgentActionId,
    ) -> Option<&TuiToolCard> {
        self.cards_by_conversation
            .get(&conversation_id)
            .and_then(|cards| cards.iter().find(|card| &card.action_id == action_id))
    }

    pub fn drain_finished_results(
        &mut self,
        conversation_id: AIConversationId,
    ) -> Vec<AIAgentActionResult> {
        self.results_by_conversation
            .remove(&conversation_id)
            .unwrap_or_default()
    }

    pub fn queue_actions(
        &mut self,
        actions: Vec<AIAgentAction>,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        if actions.is_empty() {
            return;
        }

        let current_working_directory = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().to_string());
        let shell_type = shell_type_from_env();
        let shell_path = std::env::var("SHELL").ok();
        let session_context = SessionContext::local(current_working_directory.clone());
        let background_executor = ctx.background_executor();
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();

        self.cards_by_conversation
            .entry(conversation_id)
            .or_default()
            .extend(actions.iter().map(|action| TuiToolCard {
                action_id: action.id.clone(),
                title: action.action.user_friendly_name(),
                lines: vec!["queued".to_string()],
            }));
        ctx.emit(TuiToolActionEvent::Updated { conversation_id });

        ctx.spawn(
            async move {
                let mut outputs = Vec::with_capacity(actions.len());
                for action in actions {
                    let output = execute_action(
                        action,
                        conversation_id,
                        current_working_directory.clone(),
                        shell_type,
                        shell_path.clone(),
                        session_context.clone(),
                        background_executor.clone(),
                        auth_state.clone(),
                    )
                    .await;
                    outputs.push(output);
                }
                outputs
            },
            move |model, outputs, ctx| {
                for output in outputs {
                    model
                        .results_by_conversation
                        .entry(conversation_id)
                        .or_default()
                        .push(output.result);
                    model.update_card(conversation_id, output.card);
                }
                ctx.emit(TuiToolActionEvent::Updated { conversation_id });
                ctx.emit(TuiToolActionEvent::ActionsFinished { conversation_id });
            },
        );
    }

    fn update_card(&mut self, conversation_id: AIConversationId, card: TuiToolCard) {
        let cards = self
            .cards_by_conversation
            .entry(conversation_id)
            .or_default();
        if let Some(existing) = cards
            .iter_mut()
            .find(|existing| existing.action_id == card.action_id)
        {
            *existing = card;
        } else {
            cards.push(card);
        }
    }
}

impl Entity for TuiToolActionModel {
    type Event = TuiToolActionEvent;
}

impl SingletonEntity for TuiToolActionModel {}

struct ExecutedAction {
    result: AIAgentActionResult,
    card: TuiToolCard,
}

async fn execute_action(
    action: AIAgentAction,
    _conversation_id: AIConversationId,
    current_working_directory: Option<String>,
    shell_type: ShellType,
    shell_path: Option<String>,
    session_context: SessionContext,
    background_executor: Arc<warpui::r#async::executor::Background>,
    auth_state: Arc<AuthState>,
) -> ExecutedAction {
    let result = match &action.action {
        AIAgentActionType::RequestCommandOutput {
            command,
            wait_until_completion,
            uses_pager,
            ..
        } => execute_command(
            command,
            *wait_until_completion,
            *uses_pager,
            current_working_directory,
            shell_type,
            shell_path,
        )
        .await
        .map(AIAgentActionResultType::RequestCommandOutput)
        .unwrap_or_else(|error| {
            AIAgentActionResultType::RequestCommandOutput(RequestCommandOutputResult::Completed {
                block_id: BlockId::new(),
                command: command.clone(),
                output: format!("{error:#}"),
                exit_code: ExitCode::from(1),
                start_ts: Some(Local::now()),
                completed_ts: Some(Local::now()),
            })
        }),
        AIAgentActionType::ReadShellCommandOutput { .. } => {
            AIAgentActionResultType::ReadShellCommandOutput(ReadShellCommandOutputResult::Error(
                crate::ai::agent::ShellCommandError::BlockNotFound,
            ))
        }
        AIAgentActionType::WriteToLongRunningShellCommand { .. } => {
            AIAgentActionResultType::WriteToLongRunningShellCommand(
                WriteToLongRunningShellCommandResult::Error(
                    crate::ai::agent::ShellCommandError::BlockNotFound,
                ),
            )
        }
        AIAgentActionType::TransferShellCommandControlToUser { .. } => {
            AIAgentActionResultType::TransferShellCommandControlToUser(
                TransferShellCommandControlToUserResult::Error(
                    crate::ai::agent::ShellCommandError::BlockNotFound,
                ),
            )
        }
        AIAgentActionType::ReadFiles(ReadFilesRequest { locations }) => {
            execute_read_files(locations.clone(), current_working_directory, None)
                .await
                .map(AIAgentActionResultType::ReadFiles)
                .unwrap_or_else(|error| {
                    AIAgentActionResultType::ReadFiles(ReadFilesResult::Error(format!("{error:#}")))
                })
        }
        AIAgentActionType::RequestFileEdits { file_edits, .. } => execute_file_edits(
            file_edits.clone(),
            &session_context,
            background_executor,
            auth_state,
        )
        .await
        .map(AIAgentActionResultType::RequestFileEdits)
        .unwrap_or_else(|error| {
            AIAgentActionResultType::RequestFileEdits(
                RequestFileEditsResult::DiffApplicationFailed {
                    error: format!("{error:#}"),
                },
            )
        }),
        AIAgentActionType::Grep { queries, path } => execute_grep(
            queries.clone(),
            path.clone(),
            current_working_directory,
            shell_type,
            shell_path,
        )
        .await
        .map(AIAgentActionResultType::Grep)
        .unwrap_or_else(|error| {
            AIAgentActionResultType::Grep(GrepResult::Error(format!("{error:#}")))
        }),
        AIAgentActionType::FileGlob { patterns, path } => {
            let search_dir = path.clone().unwrap_or_else(|| ".".to_string());
            execute_file_glob(
                patterns.clone(),
                search_dir,
                current_working_directory,
                shell_type,
                shell_path,
            )
            .await
            .map(|result| match result {
                FileGlobV2Result::Success { matched_files, .. } => {
                    AIAgentActionResultType::FileGlob(FileGlobResult::Success {
                        matched_files: matched_files.into_iter().map(|m| m.file_path).join("\n"),
                    })
                }
                FileGlobV2Result::Error(error) => {
                    AIAgentActionResultType::FileGlob(FileGlobResult::Error(error))
                }
                FileGlobV2Result::Cancelled => {
                    AIAgentActionResultType::FileGlob(FileGlobResult::Cancelled)
                }
            })
            .unwrap_or_else(|error| {
                AIAgentActionResultType::FileGlob(FileGlobResult::Error(format!("{error:#}")))
            })
        }
        AIAgentActionType::FileGlobV2 {
            patterns,
            search_dir,
        } => execute_file_glob(
            patterns.clone(),
            search_dir.clone().unwrap_or_else(|| ".".to_string()),
            current_working_directory,
            shell_type,
            shell_path,
        )
        .await
        .map(AIAgentActionResultType::FileGlobV2)
        .unwrap_or_else(|error| {
            AIAgentActionResultType::FileGlobV2(FileGlobV2Result::Error(format!("{error:#}")))
        }),
        unsupported => unsupported_result(unsupported),
    };

    let card = card_for_result(action.id.clone(), &action.action, &result);
    ExecutedAction {
        result: AIAgentActionResult {
            id: action.id,
            task_id: action.task_id,
            result,
        },
        card,
    }
}

async fn execute_command(
    command: &str,
    wait_until_completion: bool,
    uses_pager: Option<bool>,
    current_working_directory: Option<String>,
    shell_type: ShellType,
    shell_path: Option<String>,
) -> Result<RequestCommandOutputResult> {
    let command = if uses_pager == Some(true) && wait_until_completion {
        decorate_pager_command(command, shell_type)
    } else {
        command.to_string()
    };
    let block_id = BlockId::new();
    let start_ts = Local::now();
    let mut process = Command::new(shell_path.unwrap_or_else(|| shell_type.name().to_string()));
    match shell_type {
        ShellType::PowerShell => {
            process.arg("-NoProfile").arg("-Command").arg(&command);
        }
        ShellType::Fish => {
            process.arg("--no-config").arg("-c").arg(&command);
        }
        ShellType::Bash => {
            process.arg("--norc").arg("-c").arg(&command);
        }
        ShellType::Zsh => {
            process.arg("-f").arg("-c").arg(&command);
        }
    }
    if let Some(cwd) = current_working_directory {
        process.current_dir(cwd);
    }
    let output = process
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    let completed_ts = Local::now();
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    Ok(RequestCommandOutputResult::Completed {
        block_id,
        command,
        output: combined,
        exit_code: ExitCode::from(output.status.code().unwrap_or(1)),
        start_ts: Some(start_ts),
        completed_ts: Some(completed_ts),
    })
}

async fn execute_read_files(
    locations: Vec<crate::ai::agent::FileLocations>,
    current_working_directory: Option<String>,
    shell: Option<crate::terminal::ShellLaunchData>,
) -> Result<ReadFilesResult> {
    let result =
        read_local_file_context(&locations, current_working_directory, shell, None, None).await?;
    if result.missing_files.is_empty() {
        Ok(ReadFilesResult::Success {
            files: result.file_contexts,
        })
    } else {
        Ok(ReadFilesResult::Error(format!(
            "These files do not exist: {}",
            result.missing_files.join(", ")
        )))
    }
}

async fn execute_file_edits(
    file_edits: Vec<crate::ai::agent::FileEdit>,
    session_context: &SessionContext,
    background_executor: Arc<warpui::r#async::executor::Background>,
    auth_state: Arc<AuthState>,
) -> Result<RequestFileEditsResult> {
    let diffs = apply_edits(
        file_edits,
        session_context,
        &Default::default(),
        background_executor,
        auth_state,
        false,
        |path| async move { FileReadResult::from(std::fs::read_to_string(path)) },
    )
    .await
    .map_err(|errors| anyhow!(errors.iter().map(|e| format!("{e:?}")).join("\n")))?;

    apply_requested_diffs(diffs, session_context).await
}

async fn execute_grep(
    queries: Vec<String>,
    path: String,
    current_working_directory: Option<String>,
    shell_type: ShellType,
    shell_path: Option<String>,
) -> Result<GrepResult> {
    let absolute_path = absolutize(&path, current_working_directory.as_deref());
    let escaped_path = shell_escape(&absolute_path.to_string_lossy());
    let pattern = queries.iter().map(|q| shell_escape(q)).join(" -e ");
    let command = format!("grep -R -n -e {pattern} {escaped_path}");
    let output =
        run_shell_capture(&command, current_working_directory, shell_type, shell_path).await?;
    let mut by_file: HashMap<String, Vec<GrepLineMatch>> = HashMap::new();
    for line in output.lines() {
        let Some((file, rest)) = line.split_once(':') else {
            continue;
        };
        let Some((line_number, _)) = rest.split_once(':') else {
            continue;
        };
        if let Ok(line_number) = line_number.parse::<usize>() {
            by_file
                .entry(file.to_string())
                .or_default()
                .push(GrepLineMatch { line_number });
        }
    }
    Ok(GrepResult::Success {
        matched_files: by_file
            .into_iter()
            .map(|(file_path, matched_lines)| GrepFileMatch {
                file_path,
                matched_lines,
            })
            .collect(),
    })
}

async fn execute_file_glob(
    patterns: Vec<String>,
    search_dir: String,
    current_working_directory: Option<String>,
    shell_type: ShellType,
    shell_path: Option<String>,
) -> Result<FileGlobV2Result> {
    let absolute_path = absolutize(&search_dir, current_working_directory.as_deref());
    let command = format!(
        "find {} -type f",
        shell_escape(&absolute_path.to_string_lossy())
    );
    let output =
        run_shell_capture(&command, current_working_directory, shell_type, shell_path).await?;
    let matched_files = output
        .lines()
        .filter(|path| {
            patterns
                .iter()
                .any(|pattern| simple_glob_match(pattern, path))
        })
        .map(|file_path| FileGlobV2Match {
            file_path: file_path.to_string(),
        })
        .collect();
    Ok(FileGlobV2Result::Success {
        matched_files,
        warnings: None,
    })
}

async fn run_shell_capture(
    command: &str,
    current_working_directory: Option<String>,
    shell_type: ShellType,
    shell_path: Option<String>,
) -> Result<String> {
    match execute_command(
        command,
        true,
        None,
        current_working_directory,
        shell_type,
        shell_path,
    )
    .await?
    {
        RequestCommandOutputResult::Completed { output, .. } => Ok(output),
        RequestCommandOutputResult::LongRunningCommandSnapshot { grid_contents, .. } => {
            Ok(grid_contents)
        }
        RequestCommandOutputResult::CancelledBeforeExecution
        | RequestCommandOutputResult::Denylisted { .. } => Ok(String::new()),
    }
}

async fn apply_requested_diffs(
    diffs: Vec<AIRequestedCodeDiff>,
    session_context: &SessionContext,
) -> Result<RequestFileEditsResult> {
    let mut unified = String::new();
    let mut updated_files = Vec::new();
    let mut deleted_files = Vec::new();
    let mut lines_added = 0usize;
    let mut lines_removed = 0usize;

    for diff in diffs {
        let absolute_path = host_native_absolute_path(
            &diff.file_name,
            session_context.shell(),
            session_context.current_working_directory(),
        );
        let path = PathBuf::from(&absolute_path);
        let (new_content, deleted, added, removed) = apply_diff_to_content(&diff)?;
        lines_added += added;
        lines_removed += removed;
        unified.push_str(&format!(
            "--- {}\n+++ {}\n@@\n{}\n",
            diff.file_name, diff.file_name, new_content
        ));
        if deleted {
            if path.exists() {
                async_fs::remove_file(&path).await?;
            }
            deleted_files.push(diff.file_name);
            continue;
        }
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        async_fs::write(&path, new_content.as_bytes()).await?;
        updated_files.push(UpdatedFileContext {
            was_edited_by_user: false,
            file_context: FileContext::new(
                diff.file_name,
                AnyFileContent::StringContent(new_content),
                None,
                None,
            ),
        });
    }

    Ok(RequestFileEditsResult::Success {
        diff: unified,
        updated_files,
        deleted_files,
        lines_added,
        lines_removed,
    })
}

fn apply_diff_to_content(diff: &AIRequestedCodeDiff) -> Result<(String, bool, usize, usize)> {
    match &diff.diff_type {
        DiffType::Create { delta } => Ok((
            delta.insertion.clone(),
            false,
            delta.insertion.lines().count(),
            0,
        )),
        DiffType::Delete { delta } => {
            Ok((String::new(), true, 0, delta.replacement_line_range.len()))
        }
        DiffType::Update { deltas, .. } => {
            let had_trailing_newline = diff.original_content.ends_with('\n');
            let mut lines = diff
                .original_content
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut added = 0usize;
            let mut removed = 0usize;
            for delta in deltas.iter().rev() {
                let start = delta
                    .replacement_line_range
                    .start
                    .saturating_sub(1)
                    .min(lines.len());
                let end = delta
                    .replacement_line_range
                    .end
                    .saturating_sub(1)
                    .min(lines.len());
                let replacement = delta
                    .insertion
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                added += replacement.len();
                removed += end.saturating_sub(start);
                lines.splice(start..end, replacement);
            }
            let mut content = lines.join("\n");
            if had_trailing_newline {
                content.push('\n');
            }
            Ok((content, false, added, removed))
        }
    }
}

fn unsupported_result(action: &AIAgentActionType) -> AIAgentActionResultType {
    match action {
        AIAgentActionType::UploadArtifact(_) => AIAgentActionResultType::UploadArtifact(
            UploadArtifactResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::SearchCodebase(_) => {
            AIAgentActionResultType::SearchCodebase(SearchCodebaseResult::Failed {
                reason: SearchCodebaseFailureReason::ClientError,
                message: "Tool is not implemented in warp-tui v0".to_string(),
            })
        }
        AIAgentActionType::ReadMCPResource { .. } => AIAgentActionResultType::ReadMCPResource(
            ReadMCPResourceResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::CallMCPTool { .. } => AIAgentActionResultType::CallMCPTool(
            CallMCPToolResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::SuggestNewConversation { message_id } => {
            AIAgentActionResultType::SuggestNewConversation(
                SuggestNewConversationResult::Accepted {
                    message_id: message_id.clone(),
                },
            )
        }
        AIAgentActionType::SuggestPrompt(request) => {
            AIAgentActionResultType::SuggestPrompt(SuggestPromptResult::Accepted {
                query: match request {
                    crate::ai::agent::SuggestPromptRequest::UnitTestsSuggestion {
                        query, ..
                    }
                    | crate::ai::agent::SuggestPromptRequest::PromptSuggestion {
                        prompt: query,
                        ..
                    } => query.clone(),
                },
            })
        }
        AIAgentActionType::OpenCodeReview => AIAgentActionResultType::OpenCodeReview,
        AIAgentActionType::InitProject => AIAgentActionResultType::InitProject,
        AIAgentActionType::ReadDocuments(_) => AIAgentActionResultType::ReadDocuments(
            ReadDocumentsResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::EditDocuments(_) => AIAgentActionResultType::EditDocuments(
            EditDocumentsResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::CreateDocuments(_) => AIAgentActionResultType::CreateDocuments(
            CreateDocumentsResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::UseComputer(_) => AIAgentActionResultType::UseComputer(
            UseComputerResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::InsertCodeReviewComments { repo_path, .. } => {
            AIAgentActionResultType::InsertReviewComments(InsertReviewCommentsResult::Error {
                repo_path: repo_path.to_string_lossy().to_string(),
                message: "Tool is not implemented in warp-tui v0".to_string(),
            })
        }
        AIAgentActionType::RequestComputerUse(_) => AIAgentActionResultType::RequestComputerUse(
            RequestComputerUseResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::ReadSkill(_) => AIAgentActionResultType::ReadSkill(
            ReadSkillResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::FetchConversation { .. } => AIAgentActionResultType::FetchConversation(
            FetchConversationResult::Error("Tool is not implemented in warp-tui v0".to_string()),
        ),
        AIAgentActionType::StartAgent { version, .. } => {
            AIAgentActionResultType::StartAgent(StartAgentResult::Error {
                error: "Tool is not implemented in warp-tui v0".to_string(),
                version: *version,
            })
        }
        AIAgentActionType::SendMessageToAgent { .. } => {
            AIAgentActionResultType::SendMessageToAgent(SendMessageToAgentResult::Error(
                "Tool is not implemented in warp-tui v0".to_string(),
            ))
        }
        AIAgentActionType::AskUserQuestion { .. } => AIAgentActionResultType::AskUserQuestion(
            crate::ai::agent::AskUserQuestionResult::Error(
                "Tool is not implemented in warp-tui v0".to_string(),
            ),
        ),
        AIAgentActionType::RunAgents(_) => {
            AIAgentActionResultType::RunAgents(RunAgentsResult::Failure {
                error: "Tool is not implemented in warp-tui v0".to_string(),
            })
        }
        AIAgentActionType::WaitForEvents { .. } => {
            AIAgentActionResultType::WaitForEvents(WaitForEventsResult::Completed)
        }
        _ => action.cancelled_result(),
    }
}

fn card_for_result(
    action_id: AIAgentActionId,
    action: &AIAgentActionType,
    result: &AIAgentActionResultType,
) -> TuiToolCard {
    let mut lines = Vec::new();
    match result {
        AIAgentActionResultType::RequestCommandOutput(RequestCommandOutputResult::Completed {
            command,
            output,
            exit_code,
            ..
        }) => {
            lines.push(command.clone());
            lines.push(format!(
                "exit {} · {} lines captured",
                exit_code.value(),
                output.lines().count()
            ));
        }
        AIAgentActionResultType::RequestFileEdits(RequestFileEditsResult::Success {
            updated_files,
            lines_added,
            lines_removed,
            ..
        }) => {
            lines.push(
                updated_files
                    .iter()
                    .map(|file| file.file_context.file_name.as_str())
                    .join(", "),
            );
            lines.push(format!(
                "+{lines_added} -{lines_removed} · applied automatically"
            ));
        }
        other => {
            lines.push(other.to_string());
        }
    }
    TuiToolCard {
        action_id,
        title: format!("Tool: {}", action.user_friendly_name()),
        lines,
    }
}

fn decorate_pager_command(command: &str, shell_type: ShellType) -> String {
    match shell_type {
        ShellType::Zsh | ShellType::Bash => format!("({command}) | command cat"),
        ShellType::Fish => format!("begin; {command}; end | command cat"),
        ShellType::PowerShell => format!("({command}) | \\Out-Host"),
    }
}

fn shell_type_from_env() -> ShellType {
    std::env::var("SHELL")
        .ok()
        .as_deref()
        .and_then(ShellType::from_name)
        .unwrap_or(ShellType::Zsh)
}

fn absolutize(path: &str, cwd: Option<&str>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        cwd.map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join(path)
    }
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn simple_glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let needle = pattern.trim_matches('*');
    if needle.is_empty() {
        true
    } else if pattern.starts_with('*') && pattern.ends_with('*') {
        path.contains(needle)
    } else if pattern.starts_with('*') {
        path.ends_with(needle)
    } else if pattern.ends_with('*') {
        path.starts_with(needle)
    } else {
        path == pattern || path.ends_with(pattern)
    }
}
