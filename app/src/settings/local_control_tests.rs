use settings::{Setting, SyncToCloud};

use super::{
    AllowInsideWarpAppStateMutations, AllowInsideWarpControl,
    AllowInsideWarpMetadataConfigurationMutations, AllowInsideWarpMetadataReads,
    AllowInsideWarpUnderlyingDataMutations, AllowInsideWarpUnderlyingDataReads,
    AllowOutsideWarpAppStateMutations, AllowOutsideWarpControl,
    AllowOutsideWarpMetadataConfigurationMutations, AllowOutsideWarpMetadataReads,
    AllowOutsideWarpUnderlyingDataMutations, AllowOutsideWarpUnderlyingDataReads,
    LocalControlInvocationContext, LocalControlPermissionCategory, LocalControlSettings,
};

fn settings_with_values(
    inside_enabled: bool,
    outside_enabled: bool,
    inside_permission_enabled: bool,
    outside_permission_enabled: bool,
) -> LocalControlSettings {
    LocalControlSettings {
        allow_inside_warp_control: AllowInsideWarpControl::new(Some(inside_enabled)),
        allow_outside_warp_control: AllowOutsideWarpControl::new(Some(outside_enabled)),
        allow_inside_warp_metadata_reads: AllowInsideWarpMetadataReads::new(Some(
            inside_permission_enabled,
        )),
        allow_outside_warp_metadata_reads: AllowOutsideWarpMetadataReads::new(Some(
            outside_permission_enabled,
        )),
        allow_inside_warp_underlying_data_reads: AllowInsideWarpUnderlyingDataReads::new(Some(
            inside_permission_enabled,
        )),
        allow_outside_warp_underlying_data_reads: AllowOutsideWarpUnderlyingDataReads::new(Some(
            outside_permission_enabled,
        )),
        allow_inside_warp_app_state_mutations: AllowInsideWarpAppStateMutations::new(Some(
            inside_permission_enabled,
        )),
        allow_outside_warp_app_state_mutations: AllowOutsideWarpAppStateMutations::new(Some(
            outside_permission_enabled,
        )),
        allow_inside_warp_metadata_configuration_mutations:
            AllowInsideWarpMetadataConfigurationMutations::new(Some(inside_permission_enabled)),
        allow_outside_warp_metadata_configuration_mutations:
            AllowOutsideWarpMetadataConfigurationMutations::new(Some(outside_permission_enabled)),
        allow_inside_warp_underlying_data_mutations: AllowInsideWarpUnderlyingDataMutations::new(
            Some(inside_permission_enabled),
        ),
        allow_outside_warp_underlying_data_mutations: AllowOutsideWarpUnderlyingDataMutations::new(
            Some(outside_permission_enabled),
        ),
    }
}

#[test]
fn defaults_allow_inside_warp_permissions_only() {
    let settings = settings_with_values(true, false, true, false);

    for permission in [
        LocalControlPermissionCategory::MetadataReads,
        LocalControlPermissionCategory::UnderlyingDataReads,
        LocalControlPermissionCategory::AppStateMutations,
        LocalControlPermissionCategory::MetadataConfigurationMutations,
        LocalControlPermissionCategory::UnderlyingDataMutations,
    ] {
        assert!(settings.allows(LocalControlInvocationContext::InsideWarp, permission));
        assert!(!settings.allows(LocalControlInvocationContext::OutsideWarp, permission));
    }
}

#[test]
fn generated_settings_are_private_local_only_with_expected_defaults() {
    assert!(*AllowInsideWarpControl::new(None));
    assert!(!*AllowOutsideWarpControl::new(None));
    assert!(*AllowInsideWarpMetadataReads::new(None));
    assert!(!*AllowOutsideWarpMetadataReads::new(None));
    assert!(*AllowInsideWarpUnderlyingDataReads::new(None));
    assert!(!*AllowOutsideWarpUnderlyingDataReads::new(None));
    assert!(*AllowInsideWarpAppStateMutations::new(None));
    assert!(!*AllowOutsideWarpAppStateMutations::new(None));
    assert!(*AllowInsideWarpMetadataConfigurationMutations::new(None));
    assert!(!*AllowOutsideWarpMetadataConfigurationMutations::new(None));
    assert!(*AllowInsideWarpUnderlyingDataMutations::new(None));
    assert!(!*AllowOutsideWarpUnderlyingDataMutations::new(None));
    assert_eq!(AllowInsideWarpControl::sync_to_cloud(), SyncToCloud::Never);
    assert_eq!(AllowOutsideWarpControl::sync_to_cloud(), SyncToCloud::Never);
    assert_eq!(
        AllowInsideWarpMetadataReads::sync_to_cloud(),
        SyncToCloud::Never
    );
    assert_eq!(
        AllowOutsideWarpUnderlyingDataMutations::sync_to_cloud(),
        SyncToCloud::Never
    );
    assert!(AllowInsideWarpControl::is_private());
    assert!(AllowOutsideWarpControl::is_private());
    assert!(AllowInsideWarpMetadataReads::is_private());
    assert!(AllowOutsideWarpUnderlyingDataMutations::is_private());
}

#[test]
fn disabled_context_blocks_enabled_granular_permissions() {
    let settings = settings_with_values(false, false, true, true);

    for permission in [
        LocalControlPermissionCategory::MetadataReads,
        LocalControlPermissionCategory::UnderlyingDataReads,
        LocalControlPermissionCategory::AppStateMutations,
        LocalControlPermissionCategory::MetadataConfigurationMutations,
        LocalControlPermissionCategory::UnderlyingDataMutations,
    ] {
        assert!(!settings.allows(LocalControlInvocationContext::InsideWarp, permission));
        assert!(!settings.allows(LocalControlInvocationContext::OutsideWarp, permission));
    }
}
