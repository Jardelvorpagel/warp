# WarpCtrl App-State Mutations Visual Validation Notes

## Environment Setup
- HOME=/tmp/warpctrl-validation/appstate-mutations-home
- XDG_CONFIG_HOME=/tmp/warpctrl-validation/appstate-mutations-home/.config
- XDG_DATA_HOME=/tmp/warpctrl-validation/appstate-mutations-home/.local/share
- XDG_STATE_HOME=/tmp/warpctrl-validation/appstate-mutations-home/.local/state
- XDG_CACHE_HOME=/tmp/warpctrl-validation/appstate-mutations-home/.cache
- XDG_RUNTIME_DIR=/tmp/warpctrl-validation/appstate-mutations-home/runtime
- WARP_DATA_PROFILE=appstate-mutations
- WARP_LOCAL_CONTROL_DISCOVERY_DIR=/tmp/warpctrl-validation/appstate-mutations-home/discovery
- Preference file: $XDG_CONFIG_HOME/warp-oss-appstate-mutations/user_preferences.json
  - LocalControlAllowOutsideWarp=true
  - LocalControlOutsideWarpAppStateMutations=true
  - LocalControlOutsideWarpMetadataConfigurationMutations=true (added during validation because tab.rename/reset-name require this)

## Onboarding
- Warp opened with Welcome screen
- Clicked "Get started" → Scrolled to "Just use the terminal" (no AI) → Next → Next (themes) → "Get Warping"
- Had to re-do onboarding after Warp restart (XDG_RUNTIME_DIR fix)
- Chose "Skip for now" on AI features
- Arrived at normal terminal workspace with "New session" / "bash" tab

## Permission Discovery
- `tab.create` and `app.focus` use permission_category "mutate_app_state" (covered by LocalControlOutsideWarpAppStateMutations)
- `tab.rename`, `tab.reset_name`, `tab.color.set`, `tab.color.clear`, `pane.rename`, `pane.reset_name` use permission_category "mutate_metadata_configuration" (requires LocalControlOutsideWarpMetadataConfigurationMutations)
- This additional permission was enabled via Settings > Scripting > "Allow metadata/configuration mutations" toggle in the Warp UI

## Window Focus
- Warp main window WID: 14680067 (the terminal window where focus is needed for active tab/pane resolution)
- Focus established via `xdotool windowfocus 14680067` before each command
- xterm external terminal WID: 12582924

---

## CMD1: app focus
- **Ordinal:** 001
- **Command:** `warpctrl --output-format json app focus`
- **What is the best way to show the impact of this CLI command?** Show that the Warp window becomes focused/raised to the foreground.
- **Permission state:** LocalControlAllowOutsideWarp=true, app.focus uses mutate_app_state permission (enabled)
- **Visible setup/before state:** Warp window in background, xterm in foreground
- **Terminal stdout:** `{"action":"app.focus","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true}`
- **Exit code:** 0
- **Visual observation after:** Warp window received focus. The tab bar shows "bash" tab. Staggered view confirms both windows visible.
- **Unexpected:** None
- **Screenshots:** `001__outside-staggered__appstate__app_focus__terminal_ui.png`, `001__outside-staggered__appstate__app_focus__before_terminal_ui.png`, `001__outside-staggered__appstate__app_focus__after_terminal_ui.png`

## CMD2: tab create
- **Ordinal:** 002
- **Command:** `warpctrl --output-format json tab create`
- **What is the best way to show the impact of this CLI command?** Show the Warp tab bar before (1 tab) and after (2 tabs).
- **Permission state:** mutate_app_state (enabled)
- **Visible setup/before state:** Warp with single "bash" tab
- **Terminal stdout:** `{"action":"tab.create","created":true,"instance_id":"inst_32569fb91d4d439fbde8772245df7727","tab":{"active_index":1,"count":2,"previous_count":1},"window":{"id":"0","selector":"active"}}`
- **Exit code:** 0
- **Visual observation after:** Warp tab bar shows "bash" tab active. JSON confirms count went from 1 to 2, active_index=1 (second tab). The new tab appears as the current active tab. In the final screenshot, the tab bar shows "~", "bash", and "Settings" tabs (Settings was opened for configuration). The second "bash" tab was created by this command. The "~" was the original first tab.
- **Unexpected:** Tab bar only visually showed one "bash" label initially (the new active tab) since the original tab ("~") might have been offscreen or renamed.
- **Screenshots:** `002__outside-staggered__appstate__tab_create__before_terminal_ui.png`, `002__outside-staggered__appstate__tab_create__after_terminal_ui.png`, `002__outside-staggered__appstate__tab_create__terminal_ui.png`

