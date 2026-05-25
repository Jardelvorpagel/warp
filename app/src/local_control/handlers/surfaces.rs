//! App surface and UI panel mutation handlers for local-control actions.
use ::local_control::protocol::{AppSurfaceParams, TargetSelector, WindowTarget};
use ::local_control::{ActionKind, ControlError, ErrorCode};
use serde_json::json;
use warpui::{ModelContext, TypedActionView, WindowId};

use crate::local_control::LocalControlBridge;
use crate::palette::PaletteMode;
use crate::server::telemetry::PaletteSource;
use crate::settings_view::SettingsSection;
use crate::workspace::{CommandSearchOptions, InitContent, Workspace, WorkspaceAction};

pub(crate) fn open_or_toggle_surface(
    action: ActionKind,
    target: &TargetSelector,
    params: AppSurfaceParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let window_id = select_window_for_surface_action(action, target, ctx)?;
    let workspace = workspace_for_window(action, window_id, ctx)?;
    let workspace_action = workspace_action_for_surface(action, params)?;
    workspace.update(ctx, |workspace, ctx| {
        workspace.handle_action(&workspace_action, ctx);
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "action": action.as_str(),
        "handled": true,
        "window_id": window_id.to_string(),
    }))
}

fn select_window_for_surface_action(
    action: ActionKind,
    target: &TargetSelector,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<WindowId, ControlError> {
    if target.tab.is_some() || target.pane.is_some() || target.session.is_some() {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} does not accept tab, pane, or session selectors",
                action.as_str()
            ),
        ));
    }
    match target.window.as_ref() {
        None | Some(WindowTarget::Active) => ctx.windows().active_window().ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                format!("{} requires an active Warp window", action.as_str()),
            )
        }),
        Some(WindowTarget::Id { id }) => ctx
            .window_ids()
            .find(|window_id| window_id.to_string() == id.0)
            .ok_or_else(|| {
                ControlError::new(
                    ErrorCode::StaleTarget,
                    format!("{} cannot resolve the requested window id", action.as_str()),
                )
            }),
        Some(WindowTarget::Index { .. } | WindowTarget::Title { .. }) => Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} only supports active and opaque window id selectors",
                action.as_str()
            ),
        )),
    }
}

fn workspace_for_window(
    action: ActionKind,
    window_id: WindowId,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<warpui::ViewHandle<Workspace>, ControlError> {
    ctx.views_of_type::<Workspace>(window_id)
        .and_then(|workspaces| workspaces.into_iter().next())
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                format!(
                    "{} requires a workspace in the target window",
                    action.as_str()
                ),
            )
        })
}

fn workspace_action_for_surface(
    action: ActionKind,
    params: AppSurfaceParams,
) -> Result<WorkspaceAction, ControlError> {
    match action {
        ActionKind::SurfaceSettingsOpen => settings_surface_action(params),
        ActionKind::SurfaceCommandPaletteOpen => command_palette_surface_action(params),
        ActionKind::SurfaceCommandSearchOpen => command_search_surface_action(params),
        ActionKind::SurfaceWarpDriveOpen => {
            no_params_surface_action(action, params, WorkspaceAction::OpenWarpDrive)
        }
        ActionKind::SurfaceWarpDriveToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleWarpDrive)
        }
        ActionKind::SurfaceResourceCenterToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleResourceCenter)
        }
        ActionKind::SurfaceAiAssistantToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleAIAssistant)
        }
        ActionKind::SurfaceCodeReviewToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleRightPanel)
        }
        ActionKind::SurfaceLeftPanelToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleLeftPanel)
        }
        ActionKind::SurfaceRightPanelToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleRightPanel)
        }
        ActionKind::SurfaceVerticalTabsToggle => {
            no_params_surface_action(action, params, WorkspaceAction::ToggleVerticalTabsPanel)
        }
        _ => Err(ControlError::new(
            ErrorCode::UnsupportedAction,
            format!("{} is not an app surface action", action.as_str()),
        )),
    }
}

fn settings_surface_action(params: AppSurfaceParams) -> Result<WorkspaceAction, ControlError> {
    let section = params
        .page
        .as_deref()
        .map(settings_section_from_param)
        .transpose()?;
    match (section, params.query) {
        (Some(section), Some(query)) => Ok(WorkspaceAction::ShowSettingsPageWithSearch {
            search_query: query,
            section: Some(section),
        }),
        (None, Some(query)) => Ok(WorkspaceAction::ShowSettingsPageWithSearch {
            search_query: query,
            section: None,
        }),
        (Some(section), None) => Ok(WorkspaceAction::ShowSettingsPage(section)),
        (None, None) => Ok(WorkspaceAction::ShowSettings),
    }
}

fn command_palette_surface_action(
    params: AppSurfaceParams,
) -> Result<WorkspaceAction, ControlError> {
    reject_surface_page(ActionKind::SurfaceCommandPaletteOpen, params.page)?;
    Ok(WorkspaceAction::OpenPalette {
        mode: PaletteMode::Command,
        source: PaletteSource::Keybinding,
        query: params.query,
    })
}

fn command_search_surface_action(
    params: AppSurfaceParams,
) -> Result<WorkspaceAction, ControlError> {
    reject_surface_page(ActionKind::SurfaceCommandSearchOpen, params.page)?;
    let init_content = params
        .query
        .map(InitContent::Custom)
        .unwrap_or(InitContent::FromInputBuffer);
    Ok(WorkspaceAction::ShowCommandSearch(CommandSearchOptions {
        filter: None,
        init_content,
    }))
}

fn no_params_surface_action(
    action: ActionKind,
    params: AppSurfaceParams,
    workspace_action: WorkspaceAction,
) -> Result<WorkspaceAction, ControlError> {
    if params.query.is_some() || params.page.is_some() {
        return Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!(
                "{} does not accept query or page parameters",
                action.as_str()
            ),
        ));
    }
    Ok(workspace_action)
}

fn reject_surface_page(action: ActionKind, page: Option<String>) -> Result<(), ControlError> {
    if page.is_some() {
        return Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!("{} does not accept a page parameter", action.as_str()),
        ));
    }
    Ok(())
}

fn settings_section_from_param(page: &str) -> Result<SettingsSection, ControlError> {
    let normalized = page.replace(['-', '_'], " ");
    let mut words = normalized.split_whitespace();
    let title_case = words.try_fold(String::new(), |mut output, word| {
        if !output.is_empty() {
            output.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            output.extend(first.to_uppercase());
            output.push_str(&chars.as_str().to_lowercase());
        }
        Some(output)
    });
    let mut candidates = vec![page.to_owned(), normalized];
    if let Some(title_case) = title_case {
        candidates.push(title_case);
    }
    candidates
        .iter()
        .find_map(|candidate| <SettingsSection as std::str::FromStr>::from_str(candidate).ok())
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::InvalidParams,
                format!("unknown settings page {page}"),
            )
        })
}
