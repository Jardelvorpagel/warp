use ::local_control::protocol::{
    AppearanceMutationResult, AppearanceStateResult, SettingGetParams, SettingGetResult,
    SettingListResult, SettingMutationResult, SettingSetParams, SettingSummary,
    SettingToggleParams, ThemeListResult, ThemeSetParams, ThemeSummary,
};
use ::local_control::{ActionKind, ControlError, ErrorCode};
use serde::Serialize;
use serde_json::{json, Value};
use settings::Setting as _;
use warpui::accessibility::AccessibilityVerbosity;
use warpui::{ModelContext, SingletonEntity};

use crate::local_control::LocalControlBridge;
use crate::settings::{
    derived_theme_kind, AccessibilitySettings, FontSettings, InputSettings, ThemeSettings,
};
use crate::themes::theme::{SelectedSystemThemes, ThemeKind};
use crate::user_config::WarpConfig;
use crate::window_settings::ZoomLevel;
use crate::WindowSettings;

const ALLOWLISTED_SETTING_KEYS: &[&str] = &[
    "accessibility.accessibility_verbosity",
    "appearance.text.font_name",
    "appearance.text.font_size",
    "appearance.themes.dark_theme",
    "appearance.themes.light_theme",
    "appearance.themes.system_theme",
    "appearance.themes.theme",
    "appearance.window.zoom_level",
    "terminal.input.error_underlining_enabled",
    "terminal.input.syntax_highlighting",
];

const PRIVATE_OR_SENSITIVE_SETTING_KEYS: &[&str] = &[
    "local_control.allow_inside_warp_control",
    "local_control.allow_inside_warp_metadata_reads",
    "local_control.allow_inside_warp_underlying_data_reads",
    "local_control.allow_inside_warp_app_state_mutations",
    "local_control.allow_inside_warp_metadata_configuration_mutations",
    "local_control.allow_inside_warp_underlying_data_mutations",
    "local_control.allow_outside_warp_control",
    "local_control.allow_outside_warp_metadata_reads",
    "local_control.allow_outside_warp_underlying_data_reads",
    "local_control.allow_outside_warp_app_state_mutations",
    "local_control.allow_outside_warp_metadata_configuration_mutations",
    "local_control.allow_outside_warp_underlying_data_mutations",
    "terminal.input.autosuggestion_accepted_count",
    "terminal.input.completions_menu_height",
    "terminal.input.completions_menu_width",
    "terminal.input.inline_menu_custom_content_heights",
    "terminal.input.workflows_box_expanded",
];

pub(crate) fn theme_list(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    to_control_data(theme_list_result(ctx)?)
}

pub(crate) fn appearance_get(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    to_control_data(appearance_state_result(ctx)?)
}

pub(crate) fn setting_list(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    to_control_data(setting_list_result(ctx)?)
}

pub(crate) fn setting_get(
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = action.params_as::<SettingGetParams>()?;
    to_control_data(setting_get_result(&params.key, ctx)?)
}

pub(crate) fn theme_set(
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = action.params_as::<ThemeSetParams>()?;
    to_control_data(theme_set_result(params, ctx)?)
}

pub(crate) fn setting_set(
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = action.params_as::<SettingSetParams>()?;
    to_control_data(setting_set_result(params, ctx)?)
}

pub(crate) fn setting_toggle(
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = action.params_as::<SettingToggleParams>()?;
    to_control_data(setting_toggle_result(params, ctx)?)
}

pub(crate) fn theme_list_result(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<ThemeListResult, ControlError> {
    let current_theme = active_theme_kind(ThemeSettings::as_ref(ctx), ctx);
    let mut themes = WarpConfig::as_ref(ctx)
        .theme_config()
        .theme_items()
        .map(|(kind, _)| ThemeSummary {
            name: public_theme_name(kind),
            is_current: *kind == current_theme,
        })
        .collect::<Vec<_>>();
    themes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ThemeListResult { themes })
}

