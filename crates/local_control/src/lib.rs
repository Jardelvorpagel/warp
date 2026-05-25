//! Shared protocol, discovery, authentication, and client types for local Warp control.
//!
//! The `local_control` crate is intentionally UI-agnostic so the Warp app and
//! `warpctrl` CLI can share the same wire envelopes, action catalog, discovery
//! records, selectors, and credential validation rules.
pub mod auth;
pub mod catalog;
pub mod client;
pub mod discovery;
pub mod protocol;
pub mod selection;
pub mod selectors;

pub use auth::{
    AuthToken, AuthenticatedUserGrant, CredentialGrant, CredentialRequest, ScopedCredential,
};
pub use catalog::{
    ActionImplementationStatus, ActionKind, ActionMetadata, AuthenticatedUserRequirement,
    InvocationContext, PermissionCategory, RiskTier, StateDataCategory, TargetScope,
};
pub use discovery::{
    ControlEndpoint, CredentialBrokerReference, InstanceId, InstanceRecord, RegisteredInstance,
    discovery_dir,
};
pub use protocol::{
    Action, ActionGetParams, ActionGetResult, ActionListParams, ActionListResult,
    ActiveTargetChain, AppActiveParams, AppFocusParams, AppInspectParams, AppInspectResult,
    AppSurfaceParams, AppVersionResult, AppearanceFontSizeParams, AppearanceMutationResult,
    AppearanceSetParams, AppearanceStateResult, AppearanceZoomParams, BlockGetParams,
    BlockGetResult, BlockListParams, BlockListResult, BlockSummary, ControlError, ControlResponse,
    DriveCreateParams, DriveDeleteParams, DriveGetParams, DriveGetResult, DriveInsertParams,
    DriveListParams, DriveListResult, DriveMutationResult, DriveObjectSummary, DriveRunParams,
    DriveUpdateParams, EmptyParams, ErrorCode, ErrorResponseEnvelope, ExecutionContextProof,
    FileDeleteParams, FileListParams, FileListResult, FileMutationResult, FileOpenParams,
    FileSummary, FileWriteParams, HistoryEntrySummary, HistoryListParams, HistoryListResult,
    HorizontalDirection, InputClearParams, InputGetParams, InputInsertParams, InputMode,
    InputModeSetParams, InputReplaceParams, InputRunParams, InputStateResult, PROTOCOL_VERSION,
    PaneCloseParams, PaneDirection, PaneFocusParams, PaneListResult, PaneMaximizeParams,
    PaneMutationResult, PaneNavigateParams, PaneResizeParams, PaneSplitParams, PaneSummary,
    ProjectActiveParams, ProjectActiveResult, ProjectListParams, ProjectListResult, ProjectSummary,
    RequestEnvelope, ResponseEnvelope, SessionListResult, SessionMutationResult, SessionSummary,
    SettingGetParams, SettingGetResult, SettingListParams, SettingListResult,
    SettingMutationResult, SettingSetParams, SettingSummary, SettingToggleParams, SizeAdjustment,
    TabActivateParams, TabActivationTarget, TabCloseParams, TabCloseScope, TabCreateParams,
    TabListResult, TabMoveParams, TabMutationResult, TabRenameParams, TabSummary, ThemeListResult,
    ThemeSetParams, ThemeSummary, WindowCloseParams, WindowCreateParams, WindowFocusParams,
    WindowListResult, WindowMutationResult, WindowSummary,
};
pub use selectors::{
    BlockSelector, BlockTarget, DriveObjectSelector, DriveObjectType, DriveTarget, FileSelector,
    FileTarget, PaneSelector, SessionSelector, TabSelector, TargetSelector, WindowSelector,
};
