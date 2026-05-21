use prost::Message;
use warp_multi_agent_api as api;

use crate::ai::agent::PassiveSuggestionTrigger;

pub(crate) const MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES: usize = 5 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PassiveRequestBudgetExceeded {
    pub encoded_len: usize,
    pub max_encoded_len: usize,
}

pub(crate) fn passive_request_encoded_len(request: &api::Request) -> usize {
    request.encoded_len()
}

pub(crate) fn passive_request_budget_exceeded(
    request: &api::Request,
) -> Option<PassiveRequestBudgetExceeded> {
    let encoded_len = passive_request_encoded_len(request);
    (encoded_len > MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES).then_some(
        PassiveRequestBudgetExceeded {
            encoded_len,
            max_encoded_len: MAX_PASSIVE_SUGGESTION_REQUEST_ENCODED_BYTES,
        },
    )
}

pub(crate) fn passive_suggestion_trigger_name(trigger: &PassiveSuggestionTrigger) -> &'static str {
    match trigger {
        PassiveSuggestionTrigger::FilesChanged => "files_changed",
        PassiveSuggestionTrigger::CommandRun => "command_run",
        PassiveSuggestionTrigger::ShellCommandCompleted(_) => "shell_command_completed",
        PassiveSuggestionTrigger::AgentResponseCompleted { .. } => "agent_response_completed",
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
