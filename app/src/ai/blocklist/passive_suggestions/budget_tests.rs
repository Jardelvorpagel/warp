use warp_multi_agent_api as api;

use super::{
    passive_request_budget_exceeded, passive_request_encoded_len, passive_suggestion_trigger_name,
    MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES,
};
use crate::ai::agent::PassiveSuggestionTrigger;

fn passive_request_with_task_message(message_text: String) -> api::Request {
    api::Request {
        task_context: Some(api::request::TaskContext {
            tasks: vec![api::Task {
                id: "root-task".to_string(),
                messages: vec![api::Message {
                    id: "message-1".to_string(),
                    task_id: "root-task".to_string(),
                    server_message_data: String::new(),
                    citations: vec![],
                    message: Some(api::message::Message::AgentOutput(
                        api::message::AgentOutput { text: message_text },
                    )),
                    request_id: "request-1".to_string(),
                    timestamp: None,
                }],
                dependencies: None,
                description: String::new(),
                summary: String::new(),
                server_data: String::new(),
            }],
        }),
        input: Some(api::request::Input {
            context: None,
            r#type: Some(api::request::input::Type::GeneratePassiveSuggestions(
                api::request::input::GeneratePassiveSuggestions {
                    attachments: vec![],
                    trigger: Some(
                        api::request::input::generate_passive_suggestions::Trigger::AgentResponseCompleted(
                            api::request::input::generate_passive_suggestions::AgentResponseCompleted {},
                        ),
                    ),
                },
            )),
        }),
        settings: None,
        metadata: None,
        existing_suggestions: None,
        mcp_context: None,
    }
}

fn passive_request_with_shell_output(output: String) -> api::Request {
    api::Request {
        task_context: None,
        input: Some(api::request::Input {
            context: None,
            r#type: Some(api::request::input::Type::GeneratePassiveSuggestions(
                api::request::input::GeneratePassiveSuggestions {
                    attachments: vec![],
                    trigger: Some(
                        api::request::input::generate_passive_suggestions::Trigger::ShellCommandCompleted(
                            api::request::input::generate_passive_suggestions::ShellCommandCompleted {
                                executed_shell_command: Some(api::ExecutedShellCommand {
                                    command: "cargo test".to_string(),
                                    output,
                                    exit_code: 0,
                                    command_id: "block-1".to_string(),
                                    is_auto_attached: false,
                                    started_ts: None,
                                    finished_ts: None,
                                }),
                                relevant_files: vec![],
                            },
                        ),
                    ),
                },
            )),
        }),
        settings: None,
        metadata: None,
        existing_suggestions: None,
        mcp_context: None,
    }
}

#[test]
fn small_passive_request_is_within_budget() {
    let request = passive_request_with_task_message("normal passive context".to_string());

    assert!(passive_request_budget_exceeded(&request).is_none());
}

#[test]
fn large_passive_conversation_context_exceeds_budget() {
    let request = passive_request_with_task_message(
        "x".repeat(MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES + 1),
    );

    let rejection = passive_request_budget_exceeded(&request)
        .expect("large passive context should exceed the encoded request budget");

    assert_eq!(rejection.encoded_len, passive_request_encoded_len(&request));
    assert_eq!(
        rejection.max_encoded_len,
        MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES
    );
}

#[test]
fn large_shell_trigger_context_exceeds_budget() {
    let request = passive_request_with_shell_output(
        "x".repeat(MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES + 1),
    );

    assert!(passive_request_budget_exceeded(&request).is_some());
}

#[test]
fn passive_trigger_names_are_stable_for_logs() {
    assert_eq!(
        passive_suggestion_trigger_name(&PassiveSuggestionTrigger::FilesChanged),
        "files_changed"
    );
    assert_eq!(
        passive_suggestion_trigger_name(&PassiveSuggestionTrigger::CommandRun),
        "command_run"
    );
}
