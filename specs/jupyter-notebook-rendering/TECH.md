# Jupyter Notebook Rendering — TECH

Render `.ipynb` (Jupyter) files as formatted notebooks instead of raw JSON. **Render-only**: no kernel, no cell execution, no editing of outputs. Outputs are display-only (text, tracebacks, and embedded images). Behind a feature flag.

## Context
Warp already has a non-code "rendered file" surface, `FileNotebookView` (`app/src/notebooks/file/mod.rs`), which is misleadingly named — "Notebook" refers to Warp Drive rich-text notebooks, not Jupyter. It loads a file's text and feeds it straight to the markdown parser via `reset_with_markdown` (`crates/editor/src/model.rs:899`). The same renderer already supports images: `![alt](src)` serializes to `<img>` (`crates/editor/src/content/markdown_tests.rs:264`), and base64 data-URIs are used today for Mermaid (`app/src/notebooks/editor/model.rs:137`).

Two things block `.ipynb` rendering today:

1. **Routing.** `FileNotebookView` is only reached via `FileTarget::MarkdownViewer`, which `resolve_file_target_with_editor_choice` (`app/src/util/openable_file_type.rs:196`) returns only when `is_markdown_file` is true. That predicate (`crates/warp_util/src/file_type.rs:130`) matches only `md`/`markdown`. An `.ipynb` is reported as `application/json`, classified `OpenableFileType::Text` (`openable_file_type.rs:143`), and routed to `FileTarget::CodeEditor` → opens as raw JSON in `CodeView`.
2. **Content handling.** Even if routed there, `FileNotebookView::set_content` (`app/src/notebooks/file/mod.rs:329`, and the remote path at `:630`) passes raw bytes to `reset_with_markdown`. For an `.ipynb` that's JSON, so it would render JSON-as-markdown, not cells.

Relevant existing wiring this feature reuses:
- Open flow: `Workspace::open_file_with_target` `FileTarget::MarkdownViewer` arm (`app/src/workspace/view.rs:5986`) → `open_file_notebook` → `FilePane::new` (`app/src/pane_group/pane/file_pane.rs:43`) → `FileNotebookView`.
- Rendered⇄Raw toggle: `FileNotebookView` "Raw" emits `ReplaceWithCodePane` (`notebooks/file/mod.rs:1069`); `CodeView` "Rendered" (`CodeViewAction::RenderMarkdown`, `code/view.rs:2249`) emits `ReplaceWithFilePane`. Both are gated on `is_markdown_file` (`code/view.rs:284`, `:2069`; `notebooks/file/mod.rs:702`, `:1165`).

## Proposed changes
The only substantial new code is an `.ipynb` → markdown converter; everything else is detection + a one-line conversion hook + extending the "is renderable" predicate.

**1. Detection — `crates/warp_util/src/file_type.rs`**
Add `is_jupyter_notebook_file(path)` matching the `ipynb` extension (sibling to `is_markdown_file`).

**2. Routing — `app/src/util/openable_file_type.rs`**
- **Decision (revised during implementation):** do *not* add an `OpenableFileType::JupyterNotebook` variant. That variant is matched exhaustively across many file-classification surfaces (the "Open in Warp" banner in `app/src/terminal/view/open_in_warp.rs`, menu/banner/workspace code), so a new variant would force churn across all of them and widen the blast radius for a flag-gated v1. `.ipynb` keeps its current classification (`OpenableFileType::Text`) so it stays openable and, with the flag off, behaves exactly as today. `.ipynb` is never `Markdown`/`Code`, so the terminal "Open in Warp" banner (which only suggests for `Markdown | Code`) does not change behavior.
- Instead, gate the render decision at the single chokepoint that chooses between the notebook viewer and the code editor: in `resolve_file_target_with_editor_choice` (and `resolve_file_target_to_open_in_warp`), when `is_jupyter_notebook_file(path)` is true, the file is openable, and the feature flag is enabled, return `FileTarget::MarkdownViewer(layout)` **unconditionally** (i.e. not gated on `prefer_markdown_viewer` or `editor_choice`, since rendering-instead-of-JSON is the whole point). When the flag is off, fall through to today's behavior (opens as JSON).
- Add a small `renders_in_warp_notebook_viewer(path)` helper (markdown OR flag-enabled jupyter) used by the Rendered/Raw toggle gates in step 5 and the view-header gate in step 4.

