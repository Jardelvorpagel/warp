//! Serializable selectors for targeting windows, tabs, and panes.
use serde::{Deserialize, Serialize};

/// Opaque window identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowSelector(pub String);

/// Opaque tab identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TabSelector(pub String);

/// Opaque pane identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneSelector(pub String);

/// Opaque session identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionSelector(pub String);
/// Hierarchical target for actions that operate on a specific Warp surface.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TargetSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<TabTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<PaneTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<BlockTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<FileTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive: Option<DriveTarget>,
}

/// Window-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowTarget {
    Active,
    Id { id: WindowSelector },
    Index { index: u32 },
    Title { title: String },
}

/// Tab-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabTarget {
    Active,
    Id { id: TabSelector },
    Index { index: u32 },
    Title { title: String },
}

/// Pane-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PaneTarget {
    Active,
    Id { id: PaneSelector },
    Index { index: u32 },
}

/// Session-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionTarget {
    Active,
    Id { id: SessionSelector },
}

/// Opaque block identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockSelector(pub String);

/// Block-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockTarget {
    Active,
    Id { id: BlockSelector },
    Index { index: u32 },
}

/// Opaque file identifier supplied by Warp file surface metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileSelector(pub String);

/// File-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileTarget {
    Path { path: String },
    Id { id: FileSelector },
}

/// Category of a Warp Drive object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveObjectType {
    Workflow,
    Notebook,
    Environment,
    Prompt,
}

/// Opaque Drive object identifier supplied by Drive list metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DriveObjectSelector(pub String);

/// Drive-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriveTarget {
    Id {
        object_type: DriveObjectType,
        id: DriveObjectSelector,
    },
    Name {
        object_type: DriveObjectType,
        name: String,
    },
}
