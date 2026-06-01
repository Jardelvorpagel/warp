# Code Editor Gutter – Copy GitHub Permalink (Tech Spec)

## Overview
Implement a "Copy GitHub permalink" context-menu action in the local code editor gutter. The action computes a commit-pinned GitHub URL for the current file and line(s), then copies it to the system clipboard.

## Architecture

### New helper functions (`app/src/util/git.rs`)

1. **`build_github_permalink_sync`** – Pure, synchronous function that constructs the permalink URL from pre-fetched components:
   - `github_owner: &str`, `github_repo: &str`, `commit_sha: &str`, `repo_relative_path: &str`, `start_line: usize`, `end_line: Option<usize>`
   - Returns `String` of the form `https://github.com/{owner}/{repo}/blob/{sha}/{path}#L{start}` or `…#L{start}-L{end}`.

2. **`get_github_remote_url_sync`** – Synchronous (blocking) helper that runs `git remote get-url origin` and parses the result to extract `(owner, repo)` for `github.com` URLs. Handles both HTTPS (`https://github.com/owner/repo.git`) and SSH (`git@github.com:owner/repo.git`) formats. Returns `Option<(String, String)>`.

3. **`get_head_sha_sync`** – Synchronous helper that runs `git rev-parse HEAD` and returns the full 40-character SHA. Returns `Option<String>`.

### Changes to `app/src/code/local_code_editor.rs`

1. **Action enum** – Add `CopyGitHubPermalink` variant to `LocalCodeEditorAction`.

2. **`context_menu_items`** – Append a "Copy GitHub permalink" menu item (conditionally, based on whether a GitHub permalink can be built). The method gains access to `&AppContext` so it can check the file path and git state.

3. **`OpenContextMenu` handler** – Remove the LSP-only guard so the context menu opens even when no LSP server is available (LSP actions are still conditionally included). This ensures the GitHub permalink item appears for any tracked file.

4. **`CopyGitHubPermalink` handler** – When selected:
   a. Resolve the file's absolute path → repo root → relative path.
   b. Call `get_github_remote_url_sync` and `get_head_sha_sync` with the repo root.
   c. Determine the line range: use the editor's current selection (head and tail offsets converted to 1-indexed line numbers). If the selection spans one line, use a single `#L{n}` anchor; otherwise `#L{start}-L{end}`.
   d. Build the URL with `build_github_permalink_sync`.
   e. Write to clipboard via `ctx.clipboard().write(ClipboardContent::plain_text(url))`.
   f. Show a success toast via `ToastStack`.

### Selection → line range

The editor model exposes `selections()` which returns a `RenderedSelectionSet`. The first selection's `head` and `tail` `CharOffset` values are converted to 1-indexed line numbers using `offset_to_lsp_position` (which returns 0-indexed LSP lines; add 1). The smaller line number is `start_line`, the larger is `end_line`.

### Repo root detection

Reuse the existing pattern from `enable_lsp_for_path`: check `PersistedWorkspace::root_for_workspace`, fall back to `DetectedRepositories::get_root_for_path`, then `file_path.parent()`.

### Testing

- Unit tests for `build_github_permalink_sync` covering single-line, multi-line, and edge cases.
- Unit tests for `get_github_remote_url_sync` parsing HTTPS and SSH remote URLs.
- GUI verification via computer-use (screenshots captured as artifacts).

## Files changed
| File | Change |
|------|--------|
| `app/src/util/git.rs` | Add `build_github_permalink_sync`, `get_github_remote_url_sync`, `get_head_sha_sync`, and unit tests |
| `app/src/util/git_tests.rs` | Add tests for the new functions |
| `app/src/code/local_code_editor.rs` | Add `CopyGitHubPermalink` action, update `context_menu_items`, update `OpenContextMenu` handler, add permalink handler |