pub(crate) fn appearance_state_result(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<AppearanceStateResult, ControlError> {
    let theme_settings = ThemeSettings::as_ref(ctx);
    let font_settings = FontSettings::as_ref(ctx);
    let window_settings = WindowSettings::as_ref(ctx);
    let system_themes = theme_settings.selected_system_themes.value();
    Ok(AppearanceStateResult {
        theme: Some(public_theme_name(theme_settings.theme_kind.value())),
        follow_system_theme: *theme_settings.use_system_theme.value(),
        light_theme: Some(public_theme_name(&system_themes.light)),
        dark_theme: Some(public_theme_name(&system_themes.dark)),
        font_size: rounded_u32(*font_settings.monospace_font_size.value()),
        ui_zoom_percent: Some(u32::from(*window_settings.zoom_level.value())),
    })
}

pub(crate) fn setting_list_result(
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SettingListResult, ControlError> {
    let settings = ALLOWLISTED_SETTING_KEYS
        .iter()
        .map(|key| setting_summary_for_key(key, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SettingListResult { settings })
}

pub(crate) fn setting_get_result(
    key: &str,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SettingGetResult, ControlError> {
    Ok(SettingGetResult {
        setting: setting_summary_for_key(key, ctx)?,
    })
}

pub(crate) fn theme_set_result(
    params: ThemeSetParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<AppearanceMutationResult, ControlError> {
    let theme = theme_kind_for_name(&params.name, ctx)?;
    let changed = ThemeSettings::handle(ctx)
        .update(ctx, |theme_settings, ctx| {
            let changed = *theme_settings.use_system_theme.value()
                || *theme_settings.theme_kind.value() != theme;
            theme_settings.use_system_theme.set_value(false, ctx)?;
            theme_settings.theme_kind.set_value(theme, ctx)?;
            Ok::<_, anyhow::Error>(changed)
        })
        .map_err(|err| settings_write_error(ActionKind::ThemeSet, err))?;
    Ok(AppearanceMutationResult { changed })
}

pub(crate) fn setting_set_result(
    params: SettingSetParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SettingMutationResult, ControlError> {
    set_allowlisted_setting(&params.key, params.value, ctx)?;
    Ok(SettingMutationResult {
        setting: setting_summary_for_key(&params.key, ctx)?,
    })
}

pub(crate) fn setting_toggle_result(
    params: SettingToggleParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SettingMutationResult, ControlError> {
    let current = setting_summary_for_key(&params.key, ctx)?;
    let Some(value) = current.value.as_bool() else {
        return Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!(
                "{} is not a boolean setting and cannot be toggled",
                params.key
            ),
        ));
    };
    set_allowlisted_setting(&params.key, serde_json::json!(!value), ctx)?;
    Ok(SettingMutationResult {
        setting: setting_summary_for_key(&params.key, ctx)?,
    })
}

fn set_allowlisted_setting(
    key: &str,
    value: Value,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<(), ControlError> {
    match key {
        "appearance.themes.theme" => theme_set_result(
            ThemeSetParams {
                name: string_setting_value(key, &value)?,
            },
            ctx,
        )
        .map(|_| ()),
        "appearance.themes.system_theme" => {
            let enabled = bool_setting_value(key, &value)?;
            ThemeSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.use_system_theme.set_value(enabled, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "appearance.themes.light_theme" => {
            let theme = theme_kind_for_name(&string_setting_value(key, &value)?, ctx)?;
            ThemeSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    let current = settings.selected_system_themes.value().clone();
                    settings.selected_system_themes.set_value(
                        SelectedSystemThemes {
                            light: theme,
                            dark: current.dark,
                        },
                        ctx,
                    )
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "appearance.themes.dark_theme" => {
            let theme = theme_kind_for_name(&string_setting_value(key, &value)?, ctx)?;
            ThemeSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    let current = settings.selected_system_themes.value().clone();
                    settings.selected_system_themes.set_value(
                        SelectedSystemThemes {
                            light: current.light,
                            dark: theme,
                        },
                        ctx,
                    )
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "appearance.text.font_name" => {
            let font_name = string_setting_value(key, &value)?;
            if font_name.trim().is_empty() {
                return Err(ControlError::new(
                    ErrorCode::InvalidParams,
                    "appearance.text.font_name cannot be empty",
                ));
            }
            FontSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.monospace_font_name.set_value(font_name, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "appearance.text.font_size" => {
            let font_size = valid_font_size(u32_setting_value(key, &value)?)?;
            FontSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.monospace_font_size.set_value(font_size, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "appearance.window.zoom_level" => {
            let zoom_level = valid_zoom_level(u32_setting_value(key, &value)?)?;
            WindowSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.zoom_level.set_value(zoom_level, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "terminal.input.syntax_highlighting" => {
            let enabled = bool_setting_value(key, &value)?;
            InputSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.syntax_highlighting.set_value(enabled, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "terminal.input.error_underlining_enabled" => {
            let enabled = bool_setting_value(key, &value)?;
            InputSettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.error_underlining.set_value(enabled, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        "accessibility.accessibility_verbosity" => {
            let verbosity = accessibility_verbosity_value(key, &value)?;
            AccessibilitySettings::handle(ctx)
                .update(ctx, |settings, ctx| {
                    settings.a11y_verbosity.set_value(verbosity, ctx)
                })
                .map_err(|err| settings_write_error(ActionKind::SettingSet, err))
        }
        _ => Err(rejected_setting_key(key)),
    }
}

fn theme_kind_for_name(
    name: &str,
    ctx: &ModelContext<LocalControlBridge>,
) -> Result<ThemeKind, ControlError> {
    let matches = WarpConfig::as_ref(ctx)
        .theme_config()
        .theme_items()
        .filter_map(|(kind, _)| (public_theme_name(kind) == name).then_some(kind.clone()))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [theme] => Ok(theme.clone()),
        [] => Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!("{name} is not an available theme"),
        )),
        _ => Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!("{name} matches multiple themes"),
        )),
    }
}

fn valid_font_size(value: u32) -> Result<f32, ControlError> {
    if (5..=25).contains(&value) {
        return Ok(value as f32);
    }
    Err(ControlError::new(
        ErrorCode::InvalidParams,
        "font size must be between 5 and 25",
    ))
}

fn valid_zoom_level(value: u32) -> Result<u16, ControlError> {
    let value = u16::try_from(value).map_err(|err| {
        ControlError::with_details(
            ErrorCode::InvalidParams,
            "zoom level is outside the supported range",
            err.to_string(),
        )
    })?;
    if ZoomLevel::VALUES.contains(&value) {
        return Ok(value);
    }
    Err(ControlError::new(
        ErrorCode::InvalidParams,
        "zoom level must be one of the supported zoom percentages",
    ))
}

fn bool_setting_value(key: &str, value: &Value) -> Result<bool, ControlError> {
    value.as_bool().ok_or_else(|| {
        ControlError::new(
            ErrorCode::InvalidParams,
            format!("{key} requires a boolean value"),
        )
    })
}

fn string_setting_value(key: &str, value: &Value) -> Result<String, ControlError> {
    value.as_str().map(str::to_owned).ok_or_else(|| {
        ControlError::new(
            ErrorCode::InvalidParams,
            format!("{key} requires a string value"),
        )
    })
}

fn u32_setting_value(key: &str, value: &Value) -> Result<u32, ControlError> {
    if let Some(value) = value.as_u64().and_then(|value| u32::try_from(value).ok()) {
        return Ok(value);
    }
    Err(ControlError::new(
        ErrorCode::InvalidParams,
        format!("{key} requires a non-negative integer value"),
    ))
}

fn accessibility_verbosity_value(
    key: &str,
    value: &Value,
) -> Result<AccessibilityVerbosity, ControlError> {
    match string_setting_value(key, value)?.as_str() {
        "Verbose" | "verbose" | "VERBOSE" => Ok(AccessibilityVerbosity::Verbose),
        "Concise" | "concise" | "CONCISE" => Ok(AccessibilityVerbosity::Concise),
        _ => Err(ControlError::new(
            ErrorCode::InvalidParams,
            "accessibility.accessibility_verbosity must be Verbose or Concise",
        )),
    }
}

fn settings_write_error(action: ActionKind, err: anyhow::Error) -> ControlError {
    ControlError::with_details(
        ErrorCode::Internal,
        format!("{} failed to update app settings", action.as_str()),
        err.to_string(),
    )
}

pub(crate) fn rejected_setting_key(key: &str) -> ControlError {
    if PRIVATE_OR_SENSITIVE_SETTING_KEYS.contains(&key) {
        return ControlError::new(
            ErrorCode::NotAllowlisted,
            format!("{key} is private or sensitive and is not available through local control"),
        );
    }
    ControlError::new(
        ErrorCode::NotAllowlisted,
        format!("{key} is not an allowlisted local-control setting"),
    )
}

fn setting_summary_for_key(
    key: &str,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SettingSummary, ControlError> {
    let theme_settings = ThemeSettings::as_ref(ctx);
    let font_settings = FontSettings::as_ref(ctx);
    let input_settings = InputSettings::as_ref(ctx);
    let accessibility_settings = AccessibilitySettings::as_ref(ctx);
    let window_settings = WindowSettings::as_ref(ctx);
    match key {
        "appearance.themes.theme" => Ok(setting_summary(
            key,
            json!(public_theme_name(theme_settings.theme_kind.value())),
            "string",
        )),
        "appearance.themes.system_theme" => Ok(setting_summary(
            key,
            json!(*theme_settings.use_system_theme.value()),
            "bool",
        )),
        "appearance.themes.light_theme" => Ok(setting_summary(
            key,
            json!(public_theme_name(
                &theme_settings.selected_system_themes.value().light
            )),
            "string",
        )),
        "appearance.themes.dark_theme" => Ok(setting_summary(
            key,
            json!(public_theme_name(
                &theme_settings.selected_system_themes.value().dark
            )),
            "string",
        )),
        "appearance.text.font_name" => Ok(setting_summary(
            key,
            json!(font_settings.monospace_font_name.value()),
            "string",
        )),
        "appearance.text.font_size" => Ok(setting_summary(
            key,
            json!(*font_settings.monospace_font_size.value()),
            "number",
        )),
        "appearance.window.zoom_level" => Ok(setting_summary(
            key,
            json!(*window_settings.zoom_level.value()),
            "number",
        )),
        "terminal.input.syntax_highlighting" => Ok(setting_summary(
            key,
            json!(*input_settings.syntax_highlighting.value()),
            "bool",
        )),
        "terminal.input.error_underlining_enabled" => Ok(setting_summary(
            key,
            json!(*input_settings.error_underlining.value()),
            "bool",
        )),
        "accessibility.accessibility_verbosity" => Ok(setting_summary(
            key,
            json!(format!(
                "{:?}",
                accessibility_settings.a11y_verbosity.value()
            )),
            "string",
        )),
        _ => Err(rejected_setting_key(key)),
    }
}

fn setting_summary(key: &str, value: Value, value_type: &str) -> SettingSummary {
    SettingSummary {
        key: key.to_owned(),
        value,
        value_type: value_type.to_owned(),
    }
}

fn public_theme_name(theme: &ThemeKind) -> String {
    match theme {
        ThemeKind::Custom(custom) | ThemeKind::CustomBase16(custom) => custom.name(),
        ThemeKind::InMemory(_) => "In-memory theme".to_owned(),
        _ => theme.to_string(),
    }
}

fn active_theme_kind(
    theme_settings: &ThemeSettings,
    ctx: &ModelContext<LocalControlBridge>,
) -> ThemeKind {
    derived_theme_kind(theme_settings, ctx.system_theme())
}

fn rounded_u32(value: f32) -> Option<u32> {
    if value.is_finite() && value >= 0.0 && value <= u32::MAX as f32 {
        return Some(value.round() as u32);
    }
    None
}

fn to_control_data<T: Serialize>(value: T) -> Result<serde_json::Value, ControlError> {
    serde_json::to_value(value).map_err(|err| {
        ControlError::with_details(
            ErrorCode::Internal,
            "failed to serialize local-control response",
            err.to_string(),
        )
    })
}
