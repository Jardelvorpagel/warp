use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::preprocess::PendingPreprocessedActions;
use super::{RunningActionPhase, RunningActions};
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::{AIAgentAction, AIAgentActionId, AIAgentActionResult};

/// Shared action queue/result state for Agent Mode tools.
pub(crate) struct AgentToolActionModel {
    pub(super) pending_preprocessed_actions: HashMap<AIConversationId, PendingPreprocessedActions>,
    pub(super) pending_actions: HashMap<AIConversationId, VecDeque<AIAgentAction>>,
    pub(super) running_actions: HashMap<AIConversationId, RunningActions>,
    pub(super) finished_action_results: HashMap<AIConversationId, Vec<Arc<AIAgentActionResult>>>,
    pub(super) action_order: HashMap<AIConversationId, HashMap<AIAgentActionId, usize>>,
    pub(super) past_action_results: HashMap<AIAgentActionId, Arc<AIAgentActionResult>>,
}

impl AgentToolActionModel {
    pub(crate) fn new() -> Self {
        Self {
            pending_preprocessed_actions: Default::default(),
            pending_actions: Default::default(),
            running_actions: Default::default(),
            finished_action_results: Default::default(),
            action_order: Default::default(),
            past_action_results: Default::default(),
        }
    }

    /// Records the dispatch order of a batch of actions so results can be sorted back
    /// into the original tool-call order when the batch drains.
    pub(crate) fn record_action_order(
        &mut self,
        conversation_id: AIConversationId,
        actions: &[AIAgentAction],
    ) {
        self.action_order.insert(
            conversation_id,
            actions
                .iter()
                .enumerate()
                .map(|(index, action)| (action.id.clone(), index))
                .collect(),
        );
    }

    /// Records an action as currently running in the given conversation.
    pub(crate) fn record_running_action(
        &mut self,
        conversation_id: AIConversationId,
        action_id: AIAgentActionId,
        phase: RunningActionPhase,
    ) {
        match self.running_actions.entry(conversation_id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().add_action(action_id);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(RunningActions::new(phase, action_id));
            }
        }
    }

    /// Removes the given action from the running set; clears the conversation entry when empty.
    pub(crate) fn finish_running_action(
        &mut self,
        conversation_id: AIConversationId,
        action_id: &AIAgentActionId,
    ) {
        let should_remove = self
            .running_actions
            .get_mut(&conversation_id)
            .is_some_and(|running| {
                running.remove_action(action_id);
                running.is_empty()
            });
        if should_remove {
            self.running_actions.remove(&conversation_id);
        }
    }

    pub(crate) fn push_finished_result(
        &mut self,
        conversation_id: AIConversationId,
        result: Arc<AIAgentActionResult>,
    ) {
        self.finished_action_results
            .entry(conversation_id)
            .or_default()
            .push(result);
    }

    /// Returns the pending action with the given ID, if any.
    #[cfg_attr(not(feature = "tui"), allow(dead_code))]
    pub(crate) fn find_pending_action(
        &self,
        conversation_id: AIConversationId,
        action_id: &AIAgentActionId,
    ) -> Option<&AIAgentAction> {
        self.pending_actions
            .get(&conversation_id)
            .and_then(|q| q.iter().find(|a| &a.id == action_id))
    }

    /// Returns the number of currently running actions (test helper).
    #[cfg(any(test, feature = "integration_tests"))]
    pub(crate) fn running_action_count(&self, conversation_id: AIConversationId) -> usize {
        self.running_actions
            .get(&conversation_id)
            .map(|r| r.action_ids.len())
            .unwrap_or(0)
    }

    /// Returns the number of pending (not-yet-started) actions (test helper).
    #[cfg(any(test, feature = "integration_tests"))]
    pub(crate) fn pending_action_count(&self, conversation_id: AIConversationId) -> usize {
        self.pending_actions
            .get(&conversation_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub(crate) fn drain_finished_results(
        &mut self,
        conversation_id: AIConversationId,
    ) -> Vec<AIAgentActionResult> {
        let action_order = self.action_order.remove(&conversation_id);
        let mut finished_results = self
            .finished_action_results
            .remove(&conversation_id)
            .unwrap_or_default();
        if let Some(action_order) = action_order {
            finished_results
                .sort_by_key(|result| action_order.get(&result.id).copied().unwrap_or(usize::MAX));
        }
        for result in &finished_results {
            self.past_action_results
                .insert(result.id.clone(), result.clone());
        }
        finished_results
            .into_iter()
            .map(|result| (*result).clone())
            .collect()
    }
}
