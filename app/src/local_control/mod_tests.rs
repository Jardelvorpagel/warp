use ::local_control::auth::CredentialRequest;
use ::local_control::protocol::{
    ActionKind, ExecutionContextProof, InvocationContext, PaneSelector, PaneTarget, TabSelector,
    TabTarget, TargetSelector, WindowSelector, WindowTarget,
};
use ::local_control::ErrorCode;
use settings::Setting;

use crate::settings::{
    AllowInsideWarpAppStateMutations, AllowInsideWarpControl,
    AllowInsideWarpMetadataConfigurationMutations, AllowInsideWarpMetadataReads,
    AllowInsideWarpUnderlyingDataMutations, AllowInsideWarpUnderlyingDataReads,
    AllowOutsideWarpAppStateMutations, AllowOutsideWarpControl,
    AllowOutsideWarpMetadataConfigurationMutations, AllowOutsideWarpMetadataReads,
    AllowOutsideWarpUnderlyingDataMutations, AllowOutsideWarpUnderlyingDataReads,
    LocalControlSettings,
};

use super::{
    capabilities, ensure_action_allowed_by_settings, preferred_window_id,
    validate_tab_create_target, verify_invocation_context_proof,
};

#[test]
fn tab_create_accepts_default_and_active_targets() {
    validate_tab_create_target(&TargetSelector::default()).expect("default target is accepted");

    validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Active),
        tab: Some(TabTarget::Active),
        pane: Some(PaneTarget::Active),
    })
    .expect("active target is accepted");
}

#[test]
fn tab_create_rejects_concrete_targets() {
    let err = validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Id {
            id: WindowSelector("window".to_owned()),
        }),
        tab: None,
        pane: None,
    })
    .expect_err("concrete window target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);

    let err = validate_tab_create_target(&TargetSelector {
        window: None,
        tab: Some(TabTarget::Id {
            id: TabSelector("tab".to_owned()),
        }),
        pane: None,
    })
    .expect_err("concrete tab target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);

    let err = validate_tab_create_target(&TargetSelector {
        window: None,
        tab: None,
        pane: Some(PaneTarget::Id {
            id: PaneSelector("pane".to_owned()),
        }),
    })
    .expect_err("concrete pane target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);
}

#[test]
fn capabilities_only_advertises_tab_create() {
    assert_eq!(capabilities(), vec![ActionKind::TabCreate]);
}

#[test]
fn tab_create_prefers_active_window() {
    let active = warpui::WindowId::from_usize(1);

    assert_eq!(preferred_window_id(Some(active)), Some(active));
}

#[test]
fn tab_create_rejects_missing_active_window() {
    assert_eq!(preferred_window_id(None), None);
}

#[test]
fn disabled_outside_context_rejects_credential_issuance_preflight() {
    let settings = settings_with_outside_app_state(false, true);
    let err = ensure_action_allowed_by_settings(
        &settings,
        InvocationContext::OutsideWarp,
        ActionKind::TabCreate,
    )
    .expect_err("outside context is disabled");

    assert_eq!(err.code, ErrorCode::LocalControlDisabled);
}

#[test]
fn enabled_context_requires_app_state_mutation_permission() {
    let settings = settings_with_outside_app_state(true, false);
    let err = ensure_action_allowed_by_settings(
        &settings,
        InvocationContext::OutsideWarp,
        ActionKind::TabCreate,
    )
    .expect_err("app-state mutation permission is disabled");

    assert_eq!(err.code, ErrorCode::InsufficientPermissions);
}

#[test]
fn enabled_context_with_app_state_mutation_permission_allows_tab_create() {
    let settings = settings_with_outside_app_state(true, true);

    ensure_action_allowed_by_settings(
        &settings,
        InvocationContext::OutsideWarp,
        ActionKind::TabCreate,
    )
    .expect("tab.create is allowed");
}

#[test]
fn outside_context_does_not_require_execution_context_proof() {
    let request = CredentialRequest::new(ActionKind::TabCreate, InvocationContext::OutsideWarp);

    verify_invocation_context_proof(&request).expect("outside request is accepted");
}

#[test]
fn inside_context_requires_verified_warp_terminal_proof() {
    let request = CredentialRequest::new(ActionKind::TabCreate, InvocationContext::InsideWarp);
    let err = verify_invocation_context_proof(&request).expect_err("missing proof is rejected");
    assert_eq!(err.code, ErrorCode::ExecutionContextNotAllowed);

    let mut request = CredentialRequest::new(ActionKind::TabCreate, InvocationContext::InsideWarp);
    request.execution_context_proof = Some(ExecutionContextProof::ExternalClient);
    let err = verify_invocation_context_proof(&request).expect_err("external proof is rejected");
    assert_eq!(err.code, ErrorCode::ExecutionContextNotAllowed);

    let mut request = CredentialRequest::new(ActionKind::TabCreate, InvocationContext::InsideWarp);
    request.execution_context_proof = Some(ExecutionContextProof::VerifiedWarpTerminal {
        proof_id: "unverified".to_owned(),
    });
    let err = verify_invocation_context_proof(&request).expect_err("unverified proof is rejected");
    assert_eq!(err.code, ErrorCode::ExecutionContextNotAllowed);
}

fn settings_with_outside_app_state(
    outside_enabled: bool,
    outside_app_state_enabled: bool,
) -> LocalControlSettings {
    LocalControlSettings {
        allow_inside_warp_control: AllowInsideWarpControl::new(Some(true)),
        allow_outside_warp_control: AllowOutsideWarpControl::new(Some(outside_enabled)),
        allow_inside_warp_metadata_reads: AllowInsideWarpMetadataReads::new(Some(true)),
        allow_outside_warp_metadata_reads: AllowOutsideWarpMetadataReads::new(Some(false)),
        allow_inside_warp_underlying_data_reads: AllowInsideWarpUnderlyingDataReads::new(Some(
            true,
        )),
        allow_outside_warp_underlying_data_reads: AllowOutsideWarpUnderlyingDataReads::new(Some(
            false,
        )),
        allow_inside_warp_app_state_mutations: AllowInsideWarpAppStateMutations::new(Some(true)),
        allow_outside_warp_app_state_mutations: AllowOutsideWarpAppStateMutations::new(Some(
            outside_app_state_enabled,
        )),
        allow_inside_warp_metadata_configuration_mutations:
            AllowInsideWarpMetadataConfigurationMutations::new(Some(true)),
        allow_outside_warp_metadata_configuration_mutations:
            AllowOutsideWarpMetadataConfigurationMutations::new(Some(false)),
        allow_inside_warp_underlying_data_mutations: AllowInsideWarpUnderlyingDataMutations::new(
            Some(true),
        ),
        allow_outside_warp_underlying_data_mutations: AllowOutsideWarpUnderlyingDataMutations::new(
            Some(false),
        ),
    }
}
