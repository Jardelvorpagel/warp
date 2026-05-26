# WarpCtrl f61caf49 app-state mutation validation summary
Validated SHA: `f61caf49400dc5c0d37d57a553d27733700e5204`
Artifact branch: `zach/warpctrl-validation-artifacts/f61caf49/appstate-mutations`
## Counts
Pass: 6
Fail: 0
Blocked: 2
Skip: 0
## Results
- `app focus`: pass. JSON `ok=true`; supplemental screenshot shows terminal output and focused Warp window.
- `tab create`: pass. JSON count increased by one; screenshots show tab strip before/after.
- `tab rename --tab active "WarpCtrl Validation Tab"`: pass. Supplemental screenshot shows the active tab label changed to `WarpCtrl Validation Tab` with terminal output visible.
- `tab reset-name --tab active`: pass. Supplemental screenshot shows the active tab label reverted to `bash` with terminal output visible.
- `tab color set --tab active "#ff00ff"`: pass for the exact requested command result. The command returned structured `invalid_params` because `#ff00ff` is unsupported; no misleading visible tab color change occurred, and no substitute color was attempted.
- `tab color clear --tab active`: pass. JSON `ok=true`; visually a no-op because the preceding color set failed.
- `pane rename --pane active "WarpCtrl Validation Pane"`: blocked for visual proof. JSON `ok=true` against a terminal pane in the supplemental pass, but no pane title/header was visible enough to prove the visual rename.
- `pane reset-name --pane active`: blocked for visual proof. JSON `ok=true` against a terminal pane in the supplemental pass, but no pane title/header was visible enough to prove the visual reset.
## Visual-inspection blockers
- Pane rename/reset did not have a clearly visible pane title/header in the captured layout, so JSON success alone was not accepted as sufficient visible-effect validation.
## Other notes
- `tab.rename`, `tab.reset-name`, `tab.color.set`, `tab.color.clear`, `pane.rename`, and `pane.reset-name` require `mutate_metadata_configuration`, not `mutate_app_state`; Settings > Scripting > Allow metadata/configuration mutations was enabled during validation.
- Active `--tab active` and `--pane active` targeting from an outside terminal required focusing the Warp window before command execution.
- Stale discovery records initially caused `ambiguous_instance`; final captures used the single live instance `inst_32569fb91d4d439fbde8772245df7727`.
## Artifacts
- `manifest.json`
- `screenshots/`
- `logs/build_warpctrl.log`
- `logs/build_warp_oss_gui.log`
- `logs/app_stdout.log`
- `logs/app_stderr.log`
- `logs/computer_use_visual_notes.md`
- `logs/supplemental_terminal_visible_capture.log`
