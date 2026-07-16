# TECH: TUI local child agents and rich orchestration messages
This change builds on the full-view `TuiSessions` container. Accepting a local
`run_agents` request in the TUI creates native Oz children in background terminal
sessions while the parent remains focused and receives orchestration traffic.
Received messages render as rich, collapsible participant rows in both parent
and child transcripts.
## Architecture
### Shared local Oz launch contract
The GUI and TUI share the frontend-neutral parts of native child launch through
`app/src/ai/blocklist/child_agent_launch.rs (16-93)`:
- `prepare_local_oz_child_launch` normalizes the child name and creates the server task row with
  the prompt and parent run id. The returned `PreparedLocalOzChildLaunch` contains the task id and
  normalized conversation name needed by the frontend-specific materializer.
- `inherit_child_agent_settings` copies the parent's execution profile and effective base model to
  the child surface.
- `apply_child_agent_model_override` installs a non-empty run-wide model override after inheritance.
The GUI hidden-pane path uses the same helpers in
`app/src/pane_group/pane/terminal_pane.rs (1528-1705)`. The TUI therefore does
not export or directly compose `AIClient`, `AgentConfigSnapshot`,
`ServerApiProvider`, or `AIExecutionProfilesModel`.
### Session event bridge
Each `TuiTerminalSessionView` owns its `StartAgentExecutor` subscription and
converts executor events into semantic session events
(`crates/warp_tui/src/terminal_session_view.rs (111-136, 599-614)`):
- `CreateAgent` emits `StartAgentConversation` with the request and a snapshot of the parent's
  current working directory.
- `CleanupFailedChildLaunch` emits the corresponding cleanup event.
`TuiOrchestrationModel::register` runs before the first session is created. It
subscribes to `TuiSessions` and, for every `SessionAdded`, subscribes to that
session view through `AppContext`; `SessionRemoved` tears down session-owned
streamer consumers (`crates/warp_tui/src/orchestration_model.rs (47-112)`).
Because every session, including a background child session, is registered in
`TuiSessions`, children are also wired to launch descendants.
### Native child launch
`TuiOrchestrationModel` separates task creation from TUI surface creation
(`crates/warp_tui/src/orchestration_model.rs (114-238)`):
1. `begin_local_oz_child_launch` starts shared server-task preparation.
2. `create_local_oz_child_session` creates an unfocused terminal session using the parent's
   captured working directory.
3. The child inherits the parent's execution profile and effective base model, then receives the
   requested run-wide model override.
4. `BlocklistAIHistoryModel::start_new_child_conversation` establishes lineage on the child
   surface. The task id is stamped before `record_new_conversation_request_complete` resolves the
   pending `StartAgentExecutor` slot.
5. The coordinator registers event consumers for the parent and child conversations.
6. `TuiTerminalSessionView::start_orchestrated_child` attaches the task id to the child controller
   and sends the first prompt (`crates/warp_tui/src/terminal_session_view.rs (1034-1049)`).
`create_local_terminal_session` is the single session factory for both the
focused bootstrap session and background children. Its explicit startup-directory
parameter preserves the parent's current directory for child shells
(`crates/warp_tui/src/session.rs (152-217)`).
### Model selection
TUI `agents.model` remains the default model for ordinary TUI surfaces.
Explicit per-surface overrides are resolved first so a child `model_id` always
wins, including when it equals the execution profile default
(`app/src/ai/llms.rs (844-878, 1504-1526)`).
### Streamer and session ownership
The coordinator stores only frontend-specific runtime ownership
(`crates/warp_tui/src/orchestration_model.rs (31-39)`):
- `child_session_by_conversation` maps a child conversation to its background session.
- `event_consumers_by_session` records which conversation streams each live session consumes.
`TuiOrchestrationModel` intentionally does not duplicate participant names,
statuses, agent-id indexes, or ancestry. It is an ephemeral materializer for
TUI sessions; restored conversations and participants created by other
frontends can exist without passing through it.

Conversation identity and lineage remain canonical in
`BlocklistAIHistoryModel`:
- `agent_id_to_conversation_id` resolves the server-side run id carried by an
  incoming message to the loaded `AIConversation`.
- `parent_conversation_id` / `parent_agent_id` identify the current
  conversation's immediate orchestrator.
- `children_by_parent` provides sibling order for deterministic TUI identity
  styling.
- `AIConversation` owns the participant's display name and
  `ConversationStatus`.

`app/src/ai/blocklist/orchestration_topology.rs` exposes the semantic resolution
shared by GUI and TUI:
- `orchestrator_agent_id_for_conversation` resolves only the immediate parent,
  with `parent_agent_id` as the fallback when the parent conversation is not
  loaded.
- `resolve_orchestration_participant` uses the history model's reverse index to
  return the participant role, local conversation id, and display name.

