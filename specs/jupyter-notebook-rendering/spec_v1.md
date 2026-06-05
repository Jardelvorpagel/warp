# Jupyter Notebook Editing (cell-based) — SPEC v1
Umbrella spec for making `.ipynb` files **editable in their rendered, cell-based form** in Warp, and saveable back to valid notebook JSON — without running a kernel. This document frames the feature and links the detailed specs; it does not restate them.
- Product behavior (numbered invariants): `product_v1.md`
- Technical design (implementation, testing, parallelization): `tech_v1.md`
- Prior, shipped render-only feature (read-only, no editing): `PRODUCT.md` + `TECH.md`
## Summary
Today Warp can render a `.ipynb` as a read-only document by flattening the whole notebook into one Markdown buffer. That projection is lossy and one-way, so it cannot support editing cells or writing changes back. This feature replaces the read-only surface (when enabled) with a structured, cell-based editor: each cell is an editable unit, saved outputs render read-only beneath their code cell, and an explicit save writes a valid `.ipynb` back to disk preserving everything the user did not touch.
## Why a new approach (not an extension of v0)
The shipped design treats a notebook as "text to render." Editing requires treating it as "a structured document whose cells are editable units." The two cannot share the single flattened buffer:
- The Markdown projection erases cell boundaries, cell types, ids, metadata, and `execution_count`, and intermixes outputs with source — none of which can be reconstructed to save valid `.ipynb`.
- The v0 view is intentionally read-only and has no write-back path.
So v1 keeps v0's *rendering rules* (reused for read-only outputs and markdown display) but introduces a lossless notebook model, per-cell editors, and a save path. The shipped `TECH.md` explicitly deferred this "dedicated cell-based view" (`TECH.md:54`); v1 builds it.
## Scope
In scope:
- Editing markdown cells (rendered WYSIWYG) and code cells (syntax-highlighted source).
- Cell structural operations: insert, delete, move, change cell type.
- Saving back to `.ipynb` with full round-trip preservation of untouched data (outputs, metadata, ids, unknown fields).
- Robust fallback to raw-JSON editing for unparseable/non-v4 files; local and remote files.
Out of scope (v1 non-goals):
- Any cell execution, kernel, or output recomputation.
- High-fidelity rendering of `text/html`/LaTeX/table/widget outputs (preserved on save, not rendered).
- Real-time collaboration and format conversion.
## Core requirements
1. **Lossless, round-trippable model.** Load → edit → save must preserve every field the editor does not model (outputs, metadata, ids, `nbformat`, unknowns) byte-for-byte for untouched cells. (`product_v1.md` 11, 18)
2. **Edit in rendered/cell form.** Markdown cells edited as rendered rich text; code cells as highlighted source. (`product_v1.md` 3, 4, 6, 7)
3. **Outputs are read-only and never recomputed.** No execution affordance; saved outputs carried through unchanged. (`product_v1.md` 5, 9)
4. **Safe save-back.** Explicit save with dirty tracking, version-based conflict detection, and no silent data loss on external change or save failure. (`product_v1.md` 10–13)
5. **Graceful degradation.** Bad notebooks fall back to raw JSON; oversized outputs degrade in display only, never in the saved file. (`product_v1.md` 16, 17)
6. **Flagged rollout.** Gated behind a new feature flag; flag-off behavior is unchanged. (`product_v1.md` 22)
## Acceptance criteria
The feature is complete when every numbered invariant in `product_v1.md` has a passing test or verification step as enumerated in `tech_v1.md` ("Testing and validation"), and `./script/format` + `cargo clippy` pass per `WARP.md`. The highest-priority gates are: lossless round trip (11, 18), per-cell edit fidelity (6, 7), outputs untouched by edits (9), and safe save/conflict handling (10–13).
## Open questions
- **Undo granularity:** per-cell undo vs. notebook-wide undo for structural ops (see `tech_v1.md` Risks). v1 default proposal: per-cell text undo, notebook-level history for insert/delete/move/convert.
- **Raw toggle with unsaved edits:** carry edits into the raw JSON view automatically, or warn before switching? (`product_v1.md` 19)
- **Save trigger:** explicit-save only for v1, or also autosave-on-blur consistent with other Warp file surfaces?
## Relationship to feature flags
v1 introduces a new flag (proposed `JupyterNotebookEditing`) separate from the existing `JupyterNotebookRendering` (`crates/warp_features/src/lib.rs:511`), so editing can roll out independently of (and on top of) the read-only renderer.
