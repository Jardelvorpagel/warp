# warpctrl operator README
`warpctrl` is the standalone CLI for controlling already-running local Warp app instances. It is intended for scripts, demos, agent workflows, and developer automation that need allowlisted Warp UI actions without launching the GUI executable in CLI mode.
# Implementation status
The protocol catalog is broader than the set of handlers implemented by any one branch. Use `warpctrl action list`, `warpctrl action inspect <action>`, `warpctrl capability list`, or `warpctrl capability inspect <action>` to inspect the compiled public catalog and distinguish implemented actions from catalog stubs.
The current foundation path supports external logged-out-safe local control only. Implemented commands include `warpctrl instance list`, `warpctrl app ping`, `warpctrl app version`, `warpctrl action list`, `warpctrl action inspect <action>`, `warpctrl capability list`, `warpctrl capability inspect <action>`, and `warpctrl tab create`.
Catalog entries marked as `stub` are public allowlisted names but return `unsupported_action` if invoked against this implementation slice. Authenticated actions require verified Warp-managed terminal invocation and are rejected until that proof path is implemented by the selected app.
# Packaging model
`warpctrl` is packaged as a separate CLI artifact from the Warp GUI app while reusing shared repository code:
- `crates/local_control` owns discovery records, authentication material, client transport, protocol envelopes, action names, and error types.
- `crates/warp_cli` owns command parsing conventions for local-control subcommands.
- the app-side bridge owns the per-process local listener and dispatches supported actions onto the live Warp UI context.
The binary initializes only CLI parsing, instance discovery, credential loading, request serialization, transport, and output formatting. It should not initialize GUI state, rendering, workspaces, or main-app startup paths.
# Local test flow
Use matching app and CLI bits from the same branch or artifact so protocol version and catalog agree.
1. Start Warp and leave at least one window open.
2. Confirm the app registered a local-control instance: `warpctrl instance list`.
3. Inspect implementation status: `warpctrl action list --implemented-only` or `warpctrl action inspect tab.create`.
4. If exactly one compatible instance is listed, run `warpctrl tab create`.
5. If multiple compatible instances are listed, pass `--instance <instance_id>`.
6. Verify the selected app creates a new terminal tab according to Warp's normal behavior.
Expected failures:
- no running compatible app: `no_instance`;
- multiple ambiguous instances: `ambiguous_instance`;
- disabled outside-Warp control: `local_control_disabled`;
- unsupported app build or stale discovery record: protocol, stale-target, or transport error;
- catalog entry without handler support: `unsupported_action`.
# Security model
The protocol is local same-user scripting, not cross-user or network control.
- Each Warp process exposes local control through loopback or an owner-only local socket.
- Control requests require scoped credentials.
- Discovery metadata is per user and does not grant broad authority by itself.
- Browser-origin JavaScript must not receive a permissive CORS path to control endpoints.
- External invocations are limited to logged-out-safe local-control actions.
- Authenticated actions require verified Warp-managed terminal invocation and the selected app's logged-in Warp user.
- `warpctrl` does not provide standalone secret-based authenticated external scripting.
# Documentation notes
- Use `warpctrl` as the executable name.
- Keep operator examples tied to implemented commands or mark catalog entries as stubs.
- Do not document excluded surfaces as usable commands.
- Excluded API-key auth actions and local file content mutation actions are not public `warpctrl` commands. `auth.api_key.*`, `file.read`, `file.write`, `file.append`, `file.delete`, `file.copy`, `file.move`, and `file.mkdir` must remain outside the allowlisted catalog unless a future spec explicitly adds them.
