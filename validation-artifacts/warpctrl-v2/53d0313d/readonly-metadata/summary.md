# Warp Control CLI readonly metadata validation
- Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/readonly-metadata`
- Exact SHA requested: `53d0313df1f712cf98b1c53e9272c588141da350`
- Validated HEAD: `53d0313df1f712cf98b1c53e9272c588141da350`
- SHA verified before build/test: `True`
- Pass/fail/skip: `13` pass, `0` fail, `0` skip

## Build and launch
- Installed pinned `warp-channel-config` with `/workspace/warp/script/install_channel_config --force`.
- Built standalone `warpctrl` and dev app once with `cargo build --manifest-path /workspace/warp/Cargo.toml -p warp --bin dev --bin warpctrl --features gui,warp_control_cli,standalone`.
- Rebuilt the GUI app without `standalone`, then with `fast_dev`, using `cargo build --manifest-path /workspace/warp/Cargo.toml -p warp --bin dev --features gui,warp_control_cli,fast_dev`.
- Launched `/workspace/warp/target/debug/dev` under Xvfb/Openbox with isolated `WARP_DATA_PROFILE=warpctrl-validation-53d0313d-readonly-metadata`, `XDG_RUNTIME_DIR=/tmp`, and `WARP_API_KEY` removed from the app/CLI environment.
- Set `$WARPCTRL` to `/workspace/warp/target/debug/warpctrl` in every per-case terminal script.

## Results
- `pass` `$WARPCTRL --output-format json window list` → exit `0`; terminal `screenshots/01__outside__metadata__window_list__terminal.png`; UI `screenshots/01__outside__metadata__window_list__ui.png`; log `logs/01__outside__metadata__window_list.log`
- `pass` `$WARPCTRL window inspect --window active` → exit `0`; terminal `screenshots/02__outside__metadata__window_inspect_active__terminal.png`; UI `screenshots/02__outside__metadata__window_inspect_active__ui.png`; log `logs/02__outside__metadata__window_inspect_active.log`
- `pass` `$WARPCTRL tab list` → exit `0`; terminal `screenshots/03__outside__metadata__tab_list__terminal.png`; UI `screenshots/03__outside__metadata__tab_list__ui.png`; log `logs/03__outside__metadata__tab_list.log`
- `pass` `$WARPCTRL tab inspect --tab active` → exit `0`; terminal `screenshots/04__outside__metadata__tab_inspect_active__terminal.png`; UI `screenshots/04__outside__metadata__tab_inspect_active__ui.png`; log `logs/04__outside__metadata__tab_inspect_active.log`
- `pass` `$WARPCTRL tab inspect --tab-index 0` → exit `0`; terminal `screenshots/05__outside__metadata__tab_inspect_index_0__terminal.png`; UI `screenshots/05__outside__metadata__tab_inspect_index_0__ui.png`; log `logs/05__outside__metadata__tab_inspect_index_0.log`
- `pass` `$WARPCTRL pane list` → exit `0`; terminal `screenshots/06__outside__metadata__pane_list__terminal.png`; UI `screenshots/06__outside__metadata__pane_list__ui.png`; log `logs/06__outside__metadata__pane_list.log`
- `pass` `$WARPCTRL pane inspect --pane active` → exit `0`; terminal `screenshots/07__outside__metadata__pane_inspect_active__terminal.png`; UI `screenshots/07__outside__metadata__pane_inspect_active__ui.png`; log `logs/07__outside__metadata__pane_inspect_active.log`
- `pass` `$WARPCTRL pane inspect --pane-index 0` → exit `0`; terminal `screenshots/08__outside__metadata__pane_inspect_index_0__terminal.png`; UI `screenshots/08__outside__metadata__pane_inspect_index_0__ui.png`; log `logs/08__outside__metadata__pane_inspect_index_0.log`
- `pass` `$WARPCTRL session list` → exit `0`; terminal `screenshots/09__outside__metadata__session_list__terminal.png`; UI `screenshots/09__outside__metadata__session_list__ui.png`; log `logs/09__outside__metadata__session_list.log`
- `pass` `$WARPCTRL session inspect --session active` → exit `0`; terminal `screenshots/10__outside__metadata__session_inspect_active__terminal.png`; UI `screenshots/10__outside__metadata__session_inspect_active__ui.png`; log `logs/10__outside__metadata__session_inspect_active.log`
- `pass` `$WARPCTRL window inspect --window missing-window-id` → exit `1`; terminal `screenshots/11__outside__selector_edge__window_inspect_missing_window_id__terminal.png`; UI `screenshots/11__outside__selector_edge__window_inspect_missing_window_id__ui.png`; log `logs/11__outside__selector_edge__window_inspect_missing_window_id.log`
- `pass` `$WARPCTRL tab inspect --tab-index 999` → exit `1`; terminal `screenshots/12__outside__selector_edge__tab_inspect_index_999__terminal.png`; UI `screenshots/12__outside__selector_edge__tab_inspect_index_999__ui.png`; log `logs/12__outside__selector_edge__tab_inspect_index_999.log`
- `pass` `$WARPCTRL window inspect --window active --window-index 0` → exit `2`; terminal `screenshots/13__outside__selector_edge__window_inspect_active_conflict_window_index_0__terminal.png`; UI `n/a`; log `logs/13__outside__selector_edge__window_inspect_active_conflict_window_index_0.log`

## Blockers and notes
- No commands were skipped or left unexecuted.
- The final manifest uses the rerun where every required command has terminal PNG evidence.
- Active/index selector tests required refocusing the Warp window immediately before invoking `warpctrl`; without this, scoped index resolution in headless X could report `missing_target` because the xterm was active.
- No implementation branch was modified or pushed; durable output is limited to this artifact directory and branch.

## Screenshot paths
- `screenshots/01__outside__metadata__window_list__terminal.png`
- `screenshots/01__outside__metadata__window_list__ui.png`
- `screenshots/02__outside__metadata__window_inspect_active__terminal.png`
- `screenshots/02__outside__metadata__window_inspect_active__ui.png`
- `screenshots/03__outside__metadata__tab_list__terminal.png`
- `screenshots/03__outside__metadata__tab_list__ui.png`
- `screenshots/04__outside__metadata__tab_inspect_active__terminal.png`
- `screenshots/04__outside__metadata__tab_inspect_active__ui.png`
- `screenshots/05__outside__metadata__tab_inspect_index_0__terminal.png`
- `screenshots/05__outside__metadata__tab_inspect_index_0__ui.png`
- `screenshots/06__outside__metadata__pane_list__terminal.png`
- `screenshots/06__outside__metadata__pane_list__ui.png`
- `screenshots/07__outside__metadata__pane_inspect_active__terminal.png`
- `screenshots/07__outside__metadata__pane_inspect_active__ui.png`
- `screenshots/08__outside__metadata__pane_inspect_index_0__terminal.png`
- `screenshots/08__outside__metadata__pane_inspect_index_0__ui.png`
- `screenshots/09__outside__metadata__session_list__terminal.png`
- `screenshots/09__outside__metadata__session_list__ui.png`
- `screenshots/10__outside__metadata__session_inspect_active__terminal.png`
- `screenshots/10__outside__metadata__session_inspect_active__ui.png`
- `screenshots/11__outside__selector_edge__window_inspect_missing_window_id__terminal.png`
- `screenshots/11__outside__selector_edge__window_inspect_missing_window_id__ui.png`
- `screenshots/12__outside__selector_edge__tab_inspect_index_999__terminal.png`
- `screenshots/12__outside__selector_edge__tab_inspect_index_999__ui.png`
- `screenshots/13__outside__selector_edge__window_inspect_active_conflict_window_index_0__terminal.png`

## Log paths
- `logs/01__outside__metadata__window_list.log`
- `logs/02__outside__metadata__window_inspect_active.log`
- `logs/03__outside__metadata__tab_list.log`
- `logs/04__outside__metadata__tab_inspect_active.log`
- `logs/05__outside__metadata__tab_inspect_index_0.log`
- `logs/06__outside__metadata__pane_list.log`
- `logs/07__outside__metadata__pane_inspect_active.log`
- `logs/08__outside__metadata__pane_inspect_index_0.log`
- `logs/09__outside__metadata__session_list.log`
- `logs/10__outside__metadata__session_inspect_active.log`
- `logs/11__outside__selector_edge__window_inspect_missing_window_id.log`
- `logs/12__outside__selector_edge__tab_inspect_index_999.log`
- `logs/13__outside__selector_edge__window_inspect_active_conflict_window_index_0.log`
- `logs/build_and_launch.log`
- `logs/case_results.json`
- `logs/openbox.log`
- `logs/secret_scan.log`
- `logs/sha_verification.log`
- `logs/warp-app.log`
- `logs/xvfb.log`
