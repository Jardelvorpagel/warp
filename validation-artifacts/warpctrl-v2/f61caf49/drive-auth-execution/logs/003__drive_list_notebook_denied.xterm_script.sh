#!/usr/bin/env bash
unset WARP_API_KEY STAGING_USER_WARP_API_KEY PUFFINS_WARP_API_KEY DAVE_WARP_API_KEY SENTRY_MEMORY_ALERT_WARP_API_KEY KEYBOARD_AUDIT_WARP_API_KEY WARP_WORKLOAD_TOKEN WARP_QNA_BOT_TOKEN
export WARPCTRL="/workspace/warpctrl-validation/drive-auth-execution/target/debug/warpctrl"
export XDG_CONFIG_HOME="/tmp/warpctrl-validation/drive-auth-execution-run/xdg-config"
export XDG_DATA_HOME="/tmp/warpctrl-validation/drive-auth-execution-run/xdg-data"
export XDG_CACHE_HOME="/tmp/warpctrl-validation/drive-auth-execution-run/xdg-cache"
export WARP_DATA_PROFILE="warpctrl-drive-auth-execution-f61caf49"
export WARP_LOCAL_CONTROL_DISCOVERY_DIR="/tmp/warpctrl-validation/drive-auth-execution-run/discovery"
printf 'What is the best way to show the impact of this CLI command?\n'
printf '%s\n\n' "Use the same staggered terminal/UI composition and keep the logged-out app visible."
printf 'Answer: %s\n\n' "The terminal should show a structured authenticated-user/execution-context denial and the app should remain unchanged with no notebooks exposed."
printf 'Visible setup: Warp app window is visible at right. It is on first-run/logged-out onboarding, so authenticated-user Drive/execution actions should not expose data or run commands from this outside-Warp terminal.\n\n'
printf '$ %s\n' "$WARPCTRL --output-format json drive list --type notebook"
"$WARPCTRL" --output-format json drive list --type notebook > "/workspace/warpctrl-validation/drive-auth-execution/validation-artifacts/warpctrl-v2/f61caf49/drive-auth-execution/logs/003__drive_list_notebook_denied.stdout_stderr.log" 2>&1
status=$?
cat "/workspace/warpctrl-validation/drive-auth-execution/validation-artifacts/warpctrl-v2/f61caf49/drive-auth-execution/logs/003__drive_list_notebook_denied.stdout_stderr.log"
printf '\nexit_code=%s\n' "$status"
printf '%s\n' "$status" > "/workspace/warpctrl-validation/drive-auth-execution/validation-artifacts/warpctrl-v2/f61caf49/drive-auth-execution/logs/003__drive_list_notebook_denied.exit_code"
touch "/workspace/warpctrl-validation/drive-auth-execution/validation-artifacts/warpctrl-v2/f61caf49/drive-auth-execution/logs/003__drive_list_notebook_denied.done"
sleep 300
