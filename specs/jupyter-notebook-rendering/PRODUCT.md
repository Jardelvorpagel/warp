# Jupyter Notebook Rendering — PRODUCT

## Summary
Opening a `.ipynb` (Jupyter) file in Warp shows a formatted, read-only notebook — markdown cells as rich text, code cells as syntax-highlighted code, and saved outputs (text, tracebacks, images) inline — instead of the raw JSON the file contains today. This is render-only: no kernel, no cell execution, no editing of outputs.

## Figma
Figma: none provided. Visual presentation reuses Warp's existing rendered-markdown surface, so cell content, code fences, and images match how Warp already renders markdown elsewhere.

## Goals / Non-goals
Goals: replace the raw-JSON view of `.ipynb` files with a readable rendering of the notebook's existing content.
Non-goals: running a kernel, executing or re-executing cells, editing cells or outputs, round-tripping edits back to the `.ipynb` file, or high-fidelity rendering of rich HTML/LaTeX/table outputs (v1 skips these).

## Behavior

1. With the feature enabled, opening a file whose extension is `.ipynb` displays a rendered notebook view rather than the file's raw JSON. This is the default for `.ipynb` regardless of any "prefer markdown viewer" preference, because showing JSON is precisely what the feature removes.

2. Cells render in document order, top to bottom, matching their order in the file.

3. A markdown cell renders as formatted markdown identical to how Warp renders markdown elsewhere (headings, lists, bold/italic, inline code, links, images, etc.).

4. A code cell renders as a syntax-highlighted fenced code block. The highlight language comes from the notebook's kernel/language metadata (e.g. `python`); if the notebook declares no language, the code still renders as a code block without language-specific highlighting. Jupyter `raw` cells render as an unhighlighted code block so their contents cannot inject unexpected markdown.

5. A code cell's source renders verbatim, preserving original line breaks, indentation, and blank lines. Cell input prompt numbers (e.g. `In [3]:`) are not shown.

6. Saved cell outputs render directly beneath their code cell, in the order they were saved:
   - Stream output (stdout/stderr) renders as preformatted text.
   - A `text/plain` result renders as preformatted text.
   - An error/traceback renders as preformatted text with terminal color/escape (ANSI) codes stripped, so it reads as plain text rather than showing raw escape sequences.

7. Image outputs (`image/png`, `image/jpeg`) render as inline images using the data already embedded in the notebook — no network fetch. The user sees the rendered image, not its encoded data. If a cell's image data is not valid base64, the image is not shown and a short message indicating invalid image data appears in its place.

8. Output types not supported in v1 (e.g. `text/html`, rich tables, LaTeX/MathJax, interactive widgets) are skipped: they are not rendered and do not appear as raw markup or encoded blobs. Their absence must never blank out or corrupt the surrounding cells — every other cell and output still renders.

9. A notebook with no cells, or cells with empty source, renders as an empty/short notebook without error.

10. A code cell that produced no saved outputs renders as just the code block, with nothing beneath it.

11. Robust fallback: if the file is not a parseable notebook in the supported format (malformed JSON, an unsupported/older nbformat version, or otherwise unreadable as a notebook), Warp falls back to showing the file's raw text content. It must never show a blank view and must never crash on a bad `.ipynb`.

12. Oversized content degrades gracefully rather than freezing the UI: very large text outputs may be truncated, and very large embedded images may be omitted (with a visible placeholder) beyond a reasonable threshold. The rest of the notebook still renders.

13. The rendered notebook is read-only. The user cannot edit cells or outputs, and there is no run/execute affordance.

14. A user viewing a rendered `.ipynb` can switch to a raw view that shows the file's underlying JSON, and can switch back to the rendered view. This Rendered⇄Raw toggle is available for `.ipynb` files the same way it is for markdown files. The raw view reflects the file's actual on-disk JSON.

15. The feature works for `.ipynb` files opened both locally and from a remote/SSH session; the rendered result is the same in both cases.

16. When the feature is disabled, `.ipynb` files behave exactly as they do today: they open as raw JSON in the code editor, with no rendered-notebook view and no notebook-specific toggle.