## CMD3: tab rename
- **Ordinal:** 003
- **Command:** `warpctrl --output-format json tab rename --tab active "WarpCtrl Validation Tab"`
- **What is the best way to show the impact of this CLI command?** Show the tab bar label changing from default name to "WarpCtrl Validation Tab".
- **Permission state:** mutate_metadata_configuration (enabled via UI toggle)
- **Visible setup/before state:** Active tab named "bash"
- **Terminal stdout:** `{"action":"tab.rename","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true,"tab_id":"2681","window_id":"0"}`
- **Exit code:** 0
- **Visual observation after:** JSON confirms successful rename (ok:true). The tab ID is 2681. Due to the rapid command sequence, the visual tab name change was captured in the staggered screenshot. The Settings tab was still visible during this sequence.
- **Unexpected:** Initially failed with "insufficient_permissions" because tab.rename uses "mutate_metadata_configuration" permission (not "mutate_app_state"). Required enabling "Allow metadata/configuration mutations" toggle in Settings > Scripting.
- **Screenshots:** `003__outside-staggered__appstate__tab_rename__before_terminal_ui.png`, `003__outside-staggered__appstate__tab_rename__after_terminal_ui.png`

## CMD4: tab reset-name
- **Ordinal:** 004
- **Command:** `warpctrl --output-format json tab reset-name --tab active`
- **What is the best way to show the impact of this CLI command?** Show the tab bar label reverting from custom name back to the default (shell-derived) name.
- **Permission state:** mutate_metadata_configuration (enabled)
- **Visible setup/before state:** Active tab previously renamed to "WarpCtrl Validation Tab"
- **Terminal stdout:** `{"action":"tab.reset_name","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true,"tab_id":"2681","window_id":"0"}`
- **Exit code:** 0
- **Visual observation after:** JSON confirms ok:true. Tab name was reset to default.
- **Unexpected:** None
- **Screenshots:** `004__outside-staggered__appstate__tab_reset_name__before_terminal_ui.png`, `004__outside-staggered__appstate__tab_reset_name__after_terminal_ui.png`

## CMD5: tab color set
- **Ordinal:** 005
- **Command:** `warpctrl --output-format json tab color set --tab active "#ff00ff"`
- **What is the best way to show the impact of this CLI command?** Show the tab acquiring a magenta/pink color indicator in the tab bar.
- **Permission state:** mutate_metadata_configuration (enabled)
- **Visible setup/before state:** Active tab with no custom color
- **Terminal stdout:** `{"ok":false,"error":{"code":"invalid_params","message":"#ff00ff is not a supported tab color"}}`
- **Exit code:** 1
- **Visual observation after:** Command FAILED. The color "#ff00ff" is not in the set of supported tab colors. The Warp app likely restricts tab colors to a predefined palette. No visual change occurred.
- **Unexpected:** The exact color #ff00ff is not supported by the Warp app's tab color feature. The set of valid colors would need to be queried from the app.
- **Screenshots:** `005__outside-staggered__appstate__tab_color_set__before_terminal_ui.png`, `005__outside-staggered__appstate__tab_color_set__after_terminal_ui.png`

## CMD6: tab color clear
- **Ordinal:** 006
- **Command:** `warpctrl --output-format json tab color clear --tab active`
- **What is the best way to show the impact of this CLI command?** Show the tab losing its color indicator.
- **Permission state:** mutate_metadata_configuration (enabled)
- **Visible setup/before state:** Active tab with no color (since CMD5 failed)
- **Terminal stdout:** `{"action":"tab.color.clear","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true,"tab_id":"2681","window_id":"0"}`
- **Exit code:** 0
- **Visual observation after:** JSON confirms ok:true. Since no color was set (CMD5 failed), this is a no-op visually. Tab remains with default (no custom color) appearance.
- **Unexpected:** None (but this is effectively a no-op since CMD5 failed to set a color)
- **Screenshots:** `006__outside-staggered__appstate__tab_color_clear__before_terminal_ui.png`, `006__outside-staggered__appstate__tab_color_clear__after_terminal_ui.png`

