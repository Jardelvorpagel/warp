use super::{capabilities, resolve_window_target, validate_tab_create_target, ResolvedSelector};
use ::local_control::protocol::ActionKind;
use ::local_control::protocol::{
    PaneSelector, PaneTarget, TabSelector, TabTarget, TargetSelector, WindowSelector, WindowTarget,
};
use ::local_control::ErrorCode;

#[test]
fn tab_create_accepts_default_active_and_explicit_window_targets() {
    validate_tab_create_target(&TargetSelector::default()).expect("default target is accepted");

    validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Active),
        tab: Some(TabTarget::Active),
        pane: Some(PaneTarget::Active),
    })
    .expect("active target is accepted");

    validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Id {
            id: WindowSelector("1".to_owned()),
        }),
        tab: None,
        pane: None,
    })
    .expect("explicit window target is accepted");
}

#[test]
fn tab_create_rejects_unsupported_targets() {
    let err = validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Index { index: 1 }),
        tab: None,
        pane: None,
    })
    .expect_err("window index target is rejected");
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
fn active_window_selector_resolves_active_window() {
    let active = warpui::WindowId::from_usize(1);
    let open_windows = vec![active, warpui::WindowId::from_usize(2)];

    let resolved = resolve_window_target(None, &open_windows, Some(active)).expect("resolved");
    assert_eq!(resolved.window_id, active);
    assert_eq!(resolved.selector, ResolvedSelector::Active);
}

#[test]
fn active_window_selector_requires_active_open_window() {
    let open_windows = vec![warpui::WindowId::from_usize(2)];

    let err = resolve_window_target(Some(&WindowTarget::Active), &open_windows, None)
        .expect_err("missing");
    assert_eq!(err.code, ErrorCode::MissingTarget);
}

#[test]
fn explicit_window_selector_resolves_open_window() {
    let window = warpui::WindowId::from_usize(2);
    let open_windows = vec![warpui::WindowId::from_usize(1), window];

    let resolved = resolve_window_target(
        Some(&WindowTarget::Id {
            id: WindowSelector("2".to_owned()),
        }),
        &open_windows,
        None,
    )
    .expect("resolved");
    assert_eq!(resolved.window_id, window);
    assert_eq!(resolved.selector, ResolvedSelector::Id);
}

#[test]
fn explicit_stale_window_selector_returns_stale_target() {
    let open_windows = vec![warpui::WindowId::from_usize(1)];

    let err = resolve_window_target(
        Some(&WindowTarget::Id {
            id: WindowSelector("2".to_owned()),
        }),
        &open_windows,
        None,
    )
    .expect_err("stale");
    assert_eq!(err.code, ErrorCode::StaleTarget);
}

#[test]
fn malformed_window_selector_returns_invalid_selector() {
    let err = resolve_window_target(
        Some(&WindowTarget::Id {
            id: WindowSelector("window".to_owned()),
        }),
        &[],
        None,
    )
    .expect_err("invalid");
    assert_eq!(err.code, ErrorCode::InvalidSelector);
}
