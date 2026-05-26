# warpctrl v2 validation summary: drive-auth-execution
Exact SHA validated: `53d0313df1f712cf98b1c53e9272c588141da350`
Expected SHA: `53d0313df1f712cf98b1c53e9272c588141da350`
HEAD match: `True`
Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/drive-auth-execution`
## Build and launch
Build command: `cargo build --manifest-path /workspace/warp/Cargo.toml -p warp --bin warp-oss --bin warpctrl --features warp_control_cli,gui`
App binary: `/workspace/warp/target/debug/warp-oss`
`$WARPCTRL`: `/workspace/warp/target/debug/warpctrl`
Launch: `Xvfb :117` + `openbox`, isolated XDG directories and `WARP_DATA_PROFILE=warpctrl-validation-drive-auth-execution`.
Runtime flag: `FeatureFlag::WarpControlCli` was enabled through the OSS debug build compiled with `warp_control_cli`; local-control discovery was published by the running app.
## Results
Pass: 6
Fail: 0
Skip: 4
All six executed assigned commands returned JSON `execution_context_not_allowed` denials from outside-Warp context, matching the expected behavior for authenticated-user Drive and underlying-data execution actions without verified Warp-terminal proof.
## Executed commands
- `$WARPCTRL --output-format json drive list --type workflow` — pass, denied with `execution_context_not_allowed`.
- `$WARPCTRL --output-format json drive list --type notebook` — pass, denied with `execution_context_not_allowed`.
- `$WARPCTRL --output-format json drive list --type env-var-collection` — pass, denied with `execution_context_not_allowed`.
- `$WARPCTRL --output-format json drive inspect drive_object_id_unavailable_due_expected_denial` — pass, denied with `execution_context_not_allowed`.
- `$WARPCTRL --output-format json drive workflow run workflow_id_unavailable_due_expected_denial --arg validation=warpctrl` — pass, denied with `execution_context_not_allowed`.
- `$WARPCTRL --output-format json input run "printf warpctrl-validation"` — pass, denied with `execution_context_not_allowed`.
## Justified skips
- Authenticated Drive list success paths: skipped because disposable test credentials and app-issued verified terminal proof material were unavailable.
- Drive inspect success path with `<DRIVE_OBJECT_ID_FROM_DRIVE_LIST>`: skipped because Drive list success could not run, so no object ID was available.
- Workflow run success path with `<WORKFLOW_ID_FROM_DRIVE_LIST>`: skipped because disposable credentials/resources and proof material were unavailable; mutation path would be unsafe without disposable resources.
- `input run` success path: skipped because command execution requires authenticated verified Warp-terminal grant and is a high-risk success path without disposable environment/proof.
## Screenshot paths
- `screenshots/001__outside-warp__drive-auth__drive-list-workflow__terminal.png`
- `screenshots/002__outside-warp__drive-auth__drive-list-notebook__terminal.png`
- `screenshots/003__outside-warp__drive-auth__drive-list-env-var-collection__terminal.png`
- `screenshots/004__outside-warp__drive-auth__drive-inspect-dummy__terminal.png`
- `screenshots/005__outside-warp__drive-auth__drive-workflow-run-dummy__terminal.png`
- `screenshots/006__outside-warp__execution-underlying__input-run__terminal.png`
## Log paths
- `logs/001__outside-warp__drive-auth__drive-list-workflow.log`
- `logs/002__outside-warp__drive-auth__drive-list-notebook.log`
- `logs/003__outside-warp__drive-auth__drive-list-env-var-collection.log`
- `logs/004__outside-warp__drive-auth__drive-inspect-dummy.log`
- `logs/005__outside-warp__drive-auth__drive-workflow-run-dummy.log`
- `logs/006__outside-warp__execution-underlying__input-run.log`
- `logs/openbox.log`
- `logs/warp-app-stdout.log`
- `logs/xvfb.log`
## Blockers
- No disposable authenticated Warp test credentials/resources were available.
- No app-issued WARPCTRL_TERMINAL_PROOF_ID/WARPCTRL_TERMINAL_SESSION_ID/WARPCTRL_TERMINAL_PROOF_SECRET path was exposed to the standalone validation shell, so inside-Warp authenticated success paths could not be exercised.
- Drive list success was unavailable, so DRIVE_OBJECT_ID_FROM_DRIVE_LIST and WORKFLOW_ID_FROM_DRIVE_LIST could not be obtained; dummy IDs were used only to validate pre-dispatch denial behavior.
