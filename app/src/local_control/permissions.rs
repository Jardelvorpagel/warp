//! Permission checks that map protocol action metadata onto local settings.
use crate::ai::execution_profiles::{AIExecutionProfile, WarpControlPermission};
use crate::features::FeatureFlag;
use crate::settings::{LocalControlPermissionCategory, LocalControlSettings};
use ::local_control::{ActionKind, ControlError, ErrorCode, InvocationContext, PermissionCategory};
use warpui::{ModelContext, SingletonEntity};

use crate::local_control::LocalControlBridge;

pub(super) fn warp_control_cli_enabled() -> bool {
    FeatureFlag::WarpControlCli.is_enabled()
}

pub(super) fn ensure_feature_enabled() -> Result<(), ControlError> {
    if warp_control_cli_enabled() {
        return Ok(());
    }
    Err(ControlError::new(
        ErrorCode::LocalControlDisabled,
        "Warp control CLI is disabled by feature flag",
    ))
}

#[cfg(test)]
pub(crate) fn outside_warp_action_enabled_for_settings(
    settings: &LocalControlSettings,
    action: ActionKind,
) -> bool {
    outside_warp_permission_enabled_for_settings(settings, action.metadata().permission_category)
}

#[cfg(test)]
fn outside_warp_permission_enabled_for_settings(
    settings: &LocalControlSettings,
    permission: PermissionCategory,
) -> bool {
    settings.allows_outside_warp(local_permission(permission))
}

#[cfg(test)]
pub(crate) fn capabilities() -> Vec<ActionKind> {
    ActionKind::implemented_metadata()
        .into_iter()
        .map(|metadata| metadata.kind)
        .collect()
}

fn local_permission(permission: PermissionCategory) -> LocalControlPermissionCategory {
    match permission {
        PermissionCategory::ReadMetadata => LocalControlPermissionCategory::MetadataReads,
        PermissionCategory::ReadUnderlyingData => {
            LocalControlPermissionCategory::UnderlyingDataReads
        }
        PermissionCategory::MutateAppState => LocalControlPermissionCategory::AppStateMutations,
        PermissionCategory::MutateMetadataConfiguration => {
            LocalControlPermissionCategory::MetadataConfigurationMutations
        }
        PermissionCategory::MutateUnderlyingData => {
            LocalControlPermissionCategory::UnderlyingDataMutations
        }
    }
}

#[allow(dead_code)]
pub(crate) fn agent_profile_permission_for_action(
    profile: &AIExecutionProfile,
    action: ActionKind,
) -> WarpControlPermission {
    agent_profile_permission_for_category(profile, action.metadata().permission_category)
}

#[allow(dead_code)]
pub(crate) fn agent_profile_permission_for_category(
    profile: &AIExecutionProfile,
    permission: PermissionCategory,
) -> WarpControlPermission {
    profile.warp_control_permission_for_category(local_permission(permission))
}

#[allow(dead_code)]
pub(crate) fn ensure_agent_profile_allows_action(
    profile: &AIExecutionProfile,
    action: ActionKind,
) -> Result<(), ControlError> {
    let permission = agent_profile_permission_for_action(profile, action);
    if permission.is_denied() {
        return Err(ControlError::new(
            ErrorCode::InsufficientPermissions,
            format!(
                "agent profile denies the Warp control permission required by {}",
                action.as_str()
            ),
        ));
    }
    Ok(())
}

pub(super) fn ensure_action_allowed(
    context: InvocationContext,
    action: ActionKind,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<(), ControlError> {
    let settings = LocalControlSettings::as_ref(ctx);
    ensure_settings_allow_action(settings, context, action)
}

pub(crate) fn ensure_settings_allow_action(
    settings: &LocalControlSettings,
    context: InvocationContext,
    action: ActionKind,
) -> Result<(), ControlError> {
    if context == InvocationContext::InsideWarp {
        return Err(ControlError::new(
            ErrorCode::ExecutionContextNotAllowed,
            "inside-Warp local-control grants are not implemented",
        ));
    }
    if !settings.outside_warp_control_enabled() {
        return Err(ControlError::new(
            ErrorCode::LocalControlDisabled,
            "local control is disabled for this invocation context",
        ));
    }
    let permission = local_permission(action.metadata().permission_category);
    if !settings.outside_warp_permission_enabled(permission) {
        return Err(ControlError::new(
            ErrorCode::InsufficientPermissions,
            format!(
                "{} requires a local-control permission that is disabled",
                action.as_str()
            ),
        ));
    }
    Ok(())
}
