use serde_json::{json, Value};

use super::{CellDoc, CellKind, IpynbError, NotebookDoc, Source};

/// Parse JSON text into a `serde_json::Value` for order-independent comparison.
fn value(json: &str) -> Value {
    serde_json::from_str(json).expect("test JSON should be valid")
}

/// Assert that parsing `json` and serializing it back is semantically lossless
/// (every field preserved), comparing as `Value` so key/field ordering is
/// ignored.
fn assert_round_trips(json: &str) {
    let doc = NotebookDoc::parse(json).expect("should parse as v4 notebook");
    let reserialized = doc.to_json_pretty();
    assert_eq!(
        value(&reserialized),
        value(json),
        "round trip changed notebook content\n--- reserialized ---\n{reserialized}"
    );
}

const SIMPLE_NOTEBOOK: &str = r##"{
 "cells": [
  {
   "cell_type": "markdown",
   "metadata": {},
   "source": ["# Title\n", "\n", "Some *text*."]
  },
  {
   "cell_type": "code",
   "execution_count": 3,
   "metadata": {"tags": ["keep"]},
   "outputs": [
    {"name": "stdout", "output_type": "stream", "text": ["hello\n"]}
   ],
   "source": ["print('hello')"]
  }
 ],
 "metadata": {
  "kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"},
  "language_info": {"name": "python", "version": "3.11.0"}
 },
 "nbformat": 4,
 "nbformat_minor": 5
}"##;

#[test]
fn parses_and_round_trips_simple_notebook() {
    assert_round_trips(SIMPLE_NOTEBOOK);
}

#[test]
fn preserves_unknown_fields_at_every_level() {
    // Unknown keys at the notebook, cell, and output levels must all survive.
    let json = r##"{
 "cells": [
  {
   "cell_type": "code",
   "id": "abc123",
   "execution_count": 1,
   "metadata": {},
   "future_cell_field": {"nested": true},
   "outputs": [
    {"output_type": "display_data", "data": {"text/html": "<b>x</b>"}, "metadata": {}, "future_output_field": 7}
   ],
   "source": ["x = 1"]
  }
 ],
 "metadata": {"custom_top_level": [1, 2, 3]},
 "nbformat": 4,
 "nbformat_minor": 5,
 "future_top_level_field": "preserved"
}"##;
    assert_round_trips(json);
}

#[test]
fn preserves_string_and_array_source_forms() {
    // A cell whose source is a plain string, and one whose source is a list, must
    // each be preserved in their original form.
    let json = r##"{
 "cells": [
  {"cell_type": "markdown", "metadata": {}, "source": "single string source"},
  {"cell_type": "markdown", "metadata": {}, "source": ["line one\n", "line two"]}
 ],
 "metadata": {},
 "nbformat": 4,
 "nbformat_minor": 5
}"##;
    assert_round_trips(json);

    let doc = NotebookDoc::parse(json).unwrap();
    assert!(matches!(doc.cells[0].source, Source::Text(_)));
    assert!(matches!(doc.cells[1].source, Source::Lines(_)));
}

#[test]
fn preserves_empty_outputs_array_on_code_cell() {
    // A code cell with an explicit empty outputs array keeps it; a markdown cell
    // (no outputs key) does not gain one.
    let json = r##"{
 "cells": [
  {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": []},
  {"cell_type": "markdown", "metadata": {}, "source": []}
 ],
 "metadata": {},
 "nbformat": 4,
 "nbformat_minor": 5
}"##;
    assert_round_trips(json);

    let doc = NotebookDoc::parse(json).unwrap();
    assert_eq!(doc.cells[0].outputs, Some(vec![]));
    assert_eq!(doc.cells[1].outputs, None);
}

#[test]
fn rejects_non_v4_notebook() {
    let json = r##"{"cells": [], "metadata": {}, "nbformat": 3, "nbformat_minor": 0}"##;
    let err = NotebookDoc::parse(json).unwrap_err();
    assert!(matches!(
        err,
        IpynbError::UnsupportedFormat {
            nbformat: Some(3)
        }
    ));
}

#[test]
fn rejects_missing_nbformat() {
    let json = r##"{"cells": [], "metadata": {}}"##;
    let err = NotebookDoc::parse(json).unwrap_err();
    assert!(matches!(
        err,
        IpynbError::UnsupportedFormat { nbformat: None }
    ));
}

#[test]
fn rejects_malformed_json() {
    let err = NotebookDoc::parse("{ not valid json").unwrap_err();
    assert!(matches!(err, IpynbError::Parse(_)));
}

#[test]
fn empty_notebook_round_trips() {
    let json = r##"{"cells": [], "metadata": {}, "nbformat": 4, "nbformat_minor": 5}"##;
    assert_round_trips(json);
    let doc = NotebookDoc::parse(json).unwrap();
    assert!(doc.cells.is_empty());
}

#[test]
fn language_reads_from_metadata() {
    let doc = NotebookDoc::parse(SIMPLE_NOTEBOOK).unwrap();
    assert_eq!(doc.language().as_deref(), Some("python"));
}

#[test]
fn language_falls_back_to_kernelspec() {
    let json = r##"{
 "cells": [],
 "metadata": {"kernelspec": {"language": "rust", "name": "rust"}},
 "nbformat": 4,
 "nbformat_minor": 5
}"##;
    let doc = NotebookDoc::parse(json).unwrap();
    assert_eq!(doc.language().as_deref(), Some("rust"));
}

