# warpctrl operator README
`warpctrl` is the provisional CLI entrypoint for controlling an already-running local Warp app instance. It is intended for scripts, demos, agent workflows, and developer automation that need to perform allowlisted Warp UI actions through the installed channel-specific Warp binary without launching the GUI.
The first implementation slice is intentionally narrow:
- discover compatible running Warp instances;
- select one instance implicitly when unambiguous or explicitly with `--instance`;
- request brokered scoped local-control credentials for the selected instance;
- check app health and protocol compatibility with `warpctrl app ping` and `warpctrl app version`;
- create a new terminal tab with `warpctrl tab create`.
The local-control protocol and catalog are broader than this slice, but commands outside the implemented capability set should fail with structured unsupported-action errors until their handlers land.
## Packaging model
`warpctrl` should be packaged as an Oz-style wrapper script rather than a standalone Rust binary. The wrapper should resolve the installed channel-specific Warp executable and invoke it with the hidden `--warpctrl` control-mode flag:
- `crates/local_control` owns discovery records, local authentication material, client transport, protocol envelopes, action names, and error types.
- `crates/warp_cli` owns command parsing conventions for local-control subcommands.
- the channel-specific app binary owns the hidden `--warpctrl` dispatch path and exits before normal GUI startup.
- the app-side bridge owns the per-process loopback listener and dispatches supported actions onto the live Warp UI context.
The control-mode path should initialize only the work needed for CLI parsing, instance discovery, local authentication loading, request serialization, HTTP transport, and output formatting. It should not initialize GUI state, terminal models, rendering, workspaces, or main-app startup paths.
During the provisional naming period, release artifacts and helper names may be channelized, but operator docs and examples should use `warpctrl` unless an integration branch explicitly documents a channel-specific alias.
This branch wires the core hidden dispatch contract through the existing Warp binary. Platform packaging should create wrapper scripts that call the channel binary with `--warpctrl` instead of producing or selecting a separate `warpctrl` binary.
## Install and invocation guidance
### macOS
For local development checks, build the local Warp binary and invoke it with the hidden control-mode flag:
```bash
cargo run -p warp --bin warp -- --warpctrl instance list
```
For distributable checks, use the installed `warpctrl` wrapper. The wrapper execs the app bundle's channel-specific executable with `--warpctrl`.
### Linux
For local development checks, build the local Warp binary and invoke it with the hidden control-mode flag:
```bash
cargo run -p warp --bin warp -- --warpctrl instance list
```
For distributable checks, use the packaged `warpctrl` wrapper. The wrapper execs the packaged channel-specific Warp executable with `--warpctrl`.
Run `warpctrl --version` after installation to confirm the shell is resolving the expected build.
### Windows
Until the wrapper installer lands, build the local Warp binary and invoke it with the hidden control-mode flag for development checks:
```powershell
cargo run -p warp --bin warp -- --warpctrl instance list
```
Installer helper creation and release-artifact wiring still need a later packaging change before docs can promise an installer-provided `warpctrl` command.
## End-to-end local test flow
Use matching app and CLI bits from the same branch or release artifact so the protocol version and action catalog agree.
1. Start Warp and leave at least one window open.
2. Confirm that the local-control server registered the running process:
   ```bash
   warpctrl instance list
   ```
3. Confirm app health and protocol compatibility:
   ```bash
   warpctrl app ping
   warpctrl app version
   ```
4. If exactly one compatible instance is listed, create a new terminal tab:
   ```bash
   warpctrl tab create
   ```
5. If multiple compatible instances are listed, copy the desired `instance_id` and target it explicitly:
   ```bash
   warpctrl app ping --instance <instance_id>
   warpctrl app version --instance <instance_id>
   warpctrl tab create --instance <instance_id>
   ```
6. Verify the running app receives focus for the selected instance and a new terminal tab appears according to Warp's normal new-tab placement behavior.
7. In a future slice that implements `tab list`, inspect state before and after the mutation:
   ```bash
   warpctrl tab list --instance <instance_id>
   ```
