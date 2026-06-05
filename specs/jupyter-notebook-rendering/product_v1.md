# Jupyter Notebook Editing (cell-based) — PRODUCT v1
## Summary
Opening a `.ipynb` (Jupyter) file in Warp shows an **editable, cell-based** notebook: markdown cells render as rich, editable formatted text; code cells render as editable syntax-highlighted source; and saved outputs (text, tracebacks, images) appear read-only beneath their code cell. The user can edit cell contents, add/remove/reorder cells, and **save the changes back to the `.ipynb` file**. This is **edit-only**: there is no kernel, no cell execution, and outputs are never recomputed.
## Problem
The shipped behavior (see sibling `PRODUCT.md`) renders a `.ipynb` read-only by flattening the whole notebook into a single Markdown document. That projection is lossy and one-way, so it cannot support editing individual cells or writing changes back to the file. This v1 makes the rendered notebook a real editing surface while keeping the same visual presentation for content that is only being viewed.
## Goals / Non-goals
Goals: let the user edit a `.ipynb` directly in its rendered, cell-based form and save valid `.ipynb` back to disk, preserving everything the user did not touch (outputs, metadata, cell ids, unknown fields).
Non-goals: running a kernel, executing or re-executing cells, recomputing or clearing outputs, high-fidelity rendering of rich `text/html`/LaTeX/table/widget outputs, real-time collaborative editing, and converting between `.ipynb` and other formats. These are out of scope for v1.
## Behavior
1. With the feature enabled, opening a file whose extension is `.ipynb` displays an editable cell-based notebook view — not the file's raw JSON, and not a single flattened Markdown document. This is the default for `.ipynb` regardless of any "prefer markdown viewer" preference.
2. Cells render in document order, top to bottom, matching their order in the file.
3. A markdown cell renders as formatted, **editable** markdown, identical to how Warp renders and edits markdown in Warp Drive notebooks (headings, lists, bold/italic, inline code, links, images, etc.). The user edits it in rendered (WYSIWYG) form.
4. A code cell renders as an **editable**, syntax-highlighted code block. The highlight language comes from the notebook's kernel/language metadata (e.g. `python`); a notebook that declares no language still renders editable code without language-specific highlighting. The cell's source is shown verbatim, preserving line breaks, indentation, and blank lines. Input prompt numbers (e.g. `In [3]:`) are not shown.
5. A code cell's saved outputs render directly beneath it, in saved order, and are **read-only** (the user cannot place a cursor in or edit them):
   - Stream output (stdout/stderr) renders as preformatted text.
   - A `text/plain` result renders as preformatted text.
   - An error/traceback renders as preformatted text with ANSI escape codes stripped.
   - `image/png` / `image/jpeg` outputs render as inline images using the embedded data (no network fetch). Image data that is not valid base64 shows a short "invalid image data" message in place of the image.
6. Editing a markdown cell's content updates only that cell. On save, the change is written back; cells the user did not edit are not reformatted or otherwise altered.
7. Editing a code cell's source updates only that cell's source and preserves exact whitespace, indentation, blank lines, and trailing newlines on save.
8. The user can perform cell structural operations: insert a new cell (markdown or code) above or below an existing cell, delete a cell, move a cell up or down, and change a cell's type between code and markdown.
   - A newly inserted cell starts empty and (for code cells) has no outputs.
   - Converting a code cell to a markdown cell drops that cell's outputs and execution count (outputs belong only to code cells); converting a markdown cell to a code cell yields a code cell with empty source-equivalent content and no outputs.
9. Editing never executes anything. A code cell's existing saved outputs remain unchanged when its source is edited, and are written back as-is. There is no run/execute affordance anywhere in the view.
10. The view shows whether there are unsaved changes (a dirty indicator). The indicator clears once changes are saved.
11. Saving (explicit save, e.g. the standard save shortcut) writes a valid `.ipynb` to disk that preserves, for untouched data: cell order, cell ids, per-cell and notebook-level metadata, `execution_count`, all outputs (including types this view does not render), `nbformat`/`nbformat_minor`, and any other fields the editor does not model. Only content the user actually changed is changed.
12. If a save fails (e.g. permission denied, disk error, remote write failure), the user is shown an error, the edits remain in the buffer (nothing is lost), and the dirty state persists so the user can retry.
13. If the underlying file changes on disk while the notebook is open:
    - With no unsaved edits, the view reloads to reflect the new on-disk content.
    - With unsaved edits, the view does not silently discard the user's changes; it surfaces the conflict and lets the user choose to keep their edits or reload from disk.
14. A notebook with no cells, or cells with empty source, renders without error, and the user can add a first cell.
15. A code cell that produced no saved outputs renders as just the editable code block, with nothing beneath it.
16. Robust fallback: if the file is not a parseable notebook in the supported format (malformed JSON, an unsupported/older `nbformat` version, or otherwise unreadable as a notebook), Warp falls back to showing the file's raw text in the code editor, where it can be edited and saved as plain text. It must never show a blank view and must never crash on a bad `.ipynb`.
17. Display-side limits never corrupt saved data: very large text outputs may be visually truncated and very large embedded images may be omitted with a visible placeholder, but saving preserves the original output bytes in full. What is truncated on screen is never truncated in the file.
18. Output types not rendered in v1 (e.g. `text/html`, rich tables, LaTeX/MathJax, interactive widgets) are not displayed as raw markup or encoded blobs, but they are preserved verbatim on save — they are never dropped from the file.
19. A user viewing the notebook can switch to a raw view of the file's underlying JSON and back, the same way the Rendered⇄Raw toggle works for markdown files. If there are unsaved edits in the rendered view when switching to Raw, the user's edits are not silently lost (edits are either carried into the raw JSON or the user is warned before switching).
20. The feature works for `.ipynb` files opened both locally and from a remote/SSH session; in both cases the notebook is editable and saving writes back to the correct (local or remote) file.
21. When the file cannot be written (e.g. the remote host is disconnected, or the location is not writable), editing affordances and/or save are disabled with a clear indication of why, rather than appearing to save and silently failing.
22. When the feature is disabled, `.ipynb` files behave exactly as they do without it (raw JSON in the code editor, or the read-only render-only view if that earlier feature is separately enabled); there is no cell-based editor and no notebook-specific editing.
23. Keyboard and focus: the user can move focus between cells and edit within a focused cell using standard editor navigation; read-only output regions are skipped for text entry but remain selectable for copy.
