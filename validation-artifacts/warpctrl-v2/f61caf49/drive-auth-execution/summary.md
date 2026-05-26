# Warp Control CLI validation summary: drive-auth-execution
Validated SHA: `f61caf49400dc5c0d37d57a553d27733700e5204`
Expected SHA: `f61caf49400dc5c0d37d57a553d27733700e5204`
SHA match: `True`
Artifact root: `/workspace/warpctrl-validation/drive-auth-execution/validation-artifacts/warpctrl-v2/f61caf49/drive-auth-execution`
## Counts
- Pass: 4
- Fail: 0
- Skip: 2
- Executed commands: 4
- Total manifest entries: 6
## Result
The standalone `warpctrl` binary and graphical `warp-oss` app both built successfully with `warp_control_cli` enabled. A real Warp app was launched under Xvfb/Openbox with an isolated profile. Outside-Warp local control and all granular outside-Warp permissions were enabled in private local preferences, while inside-Warp authenticated-user actions remained disabled.
All executed Drive/API-key/execution-underlying attempts failed closed with JSON `execution_context_not_allowed`, matching the checked-out implementation metadata: Drive and `input.run` actions require an authenticated user and are only allowed from `inside_warp`. Visual screenshots show the external command/output and the logged-out/onboarding Warp app, with no Drive data exposed and no `warpctrl-validation` command executed in Warp.
## Visual-inspection failures/blockers
- Failures: none.
- Blockers:
  - Drive inspect could not run because no Drive object ID was returned by denied Drive list commands.
  - Drive workflow run could not run because no workflow ID was returned by denied Drive list commands.
  - Success-path authenticated Drive/execution validation was blocked by lack of verified Warp-terminal proof and the app remaining first-run/logged-out.
## Skipped commands
- `$WARPCTRL --output-format json drive inspect <DRIVE_OBJECT_ID_FROM_DRIVE_LIST>` — skipped because Drive list failed closed and returned no object ID.
- `$WARPCTRL --output-format json drive workflow run <WORKFLOW_ID_FROM_DRIVE_LIST> --arg validation=warpctrl` — skipped because workflow list failed closed and returned no workflow ID.