**3. Converter — `app/src/notebooks/file/ipynb.rs` (new)**
Pure function, no UI deps, unit-testable in isolation:
```rust path=null start=null
pub fn ipynb_to_markdown(json: &str) -> Result<String, IpynbError>;
```
- `serde` structs for nbformat v4: top-level `cells[]` + `metadata.language_info.name` (code-fence language); per cell `cell_type`, `source` (string-or-`Vec<String>`), and code-cell `outputs[]`.
- Conversion rules:
  - markdown cell → source verbatim.
  - code cell → fenced block ` ```<lang> … ``` `.
  - raw cell → unhighlighted fenced block (its contents can't inject markdown).
  - `stream` / `execute_result["text/plain"]` / `error.traceback` → fenced text block (strip ANSI from tracebacks).
  - `display_data`/`execute_result` with `image/png`|`image/jpeg` → `![output](data:image/png;base64,…)`. The payload is validated as base64 first; invalid data renders an `[output image omitted: invalid base64 data]` message instead of being embedded (prevents broken images and Markdown-injection via metacharacters in the payload).
  - `text/html` and other rich MIME → skipped in v1 (see Follow-ups).
- On parse failure, return `Err`; callers fall back to raw JSON (never blank).
- Reuses `serde_json` and `base64` (already workspace deps).

**4. View hook — `app/src/notebooks/file/mod.rs`**
- In `set_content` (`:329`), if the backing path is `.ipynb`, run `ipynb_to_markdown` and feed the result to `reset_with_markdown`; on `Err`, feed the raw JSON unchanged. This single hook covers both local (`:329`) and remote (`:630`) load paths since both call `set_content`.
- Replace the `is_markdown_file()`-gated checks (`:702`, `:1165`) with a broader "renders in notebook view" predicate (markdown OR jupyter) so the Rendered/Raw header toggle appears for `.ipynb`.

**5. Toggle parity — `app/src/code/view.rs`**
Extend the two `is_markdown_file` gates (`update_markdown_mode_segmented_control` `:284`; overflow `is_md` `:2069`) to also accept `.ipynb`. This makes "Raw" mode (JSON in `CodeView`) offer a "Rendered" toggle back to `FileNotebookView`. The existing `RenderMarkdown`→`ReplaceWithFilePane` path then works unchanged (functionally correct; the action name becomes a minor misnomer — rename optional).

**6. Feature flag — `crates/warp_features/src/lib.rs`**
Add `FeatureFlag::JupyterNotebookRendering` to the `FeatureFlag` enum (the actual enum lives in `crates/warp_features/src/lib.rs`, re-exported via `warp_core::features`; the WARP.md `warp_core/src/features.rs` reference is stale). Default-on for dogfood via `DOGFOOD_FLAGS`, add the matching `app/Cargo.toml` `[features]` entry + the `#[cfg(feature = "...")]` bridge arm in `app/src/features.rs::enabled_features`, per the `add-feature-flag` skill. Gate the routing/conversion in steps 2–5.

**Tradeoff — reuse markdown pipeline vs. dedicated cell view.** A dedicated cell-based view is the right architecture *if execution is ever the goal*, but it is a large new subsystem and none of it is needed for render-only. The markdown-reuse path ships the user-visible win (no more raw JSON) at a fraction of the cost. If execution is later prioritized, the converter's parsing + output-MIME logic carries over; the rendering surface would be rebuilt. This spec deliberately picks the cheap path.

## Testing and validation
These map to the behavior invariants in `PRODUCT.md` (each maps to a check below):

1. Opening an `.ipynb` (flag on) renders cells, not JSON. → integration: open a fixture `.ipynb`, assert a `FilePane`/`FileNotebookView` is created (not a `CodePane`); mirror `notebooks/file/mod_tests.rs`.
2. Markdown cells render as formatted markdown; code cells render as fenced code with the kernel language. → unit tests on `ipynb_to_markdown` (golden markdown output).
3. Text outputs (stream, `text/plain`, error tracebacks) render as preformatted text; ANSI stripped. → unit tests.
4. `image/png`/`image/jpeg` outputs render as images via base64 data-URI. → unit test asserting `![…](data:image/png;base64,…)`; manual screenshot to confirm the renderer displays the data-URI (key risk — see below).
5. Malformed/non-v4 `.ipynb` falls back to raw JSON, never blank/panicking. → unit test feeding invalid JSON returns `Err`; view-level test asserts raw content shown.
6. Rendered⇄Raw toggle works both directions for `.ipynb` (Raw shows JSON in `CodeView`, Rendered returns to `FileNotebookView`). → view tests extending the markdown toggle tests.
7. Flag off ⇒ unchanged behavior (`.ipynb` opens as JSON in `CodeView`). → `openable_file_type_tests.rs` assertions for both flag states (using `FeatureFlag::JupyterNotebookRendering.override_enabled(..)`).

Add `.ipynb` cases to `app/src/util/openable_file_type_tests.rs` (resolve to `MarkdownViewer` when the flag is on, `CodeEditor` when off) and `is_jupyter_notebook_file` cases alongside the markdown predicate tests. Run `./script/format` and `cargo clippy` (per WARP.md) before PR.

## Parallelization
The feature is small (~350–550 LOC) and the steps are mostly sequential: routing (step 2) and the view hook (step 4) both depend on the converter's signature (step 3) and the predicate from step 1. The cleanly isolatable unit is the converter, which has zero UI/codebase coupling behind a frozen interface (`ipynb_to_markdown(&str) -> Result<String, _>`).

Recommended default: **do it in a single PR sequentially** — coordination overhead outweighs the wall-clock savings at this size. If parallelism is still wanted, freeze the converter signature first, then split into two local agents on separate worktrees off `master`:

- **converter** (local) — owns `app/src/notebooks/file/ipynb.rs` + its unit tests only. Worktree `../warp-ipynb-converter`, branch `oz/ipynb-converter`. No other files.
- **wiring** (local) — owns steps 1, 2, 4, 5, 6 against the agreed signature (stubs the converter until merge). Worktree `../warp-ipynb-wiring`, branch `oz/ipynb-wiring`.

Merge `converter` first, then rebase `wiring` and land a single combined PR. Validation (presubmit + the integration test) is owned by `wiring` after merge.

## Risks and mitigations
- **Data-URI image rendering may differ from Mermaid's HTML `<img>`.** Validate invariant 4 early; if `![](data:…)` doesn't display, emit the Mermaid-style HTML `<img src="data:…">` instead (`notebooks/editor/model.rs:137`).
- **Large outputs / huge embedded images** can bloat the buffer. Cap per-output text length and skip/oversized-image-placeholder beyond a threshold.
- **nbformat v3 vs v4** differ in `source`/`outputs` shape. Target v4 (dominant); anything that fails to parse falls back to raw JSON (invariant 5).

## Follow-ups
- `text/html` / table / LaTeX output fidelity.
- A dedicated `prefer_notebook_viewer` setting (mirror `prefer_markdown_viewer`).
- Telemetry for notebook opens.
- Cell execution on a Python kernel — separate, much larger effort (new cell-based view + Jupyter ZeroMQ protocol client + kernel process management).
