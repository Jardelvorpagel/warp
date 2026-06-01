# Code Editor Gutter – Copy GitHub Permalink

## Summary
Add a "Copy GitHub permalink" action to the code editor's line-number gutter context menu. When a user right-clicks on the gutter of a file that lives in a GitHub-backed repository, they can copy a commit-pinned permalink to the clipboard. For multi-line selections the permalink includes the full line range.

## Problem
Developers frequently need to share links to specific lines of code in GitHub. Today this requires manually navigating to GitHub, finding the file, selecting the lines, and copying the URL. Warp already has the file open in its editor and knows the git context, so we can eliminate this friction.

## Goals
- Expose a "Copy GitHub permalink" context-menu item when right-clicking the line-number gutter in the local code editor.
- Generate a commit-pinned URL of the form `https://github.com/{owner}/{repo}/blob/{sha}/{path}#L{line}`.
- For multi-line selections, produce a range anchor `#L{start}-L{end}`.
- Copy the URL to the system clipboard and show a success toast.

## Non-goals
- Supporting non-GitHub remotes (GitLab, Bitbucket, etc.).
- Opening the permalink in a browser.
- Showing the action for files that are not tracked by a GitHub-backed git repository.

## User experience

### Trigger
Right-click on any line number in the code editor gutter. The existing context menu appears with a new "Copy GitHub permalink" item appended (below any LSP actions such as "Go to definition" and "Find references").

### Single-line
When no multi-line selection exists, the permalink targets the line the user right-clicked on. The URL ends with `#L{n}` (1-indexed).

### Multi-line selection
When the user has selected a range of lines (e.g., lines 4–8), right-clicking on the gutter within the selected range produces a permalink ending with `#L{start}-L{end}`.

### Feedback
After the URL is copied, a success toast ("Copied GitHub permalink") appears briefly.

### Unavailable state
The menu item is **not shown** when:
- The file is not inside a git worktree.
- The git remote does not point to a GitHub host.
- The HEAD commit SHA cannot be determined (e.g., empty repo).

### Edge cases
- **Detached HEAD**: The full HEAD SHA is used, which is valid for a permalink.
- **Uncommitted new file**: The file has no committed SHA, so the item is hidden.
- **Renamed/moved file**: The permalink uses the file's current path relative to the repo root, pinned to HEAD. If HEAD doesn't contain the file at that path, GitHub will 404 — acceptable since we always point at the commit that the user is actually on.

## Success criteria
1. Right-clicking a gutter line number shows "Copy GitHub permalink" for GitHub-backed files.
2. Clicking it copies a valid, commit-pinned permalink.
3. Multi-line selections produce `#Lx-Ly` range anchors.
4. A success toast confirms the copy.
5. The item is absent for non-GitHub or non-git files.

## Validation
- Manual verification via computer-use: open a tracked file, right-click gutter, verify menu text, click, paste URL, confirm format.
- Unit tests for permalink URL construction logic.