#[test]
fn editing_code_source_leaves_outputs_untouched() {
    let mut doc = NotebookDoc::parse(SIMPLE_NOTEBOOK).unwrap();
    let original_outputs = doc.cells[1].outputs.clone();

    doc.cells[1].set_source("print('changed')\nprint('again')");

    assert_eq!(doc.cells[1].source_text(), "print('changed')\nprint('again')");
    // Outputs are not recomputed or otherwise altered by a source edit.
    assert_eq!(doc.cells[1].outputs, original_outputs);
    // The markdown cell is unchanged.
    assert_eq!(doc.cells[0].source_text(), "# Title\n\nSome *text*.");
}

#[test]
fn set_source_preserves_whitespace_and_blank_lines() {
    let mut cell = CellDoc::new_code("");
    cell.set_source("a\n\n    indented\n");
    assert_eq!(cell.source_text(), "a\n\n    indented\n");
    // Stored in nbformat list form, splitting on newlines but keeping them.
    match &cell.source {
        Source::Lines(lines) => assert_eq!(lines, &["a\n", "\n", "    indented\n"]),
        Source::Text(_) => panic!("edited source should be stored as lines"),
    }
}

#[test]
fn insert_remove_and_move_cells() {
    let mut doc = NotebookDoc::parse(SIMPLE_NOTEBOOK).unwrap();
    assert_eq!(doc.cells.len(), 2);

    doc.insert_cell(1, CellDoc::new_markdown("inserted"));
    assert_eq!(doc.cells.len(), 3);
    assert_eq!(doc.cells[1].source_text(), "inserted");

    // Insert past the end clamps to the end.
    doc.insert_cell(99, CellDoc::new_code("last"));
    assert_eq!(doc.cells.last().unwrap().source_text(), "last");

    let removed = doc.remove_cell(1).expect("cell exists");
    assert_eq!(removed.source_text(), "inserted");
    assert_eq!(doc.cells.len(), 3);

    assert!(doc.move_cell(0, 2));
    assert!(!doc.move_cell(0, 99));
}

#[test]
fn convert_code_to_markdown_drops_outputs_and_execution_count() {
    let mut doc = NotebookDoc::parse(SIMPLE_NOTEBOOK).unwrap();
    let code = &mut doc.cells[1];
    assert_eq!(code.kind(), Some(CellKind::Code));

    code.convert_to(CellKind::Markdown);

    assert_eq!(code.kind(), Some(CellKind::Markdown));
    assert_eq!(code.outputs, None);
    assert!(!code.extra.contains_key("execution_count"));
    // The serialized markdown cell has no outputs/execution_count keys.
    let serialized = serde_json::to_value(&doc.cells[1]).unwrap();
    assert!(serialized.get("outputs").is_none());
    assert!(serialized.get("execution_count").is_none());
}

#[test]
fn convert_markdown_to_code_adds_outputs_and_execution_count() {
    let mut cell = CellDoc::new_markdown("# heading");
    assert_eq!(cell.outputs, None);

    cell.convert_to(CellKind::Code);

    assert_eq!(cell.kind(), Some(CellKind::Code));
    assert_eq!(cell.outputs, Some(vec![]));
    assert_eq!(cell.extra.get("execution_count"), Some(&Value::Null));
}

#[test]
fn new_cells_have_expected_shape() {
    let md = CellDoc::new_markdown("hello");
    assert_eq!(md.kind(), Some(CellKind::Markdown));
    assert_eq!(md.outputs, None);
    assert_eq!(md.extra.get("metadata"), Some(&json!({})));
    assert!(!md.extra.contains_key("execution_count"));

    let code = CellDoc::new_code("x = 1");
    assert_eq!(code.kind(), Some(CellKind::Code));
    assert_eq!(code.outputs, Some(vec![]));
    assert_eq!(code.extra.get("metadata"), Some(&json!({})));
    assert_eq!(code.extra.get("execution_count"), Some(&Value::Null));
}

#[test]
fn unknown_cell_type_is_preserved_but_unmodeled() {
    let json = r##"{
 "cells": [
  {"cell_type": "special", "metadata": {}, "source": ["x"]}
 ],
 "metadata": {},
 "nbformat": 4,
 "nbformat_minor": 5
}"##;
    assert_round_trips(json);
    let doc = NotebookDoc::parse(json).unwrap();
    assert_eq!(doc.cells[0].kind(), None);
}

#[test]
fn edited_then_serialized_notebook_reparses() {
    let mut doc = NotebookDoc::parse(SIMPLE_NOTEBOOK).unwrap();
    doc.cells[1].set_source("print('edited')");
    doc.insert_cell(0, CellDoc::new_markdown("## new intro"));

    let json = doc.to_json_pretty();
    let reparsed = NotebookDoc::parse(&json).expect("edited notebook is still valid v4");
    assert_eq!(reparsed.cells.len(), 3);
    assert_eq!(reparsed.cells[0].source_text(), "## new intro");
    assert_eq!(reparsed.cells[2].source_text(), "print('edited')");
    // The untouched code cell's outputs survived the edit + round trip.
    assert_eq!(
        reparsed.cells[2].outputs,
        Some(vec![json!({"name": "stdout", "output_type": "stream", "text": ["hello\n"]})])
    );
}