Expected failures:
- no running compatible app: exits non-zero with a no-instance error;
- multiple ambiguous instances: exits non-zero and asks for `--instance`;
- unsupported app build or stale discovery record: exits non-zero with a protocol, stale-target, or transport error;
- `tab.create` not yet implemented by the running app bridge: exits non-zero with an unsupported-action error.
## Security model
The local-control protocol is designed for same-user scripting, not cross-user or network access. The trust boundary is the local user account.
- **Loopback-only listener.** Each Warp process binds its control server to `127.0.0.1` on an ephemeral port. The listener is not reachable from the network.
- **Brokered scoped credentials.** Discovery records contain instance metadata, endpoint information, and credential-broker references only when the selected Scripting mode allows that invocation context. They must not contain raw bearer tokens or reusable full-access credentials.
- **Short-lived grants.** `warpctrl` requests an action-scoped credential from `/v1/control/credentials` for the selected instance and invocation context, then presents that credential to `/v1/control`. Missing, invalid, expired, revoked, or insufficient-scope credentials are rejected before handler dispatch.
- **Protected credential material.** Raw local-control secrets live in platform secure storage where available, with owner-only local-state fallbacks documented as weaker. On POSIX platforms, discovery records and fallback local state must use owner-only permissions. On Windows, records must be stored under the current user's app data directory with an ACL that grants access only to the current user, Administrators, and SYSTEM.
- **Stale-record pruning.** On each `instance list` or implicit discovery call, records whose PID is no longer alive are deleted automatically, preventing stale endpoint or broker metadata from lingering on disk.
- **No CORS.** The control endpoints do not set permissive CORS headers, so browser-origin JavaScript cannot read responses even if it guesses the port. The credential requirement provides a second layer since browsers cannot read the brokered credential material.
```mermaid
sequenceDiagram
    participant CLI as warpctrl
    participant FS as ~/.warp/local-control/
    participant Broker as Credential broker
    participant HTTP as Warp loopback server<br/>(127.0.0.1:ephemeral)
    participant Bridge as App bridge

    CLI->>FS: Read discovery records (user-only permissions / ACL)
    FS-->>CLI: instance_id, endpoint, credential broker metadata
    CLI->>CLI: Prune stale PIDs, select instance
    CLI->>Broker: POST /v1/control/credentials<br/>action + context + instance
    Broker->>Broker: Check Settings > Scripting mode, context, scopes
    alt Disabled, invalid, or insufficient scope
        Broker-->>CLI: Structured denial
    else Grant allowed
        Broker-->>CLI: Short-lived scoped credential
        CLI->>HTTP: POST /v1/control<br/>Authorization: Bearer <scoped credential>
        HTTP->>HTTP: Verify grant and action scope
        HTTP->>Bridge: Dispatch action to app context
        Bridge-->>HTTP: Structured result or error
        HTTP-->>CLI: JSON response envelope
    end
```
**Known limitations and future hardening:**
- Windows local-control authentication is not complete until discovery-record ACL creation and validation are implemented.
- Same-user malicious software can still invoke trusted wrappers or automate the desktop, so brokered credentials are least-privilege guardrails rather than a complete hostile same-user sandbox.
- Once higher-risk handlers land, the same-user boundary becomes more sensitive. Consider per-request nonces, stricter platform secure-storage constraints, or Unix domain sockets with `SO_PEERCRED` for stronger caller identity where available.
## Documentation review notes
- Treat `warpctrl` as provisional executable naming until packaging signs off on final artifact aliases.
- Keep examples scoped to discovery, app health/version, and `tab create` until additional app-side handlers are implemented.
- Do not document catalog commands as usable just because they exist in protocol enums or parser scaffolding; operator docs should distinguish implemented commands from planned allowlist entries.
- Windows packaging may initially follow the existing helper-wrapper pattern. Update this README when that decision is final.