This is not a second orchestration graph or a full-tree traversal. It bridges
the two identifiers available at render time: the current local conversation
id and `ReceivedMessageDisplay::sender_agent_id`. Frontends project the shared
semantic result into their own presentation: the GUI chooses an avatar and
navigation behavior, while the TUI chooses a terminal glyph/color identity.
Removing a conversation also removes its id from `children_by_parent`
(`app/src/ai/blocklist/history_model.rs (2112-2182)`).
### Unsupported modes and failed launch cleanup
Local CLI-harness and remote requests resolve as explicit per-child failures
instead of waiting for the spawn timeout
(`crates/warp_tui/src/orchestration_model.rs (114-159, 239-296)`).
The failure path creates an errored child conversation on a synthetic surface
and echoes its id to `StartAgentExecutor`. The resulting cleanup event:
- deletes the child conversation and persisted state,
- removes it from the parent-child topology,
- removes any mapped background session, and
- unregisters consumers when the session is removed.
This leaves no dead child conversation, session, or streamer registration.
### Transcript rendering
`crates/warp_tui/src/agent_block.rs`:
- suppresses the `WaitForEvents` tool-call row, matching the GUI,
- preserves every `MessagesReceivedFromAgents` payload as an `AgentMessage`
  section, and
- omits `EventsFromAgents` rows because those outputs contain opaque ids rather
  than displayable participant or lifecycle data.

`crates/warp_tui/src/agent_message.rs` owns TUI presentation:
1. Resolve the current conversation's immediate orchestrator through the
   shared topology helper.
2. Resolve the sender run id through `BlocklistAIHistoryModel`.
3. Read the sender's display name and `ConversationStatus` from the resolved
   conversation.
4. Assign a deterministic TUI identity from the sender's sibling order; use a
   stable sender-id hash only when no loaded sibling relationship exists.
5. Render a collapsed-by-default row containing the conversation-status glyph,
   participant identity glyph, bold name, and disclosure chevron. Expansion
   shows the message body with a hanging indent, falling back to the subject
   when the body is blank.

Conversation rows use `ConversationStatus` directly. Tool calls retain the
separate `ToolCallDisplayState` because constructing and pending tool calls are
not conversation lifecycle states. Both use the same semantic
`TuiUiBuilder` color recipes without forcing their domain models into one enum.
## Exports
`app/src/tui_export.rs (52-75)` exposes the shared child-launch functions and
prepared result plus the `StartAgentExecutor` request/event/outcome types needed
by the TUI surface bridge. It also exports the frontend-neutral participant
resolution functions and result types from `orchestration_topology`. GUI
elements, TUI styles, server-client types, and execution-profile implementation
types remain behind their respective boundaries.
## Non-goals
- Local CLI-harness children (Claude, Codex, OpenCode, Gemini).
- Remote/cloud child materialization.
- Navigation to or revealing background child sessions.
- Removing completed child sessions; successful children remain retained like GUI hidden panes.
- Rendering opaque lifecycle event ids as transcript content.
## Testing and validation
- `crates/warp_tui/src/orchestration_model_tests.rs (154-221)` verifies that local-harness and
  remote requests resolve with explicit failures while leaving no child topology, extra session,
  or event-consumer state. It also verifies that failed-launch cleanup preserves unrelated
  retained sessions.
- `app/src/ai/blocklist/orchestration_topology_tests.rs` verifies shared
  participant discovery and that a grandchild resolves its direct parent,
  rather than the tree root, as orchestrator.
- `crates/warp_tui/src/agent_message_tests.rs` verifies parent/orchestrator
  labeling, direct `ConversationStatus` glyphs and styles, deterministic child
  identity presentation, collapse behavior, wrapping, and subject fallback.
- `crates/warp_tui/src/agent_block_tests.rs` verifies that received messages
  remain distinct rich sections, opaque lifecycle ids render no row,
  `WaitForEvents` contributes no tool row, hidden-only exchanges reserve no
  whitespace, and collapse state is owned by the agent block.
- `crates/warp_tui/src/tool_call_labels_tests.rs` keeps tool-call-only
  constructing, pending, blocked, running, and terminal presentation covered
  independently of conversation lifecycle state.
- `app/src/ai/llms_tests.rs (936-959)` verifies that an explicit surface override precedes the TUI
  file-backed default.
- `app/src/ai/blocklist/history_model_tests.rs (1872-1897)` verifies that removing a child
  conversation cleans the parent index.

Validation commands:
- `cargo nextest run -p warp_tui`
- `cargo nextest run -p warp -E 'test(orchestration_topology)'`
- `cargo clippy -p warp_tui --all-targets --all-features --tests -- -D warnings`
- `cargo clippy -p warp --lib --tests --features tui,test-util -- -D warnings`
- `./script/format`
## Follow-ups
- Add local CLI-harness children by reusing the existing local-harness preparation path.
- Add a TUI-native remote child materializer.
- Add child-session navigation and status UI on top of the retained `TuiSessions` entries.
