//! A **lossless, round-trippable** model of an `.ipynb` (Jupyter) notebook.
//!
//! Unlike a render-only converter, this model is the source of truth for an
//! *editable* notebook: it preserves every field it does not itself model
//! (cell ids, per-cell and notebook-level metadata, `execution_count`, all
//! outputs — including output types the UI does not render — and any unknown
//! fields) so that load → edit → save changes only what the user actually
//! edited.
//!
//! Only nbformat v4 is supported. Anything that fails to parse as a v4 notebook
//! returns an [`IpynbError`] so callers can fall back to showing the raw file
//! contents instead of a blank view.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The only nbformat major version this model understands.
const SUPPORTED_NBFORMAT: i64 = 4;

/// Error produced when the input cannot be parsed as a supported notebook.
#[derive(Debug, thiserror::Error)]
pub enum IpynbError {
    /// The input was not valid notebook JSON.
    #[error("failed to parse notebook JSON: {0}")]
    Parse(#[from] serde_json::Error),
    /// The notebook used an unsupported nbformat version (only v4 is supported).
    #[error("unsupported notebook format: nbformat={nbformat:?} (only v{SUPPORTED_NBFORMAT} is supported)")]
    UnsupportedFormat { nbformat: Option<i64> },
}

/// The cell types this model renders/edits as first-class kinds. Unknown cell
/// types are preserved verbatim but report `None` from [`CellDoc::kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Markdown,
    Code,
    Raw,
}

impl CellKind {
    /// The nbformat `cell_type` string for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            CellKind::Markdown => "markdown",
            CellKind::Code => "code",
            CellKind::Raw => "raw",
        }
    }

    /// Parse an nbformat `cell_type` string into a known kind, if recognized.
    pub fn from_type_str(s: &str) -> Option<Self> {
        match s {
            "markdown" => Some(CellKind::Markdown),
            "code" => Some(CellKind::Code),
            "raw" => Some(CellKind::Raw),
            _ => None,
        }
    }
}

/// Top-level notebook. Modeled fields (`nbformat`, `cells`) are the ones we read
/// or edit; everything else (`metadata`, `nbformat_minor`, unknown keys) is
/// preserved verbatim in `extra` so it round-trips untouched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookDoc {
    /// nbformat major version. Required to be `Some(4)` by [`NotebookDoc::parse`].
    #[serde(default)]
    pub nbformat: Option<i64>,
    #[serde(default)]
    pub cells: Vec<CellDoc>,
    /// All other top-level fields (`metadata`, `nbformat_minor`, unknowns),
    /// preserved verbatim.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl NotebookDoc {
    /// Parse the JSON contents of a `.ipynb` file into the model.
    ///
    /// Returns an [`IpynbError`] if the input is not a parseable nbformat v4
    /// notebook; callers should fall back to displaying the raw contents in that
    /// case (never a blank view).
    pub fn parse(json: &str) -> Result<Self, IpynbError> {
        let doc: NotebookDoc = serde_json::from_str(json)?;
        // Guard against arbitrary JSON that happens to deserialize into an empty
        // notebook: require an explicit, supported nbformat version.
        if doc.nbformat != Some(SUPPORTED_NBFORMAT) {
            return Err(IpynbError::UnsupportedFormat {
                nbformat: doc.nbformat,
            });
        }
        Ok(doc)
    }

    /// Serialize the model back to notebook JSON.
    ///
    /// Uses a 1-space indent and a trailing newline to match the convention the
    /// reference `nbformat` writer uses, which keeps on-disk diffs small.
    pub fn to_json_pretty(&self) -> String {
        let mut buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        self.serialize(&mut ser)
            .expect("NotebookDoc always serializes to JSON");
        let mut out = String::from_utf8(buf).expect("serde_json emits valid UTF-8");
        out.push('\n');
        out
    }

    /// The code-fence language for syntax highlighting, derived from notebook
    /// metadata (`language_info.name`, falling back to `kernelspec.language`).
    pub fn language(&self) -> Option<String> {
        let metadata = self.extra.get("metadata")?;
        metadata
            .get("language_info")
            .and_then(|info| info.get("name"))
            .and_then(Value::as_str)
            .or_else(|| {
                metadata
                    .get("kernelspec")
                    .and_then(|spec| spec.get("language"))
                    .and_then(Value::as_str)
            })
            .map(str::to_string)
    }

    /// Insert a cell at `index`, clamped to the end of the notebook.
    pub fn insert_cell(&mut self, index: usize, cell: CellDoc) {
        let index = index.min(self.cells.len());
        self.cells.insert(index, cell);
    }

    /// Remove and return the cell at `index`, if it exists.
    pub fn remove_cell(&mut self, index: usize) -> Option<CellDoc> {
        if index < self.cells.len() {
            Some(self.cells.remove(index))
        } else {
            None
        }
    }

    /// Move the cell at `from` to `to`. Returns `false` (and does nothing) if
    /// either index is out of bounds.
    pub fn move_cell(&mut self, from: usize, to: usize) -> bool {
        if from >= self.cells.len() || to >= self.cells.len() {
            return false;
        }
        let cell = self.cells.remove(from);
        self.cells.insert(to, cell);
        true
    }
}

