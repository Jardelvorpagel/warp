*Spec: macOS playback-speed parity (REMOTE-2201)*

*Summary:* Wire the Linux `RecordingConfig::playback_speed_multiplier` behavior into the macOS avfoundation recorder in PR #13742. Window-scoped macOS recording is explicitly deferred to a follow-up.

*Key design choices:* Rebase the recorder branch onto current `master`; mirror Linux's output `setpts` filter and move `-t` before `-i`; preserve all existing macOS capture and encode settings. Do not add crop or ScreenCaptureKit code in this PR.

*Design alternatives:* The only meaningful choice for this XS change is whether to share or duplicate the small ffmpeg argument block. Keep the macOS builder local and mirror the Linux code: a broader cross-platform refactor would increase rebase and Linux blast radius without improving this fix. Window capture has two viable designs (avfoundation crop versus ScreenCaptureKit), but both are out of scope and are deferred to a separately specified follow-on.

*Root cause / approach:* PR #13742 is based on `f459e858` and predates Linux commits `eb1c691a` (window target) and `ea24a1a5` (playback speed), so its `RecordingConfig` lacks both fields. Rebase `factory/macos-video-recording` onto current `master` first. In the rebased `mac/recording.rs`, apply `-vf setpts={1/N:.6}*PTS` when `config.playback_speed_multiplier > 1.0`, matching Linux's reciprocal and six-decimal formatting. Place `-t <max_duration>` before `-i` so the wall-clock input limit is not stretched by `setpts`; leave `-fs` as an output limit. Values `<= 1.0` omit `-vf`.

*Affected files:* `warp/crates/computer_use/src/mac/recording.rs` (ffmpeg argument order and playback filter); `warp/crates/computer_use/src/recording_tests.rs` or a macOS recorder test module (deterministic argument assertions); shared `lib.rs`/rebase conflict resolution only as required to consume the already-landed `RecordingConfig` fields. No Linux implementation changes.

*Open questions resolved:* The fix lands as additional commits on PR #13742 after rebase if that PR remains open; if it is finalized, use the same contract in a follow-up based on its merged macOS recorder. Window-scoped recording is deferred: implementation must add a `TODO(vkodithala)` in the relevant macOS recorder code and document in the implementation PR description that a future follow-up must choose between static avfoundation crop and ScreenCaptureKit true per-window capture. No crop behavior, multi-display contract, move/resize tracking, or occlusion semantics are part of this spec.

*Risks / blast radius:* Incorrect filter ordering can stretch the duration cap or produce a wrong playback rate; deterministic argv tests must assert both. Rebase conflicts could disturb the existing macOS lifecycle; Linux code and encode settings must remain unchanged. The deferred window TODO must not alter current whole-screen behavior.

*Validation & verification criteria* (must ALL pass before merge):
1. Rebase `factory/macos-video-recording` onto current `master` (which contains `RecordingConfig::playback_speed_multiplier` and `target`) and resolve the open bot review concern at `mac/recording.rs:93`; the macOS `computer_use` crate compiles on a macOS runner.
2. Add a deterministic macOS argument-builder/unit test that fails against the current PR and passes after the fix: with multiplier `4.0`, the argv contains exactly one `-vf` whose value is `setpts=0.250000*PTS`; with `1.0` and `0.0`, no `-vf` is emitted.
3. The same test asserts `-t` appears before `-i`, while `-fs` remains after input/output encoding arguments; existing avfoundation input, cursor/click capture, dimensions, codec, preset, pixel format, `+faststart`, and output path arguments remain present.
4. Existing macOS ffmpeg-absent, mock/drop-cleanup, and finalization tests pass; `Target::Screen` continues to build the current whole-main-display command. Linux recording tests and behavior remain unchanged.
5. On a macOS local direct-mode run with ffmpeg on `PATH` and Screen Recording permission granted, run a real recording using the default `RecordingConfig` and verify the MP4 is non-empty/playable, uses 4x playback, and stops within the configured wall-clock `max_duration` rather than a speed-stretched limit.
6. On a macOS Namespace VM/sidecar, repeat the default recording flow and attach the run link plus a playable sample/probe result to the task/PR. Linux cannot exercise the avfoundation path and is not evidence for this criterion.
7. Add a code TODO tagged `TODO(vkodithala)` at the macOS recorder's deferred window-target branch and a PR-description note stating that window-scoped macOS recording is a follow-on; do not implement crop or ScreenCaptureKit here.
8. `./script/presubmit` passes from the `warp` repository root, with no Linux or unrelated server changes.
