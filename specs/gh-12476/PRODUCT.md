# gh-12476: Agent Management repository filter
## Summary
Add a Repository filter to Warp's Agent Management panel so users can see accessible local conversations and cloud runs relevant to the Git repository they are working in. The filter reuses Agent Management's existing searchable, single-select filter behavior and repository metadata Warp already has.

Related issue: https://github.com/warpdotdev/Warp/issues/12476

## Problem
Agent Management combines activity from many projects, but its existing filters cannot answer “show me the agent conversations and runs relevant to this repository.” Filtering by exact working directory is too narrow because work in one repository may span sibling directories or multiple local checkouts.

## Goals
- Let users filter Agent Management to one repository, especially the repository containing the active pane's current directory.
- Include both local conversations and cloud runs when Warp already has enough metadata to associate them with the selected repository.
- Extend existing Agent Management filter behavior without introducing a new organization or grouping system.

## Non-goals
- Grouping or visually organizing Agent Management rows by repository.
- Renaming conversations or assigning conversations to manually created projects.
- Filtering by an exact working directory.
- Selecting multiple repositories at once.
- Loading full historical conversations solely to discover additional repositories.
- Resolving repository identity for historical activity from another machine or an unavailable remote session.
- Changing the separate inline conversations menu or its Current Directory tab.
- Requiring server or public API changes.

## Figma
Figma: none provided

## Behavior

### Repository filter

1. Agent Management includes a single-select **Repository** filter alongside its existing filters. The filter is available in both the Personal and All views.

2. The Repository filter defaults to **All**. With All selected, repository association does not affect which entries appear.

3. The Repository filter menu contains:
   - **All**.
   - **Current repository**.
   - **No repository**.
   - A searchable, deduplicated list of repositories Warp can identify from entries Agent Management can list and from accessible cloud environments.

4. Repository options are independent of the other active Agent Management filters. Applying another filter or title search does not remove options from the Repository menu.

5. Selecting an option closes the menu, updates the visible selection, and filters the list immediately using the repository associations Warp currently knows.

6. The Repository filter composes with every existing Agent Management filter and title search using AND semantics.

7. Repository filtering does not otherwise change row ordering, row content, available actions, or what happens when a row is opened.

### Repository identity

8. When Warp can derive a repository's GitHub owner and name, it displays and identifies the repository as `owner/repo`. This is the only case where Warp treats activity from different locations as the same repository.

9. Known repositories with the same name but different owners remain distinct options and do not match one another. `owner/repo` comparison is case-insensitive.

10. When Warp resolves a local Git root but cannot derive a known `owner/repo` — for example, the repository has no `origin` remote, the remote is not GitHub-hosted, or the remote URL is unrecognized — the repository is identified by that Git root. Its option is labeled with the root directory's name, with enough path context to distinguish same-named roots.

11. A Git-root identity matches only entries resolved to that same root. Warp does not guess that repositories are the same based on a shared directory or repository name alone, and does not merge a Git-root identity with any `owner/repo` identity.

12. Different local checkouts or Git worktrees that resolve to the same known `owner/repo` match the same option. Checkouts without a known `owner/repo` remain separate options per Git root.

### Entry association

13. A local conversation is associated with repositories Warp can resolve from the conversation's recorded initial and latest working directories.

14. Resolving a local working directory uses its nearest enclosing Git repository. A nested repository or submodule is associated independently from its parent repository.

15. Warp does not load a full historical conversation or scan all of its actions solely to enrich repository filtering.

16. A cloud run is associated with every repository listed by its accessible cloud environment or already available run metadata.

17. An entry associated with multiple repositories matches when any association matches the selected repository.

18. Local conversations from different checkouts and cloud runs from different environments match one known `owner/repo` selection when their available metadata identifies that same repository.

19. A recorded working directory that cannot be safely resolved in the current context, including a path from another machine or an unavailable remote session, does not produce a repository association.

20. Warp does not guess repository association from titles, prompts, branch names, arbitrary path text, or similarly ambiguous content.

21. Repository options and matches include only entries and environments the user is allowed to access.

### Current repository and No repository

22. **Current repository** resolves the active terminal pane's current directory to its nearest enclosing Git repository.

23. When Current repository resolves successfully, Warp selects and persists the resulting concrete repository. Subsequent pane or directory changes do not silently change the active filter.

24. When there is no active terminal pane, its directory is not inside a Git repository, or Warp cannot resolve it, Current repository remains visible but unavailable and explains that no repository is available from the current pane.

25. Resolving Current repository does not block use of the rest of Agent Management. Until resolution succeeds, selecting it does not change the active filter.

26. **No repository** shows entries for which repository resolution has completed and found no enclosing Git repository. An entry that resolves to a Git root without a known `owner/repo` is associated with that Git-root identity and does not appear under No repository.

27. An entry whose repository association has not finished resolving matches neither a concrete repository nor No repository. It may begin matching after resolution completes.

### Existing filter behavior

28. Typing in the Repository menu searches repository options by their displayed labels, including `owner/repo` values and Git-root directory names. Search is case-insensitive.

29. The Repository menu uses the same mouse, keyboard navigation, focus, selection, dismissal, no-match, and accessibility behavior as Agent Management's existing searchable filter menus.

30. Repository resolution does not block Agent Management from opening. Known options and matching entries update as additional associations become available without changing the active selection.

31. If repository resolution fails, Warp preserves the active filter and known matching results. It does not silently clear the selection or show unfiltered entries.

32. A selected repository, All, or No repository persists and restores in the same situations as Agent Management's existing filters. Current repository persists as the concrete repository it resolved to.

33. If a persisted repository selection is no longer discoverable, Warp keeps it selected and shows the standard filtered no-results state until the user changes or clears it.

34. Agent Management's existing **Clear all** and filtered no-results **Clear filters** actions reset Repository to All. Switching between Personal and All does not clear the Repository selection.

35. When no entries match, Agent Management shows its standard filtered no-results state and clear-filters affordance.

36. If Warp cannot discover any repositories, the menu still offers All, Current repository in its appropriate available or unavailable state, and No repository.