/// A single notebook cell. `cell_type`, `source`, and `outputs` are modeled
/// because the editor reads or mutates them; every other field (`id`,
/// `metadata`, `execution_count`, `attachments`, unknowns) is preserved verbatim
/// in `extra`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellDoc {
    pub cell_type: String,
    #[serde(default)]
    pub source: Source,
    /// Saved outputs (code cells only). Stored as opaque JSON and never mutated
    /// by the editor, so output types the UI does not render still round-trip.
    /// `None` means the cell had no `outputs` key (e.g. markdown/raw cells);
    /// `Some(vec![])` preserves a code cell's empty `outputs` array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<Value>>,
    /// All other cell fields (`id`, `metadata`, `execution_count`, ...),
    /// preserved verbatim.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl CellDoc {
    /// Create a new, empty markdown cell with empty metadata.
    pub fn new_markdown(source: &str) -> Self {
        let mut extra = Map::new();
        extra.insert("metadata".to_string(), Value::Object(Map::new()));
        Self {
            cell_type: CellKind::Markdown.as_str().to_string(),
            source: Source::from_text(source),
            outputs: None,
            extra,
        }
    }

    /// Create a new, empty code cell with empty metadata, a null execution
    /// count, and an empty outputs array (per nbformat code-cell shape).
    pub fn new_code(source: &str) -> Self {
        let mut extra = Map::new();
        extra.insert("metadata".to_string(), Value::Object(Map::new()));
        extra.insert("execution_count".to_string(), Value::Null);
        Self {
            cell_type: CellKind::Code.as_str().to_string(),
            source: Source::from_text(source),
            outputs: Some(Vec::new()),
            extra,
        }
    }

    /// The known kind of this cell, or `None` for unmodeled `cell_type`s.
    pub fn kind(&self) -> Option<CellKind> {
        CellKind::from_type_str(&self.cell_type)
    }

    /// The cell's source as a single string.
    pub fn source_text(&self) -> String {
        self.source.to_text()
    }

    /// Replace the cell's source. The new source is stored in nbformat list form
    /// (one entry per line, trailing newline retained on all but the last line).
    pub fn set_source(&mut self, text: &str) {
        self.source = Source::from_text(text);
    }

    /// Convert this cell to another kind. Converting away from `Code` drops the
    /// outputs and execution count (which belong only to code cells); converting
    /// to `Code` gives it an empty outputs array and a null execution count.
    pub fn convert_to(&mut self, kind: CellKind) {
        self.cell_type = kind.as_str().to_string();
        match kind {
            CellKind::Code => {
                if self.outputs.is_none() {
                    self.outputs = Some(Vec::new());
                }
                self.extra
                    .entry("execution_count".to_string())
                    .or_insert(Value::Null);
            }
            CellKind::Markdown | CellKind::Raw => {
                self.outputs = None;
                self.extra.remove("execution_count");
            }
        }
    }
}

/// A notebook `source`/`text` field, which nbformat allows to be either a single
/// string or a list of strings (each line typically retaining its trailing
/// newline). The original form is preserved for untouched cells; edited cells
/// are re-stored in list form via [`Source::from_text`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Source {
    Lines(Vec<String>),
    Text(String),
}

impl Default for Source {
    fn default() -> Self {
        Source::Text(String::new())
    }
}

impl Source {
    /// Collapse the source into a single string.
    pub fn to_text(&self) -> String {
        match self {
            Source::Lines(lines) => lines.concat(),
            Source::Text(text) => text.clone(),
        }
    }

    /// Build a list-form source from text, splitting on newlines while retaining
    /// each line's trailing `\n` (matching nbformat's on-disk convention). Empty
    /// text yields an empty list.
    pub fn from_text(text: &str) -> Self {
        if text.is_empty() {
            return Source::Lines(Vec::new());
        }
        Source::Lines(text.split_inclusive('\n').map(str::to_string).collect())
    }
}

#[cfg(test)]
#[path = "ipynb_model_tests.rs"]
mod tests;