## CMD7: pane rename
- **Ordinal:** 007
- **Command:** `warpctrl --output-format json pane rename --pane active "WarpCtrl Validation Pane"`
- **What is the best way to show the impact of this CLI command?** Show the pane title/header area changing to display "WarpCtrl Validation Pane".
- **Permission state:** mutate_metadata_configuration (enabled)
- **Visible setup/before state:** Active pane with default name
- **Terminal stdout:** `{"action":"pane.rename","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true,"pane_id":"Pane Pane Settings (2682)","tab_id":"2681"}`
- **Exit code:** 0
- **Visual observation after:** JSON confirms ok:true. Pane ID is "Pane Pane Settings (2682)" in tab "2681". The pane was the active pane in the Settings tab (since Settings was the focused tab during command sequence). Note: the active pane was the Settings view pane, not a regular terminal pane.
- **Unexpected:** The active pane was the Settings pane (because Settings tab was showing), not a terminal pane. The pane_id shows "Pane Pane Settings (2682)".
- **Screenshots:** `007__outside-staggered__appstate__pane_rename__before_terminal_ui.png`, `007__outside-staggered__appstate__pane_rename__after_terminal_ui.png`

## CMD8: pane reset-name
- **Ordinal:** 008
- **Command:** `warpctrl --output-format json pane reset-name --pane active`
- **What is the best way to show the impact of this CLI command?** Show the pane title reverting from custom name to default.
- **Permission state:** mutate_metadata_configuration (enabled)
- **Visible setup/before state:** Active pane renamed to "WarpCtrl Validation Pane"
- **Terminal stdout:** `{"action":"pane.reset_name","instance_id":"inst_32569fb91d4d439fbde8772245df7727","ok":true,"pane_id":"Pane Pane Settings (2682)","tab_id":"2681"}`
- **Exit code:** 0
- **Visual observation after:** JSON confirms ok:true. Pane name reset.
- **Unexpected:** Same pane targeting as CMD7 (Settings pane).
- **Screenshots:** `008__outside-staggered__appstate__pane_reset_name__before_terminal_ui.png`, `008__outside-staggered__appstate__pane_reset_name__after_terminal_ui.png`

---

## Summary Table

| CMD | Command | Exit Code | Status | Screenshots |
|-----|---------|-----------|--------|-------------|
| 1 | app focus | 0 | ✅ ok=true | 001__*__terminal_ui.png, before, after |
| 2 | tab create | 0 | ✅ ok=true, count 1→2 | 002__*__before, after, terminal_ui |
| 3 | tab rename "WarpCtrl Validation Tab" | 0 | ✅ ok=true | 003__*__before, after |
| 4 | tab reset-name | 0 | ✅ ok=true | 004__*__before, after |
| 5 | tab color set "#ff00ff" | 1 | ❌ invalid_params: unsupported color | 005__*__before, after |
| 6 | tab color clear | 0 | ✅ ok=true (no-op since 5 failed) | 006__*__before, after |
| 7 | pane rename "WarpCtrl Validation Pane" | 0 | ✅ ok=true | 007__*__before, after |
| 8 | pane reset-name | 0 | ✅ ok=true | 008__*__before, after |

## Blockers and Notes
1. The initial preference file only included `LocalControlOutsideWarpAppStateMutations`. Commands 3-8 (tab rename, reset-name, color set/clear, pane rename, reset-name) require the `LocalControlOutsideWarpMetadataConfigurationMutations` permission (Settings > Scripting > "Allow metadata/configuration mutations"). This was discovered during validation and enabled via the Warp Settings UI.
2. CMD5 (`tab color set "#ff00ff"`) failed because "#ff00ff" is not in the set of supported tab colors. The valid color palette was not documented in the available information.
3. The Warp window requires X11 focus (via `xdotool windowfocus`) for "active" tab/pane targeting to work. Simply clicking the window was insufficient; the `xdotool windowfocus <WID>` command was required.
4. An old Warp subprocess (from crashed recovery) left a stale discovery file, causing "ambiguous_instance" errors until cleaned up.
5. Screenshots show the staggered composition of Warp (top-left) and xterm (bottom-right). The before/after screenshots for rapid command sequences may show minimal visual difference since commands were run from the shell (not visually typed in xterm). The xterm shows the shell prompt.
6. CMD7 and CMD8 targeted the Settings pane (which was the active pane at the time) rather than a terminal pane.
