use super::{
    settings_page::{
        render_body_item, render_settings_info_banner, LocalOnlyIconState, MatchData, PageType,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
    },
    SettingsSection, ToggleState,
};
use crate::appearance::Appearance;
use crate::features::FeatureFlag;
use crate::report_if_error;
use crate::settings::{
    AllowInsideWarpAppStateMutations, AllowInsideWarpControl,
    AllowInsideWarpMetadataConfigurationMutations, AllowInsideWarpMetadataReads,
    AllowInsideWarpUnderlyingDataMutations, AllowInsideWarpUnderlyingDataReads,
    AllowOutsideWarpAppStateMutations, AllowOutsideWarpControl,
    AllowOutsideWarpMetadataConfigurationMutations, AllowOutsideWarpMetadataReads,
    AllowOutsideWarpUnderlyingDataMutations, AllowOutsideWarpUnderlyingDataReads,
    LocalControlInvocationContext, LocalControlSettings,
};
use settings::{Setting as _, ToggleableSetting as _};
use std::cell::RefCell;
use std::collections::HashMap;
use warp_core::settings::SyncToCloud;
use warpui::elements::{Container, Element, MouseStateHandle};
use warpui::ui_components::components::UiComponent;
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

#[derive(Clone, Copy, Debug)]
pub enum ScriptingToggle {
    InsideWarpControl,
    InsideWarpMetadataReads,
    InsideWarpUnderlyingDataReads,
    InsideWarpAppStateMutations,
    InsideWarpMetadataConfigurationMutations,
    InsideWarpUnderlyingDataMutations,
    OutsideWarpControl,
    OutsideWarpMetadataReads,
    OutsideWarpUnderlyingDataReads,
    OutsideWarpAppStateMutations,
    OutsideWarpMetadataConfigurationMutations,
    OutsideWarpUnderlyingDataMutations,
}

impl ScriptingToggle {
    fn label(self) -> &'static str {
        match self {
            Self::InsideWarpControl => "Warp control within Warp",
            Self::OutsideWarpControl => "Warp control outside Warp",
            Self::InsideWarpMetadataReads | Self::OutsideWarpMetadataReads => {
                "Allow metadata reads"
            }
            Self::InsideWarpUnderlyingDataReads | Self::OutsideWarpUnderlyingDataReads => {
                "Allow underlying data reads"
            }
            Self::InsideWarpAppStateMutations | Self::OutsideWarpAppStateMutations => {
                "Allow app-state mutations"
            }
            Self::InsideWarpMetadataConfigurationMutations
            | Self::OutsideWarpMetadataConfigurationMutations => {
                "Allow metadata/configuration mutations"
            }
            Self::InsideWarpUnderlyingDataMutations | Self::OutsideWarpUnderlyingDataMutations => {
                "Allow underlying data mutations"
            }
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::InsideWarpControl => {
                "Allows control commands launched from verified Warp-managed terminal sessions."
            }
            Self::OutsideWarpControl => {
                "Allows other local apps, terminals, IDEs, launch agents, and scripts to request Warp control."
            }
            Self::InsideWarpMetadataReads => {
                "Allows commands inside Warp to inspect non-sensitive app structure and configuration metadata."
            }
            Self::OutsideWarpMetadataReads => {
                "Allows external local clients to inspect non-sensitive app structure and configuration metadata after outside-Warp control is enabled."
            }
            Self::InsideWarpUnderlyingDataReads => {
                "Allows commands inside Warp to read content-bearing local state such as terminal output or command history."
            }
            Self::OutsideWarpUnderlyingDataReads => {
                "Allows external local clients to read content-bearing local state after outside-Warp control is enabled."
            }
            Self::InsideWarpAppStateMutations => {
                "Allows commands inside Warp to change local UI and layout state, such as creating a tab."
            }
            Self::OutsideWarpAppStateMutations => {
                "Allows external local clients to change local UI and layout state after outside-Warp control is enabled."
            }
            Self::InsideWarpMetadataConfigurationMutations => {
                "Allows commands inside Warp to change persistent metadata or configuration."
            }
            Self::OutsideWarpMetadataConfigurationMutations => {
                "Allows external local clients to change persistent metadata or configuration after outside-Warp control is enabled."
            }
            Self::InsideWarpUnderlyingDataMutations => {
                "Allows commands inside Warp to execute or change underlying data."
            }
            Self::OutsideWarpUnderlyingDataMutations => {
                "Allows external local clients to execute or change underlying data after outside-Warp control is enabled."
            }
        }
    }

    fn search_terms(self) -> &'static str {
        match self {
            Self::InsideWarpControl => "inside warp control terminal scripting automation",
            Self::OutsideWarpControl => {
                "outside warp control external scripts automation local cli"
            }
            Self::InsideWarpMetadataReads => {
                "inside warp metadata reads query windows tabs panes instances"
            }
            Self::OutsideWarpMetadataReads => {
                "outside warp metadata reads query windows tabs panes instances"
            }
            Self::InsideWarpUnderlyingDataReads => {
                "inside warp underlying data reads terminal output history contents"
            }
            Self::OutsideWarpUnderlyingDataReads => {
                "outside warp underlying data reads terminal output history contents"
            }
            Self::InsideWarpAppStateMutations => "inside warp app state mutations tab create",
            Self::OutsideWarpAppStateMutations => "outside warp app state mutations tab create",
            Self::InsideWarpMetadataConfigurationMutations => {
                "inside warp metadata configuration mutations settings theme rename"
            }
            Self::OutsideWarpMetadataConfigurationMutations => {
                "outside warp metadata configuration mutations settings theme rename"
            }
            Self::InsideWarpUnderlyingDataMutations => {
                "inside warp underlying data mutations command execution file write"
            }
            Self::OutsideWarpUnderlyingDataMutations => {
                "outside warp underlying data mutations command execution file write"
            }
        }
    }

    fn value(self, settings: &LocalControlSettings) -> bool {
        match self {
            Self::InsideWarpControl => *settings.allow_inside_warp_control,
            Self::OutsideWarpControl => *settings.allow_outside_warp_control,
            Self::InsideWarpMetadataReads => *settings.allow_inside_warp_metadata_reads,
            Self::OutsideWarpMetadataReads => *settings.allow_outside_warp_metadata_reads,
            Self::InsideWarpUnderlyingDataReads => {
                *settings.allow_inside_warp_underlying_data_reads
            }
            Self::OutsideWarpUnderlyingDataReads => {
                *settings.allow_outside_warp_underlying_data_reads
            }
            Self::InsideWarpAppStateMutations => *settings.allow_inside_warp_app_state_mutations,
            Self::OutsideWarpAppStateMutations => *settings.allow_outside_warp_app_state_mutations,
            Self::InsideWarpMetadataConfigurationMutations => {
                *settings.allow_inside_warp_metadata_configuration_mutations
            }
            Self::OutsideWarpMetadataConfigurationMutations => {
                *settings.allow_outside_warp_metadata_configuration_mutations
            }
            Self::InsideWarpUnderlyingDataMutations => {
                *settings.allow_inside_warp_underlying_data_mutations
            }
            Self::OutsideWarpUnderlyingDataMutations => {
                *settings.allow_outside_warp_underlying_data_mutations
            }
        }
    }

    fn storage_key(self) -> &'static str {
        match self {
            Self::InsideWarpControl => AllowInsideWarpControl::storage_key(),
            Self::OutsideWarpControl => AllowOutsideWarpControl::storage_key(),
            Self::InsideWarpMetadataReads => AllowInsideWarpMetadataReads::storage_key(),
            Self::OutsideWarpMetadataReads => AllowOutsideWarpMetadataReads::storage_key(),
            Self::InsideWarpUnderlyingDataReads => {
                AllowInsideWarpUnderlyingDataReads::storage_key()
            }
            Self::OutsideWarpUnderlyingDataReads => {
                AllowOutsideWarpUnderlyingDataReads::storage_key()
            }
            Self::InsideWarpAppStateMutations => AllowInsideWarpAppStateMutations::storage_key(),
            Self::OutsideWarpAppStateMutations => AllowOutsideWarpAppStateMutations::storage_key(),
            Self::InsideWarpMetadataConfigurationMutations => {
                AllowInsideWarpMetadataConfigurationMutations::storage_key()
            }
            Self::OutsideWarpMetadataConfigurationMutations => {
                AllowOutsideWarpMetadataConfigurationMutations::storage_key()
            }
            Self::InsideWarpUnderlyingDataMutations => {
                AllowInsideWarpUnderlyingDataMutations::storage_key()
            }
            Self::OutsideWarpUnderlyingDataMutations => {
                AllowOutsideWarpUnderlyingDataMutations::storage_key()
            }
        }
    }

    fn sync_to_cloud(self) -> SyncToCloud {
        match self {
            Self::InsideWarpControl => AllowInsideWarpControl::sync_to_cloud(),
            Self::OutsideWarpControl => AllowOutsideWarpControl::sync_to_cloud(),
            Self::InsideWarpMetadataReads => AllowInsideWarpMetadataReads::sync_to_cloud(),
            Self::OutsideWarpMetadataReads => AllowOutsideWarpMetadataReads::sync_to_cloud(),
            Self::InsideWarpUnderlyingDataReads => {
                AllowInsideWarpUnderlyingDataReads::sync_to_cloud()
            }
            Self::OutsideWarpUnderlyingDataReads => {
                AllowOutsideWarpUnderlyingDataReads::sync_to_cloud()
            }
            Self::InsideWarpAppStateMutations => AllowInsideWarpAppStateMutations::sync_to_cloud(),
            Self::OutsideWarpAppStateMutations => {
                AllowOutsideWarpAppStateMutations::sync_to_cloud()
            }
            Self::InsideWarpMetadataConfigurationMutations => {
                AllowInsideWarpMetadataConfigurationMutations::sync_to_cloud()
            }
            Self::OutsideWarpMetadataConfigurationMutations => {
                AllowOutsideWarpMetadataConfigurationMutations::sync_to_cloud()
            }
            Self::InsideWarpUnderlyingDataMutations => {
                AllowInsideWarpUnderlyingDataMutations::sync_to_cloud()
            }
            Self::OutsideWarpUnderlyingDataMutations => {
                AllowOutsideWarpUnderlyingDataMutations::sync_to_cloud()
            }
        }
    }

    fn parent_context(self) -> Option<LocalControlInvocationContext> {
        match self {
            Self::InsideWarpMetadataReads
            | Self::InsideWarpUnderlyingDataReads
            | Self::InsideWarpAppStateMutations
            | Self::InsideWarpMetadataConfigurationMutations
            | Self::InsideWarpUnderlyingDataMutations => {
                Some(LocalControlInvocationContext::InsideWarp)
            }
            Self::OutsideWarpMetadataReads
            | Self::OutsideWarpUnderlyingDataReads
            | Self::OutsideWarpAppStateMutations
            | Self::OutsideWarpMetadataConfigurationMutations
            | Self::OutsideWarpUnderlyingDataMutations => {
                Some(LocalControlInvocationContext::OutsideWarp)
            }
            Self::InsideWarpControl | Self::OutsideWarpControl => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ScriptingSettingsPageAction {
    Toggle(ScriptingToggle),
}

pub struct ScriptingSettingsPageView {
    page: PageType<Self>,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
}

impl ScriptingSettingsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&LocalControlSettings::handle(ctx), |_, _, _, ctx| {
            ctx.notify();
        });

        Self {
            page: PageType::new_uncategorized(
                vec![
                    Box::new(ScriptingIntroWidget),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpControl,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpMetadataReads,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpUnderlyingDataReads,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpAppStateMutations,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpMetadataConfigurationMutations,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::InsideWarpUnderlyingDataMutations,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpControl,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpMetadataReads,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpUnderlyingDataReads,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpAppStateMutations,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpMetadataConfigurationMutations,
                    )),
                    Box::new(ScriptingToggleWidget::new(
                        ScriptingToggle::OutsideWarpUnderlyingDataMutations,
                    )),
                ],
                Some("Scripting"),
            ),
            local_only_icon_tooltip_states: RefCell::new(HashMap::new()),
        }
    }
}

impl Entity for ScriptingSettingsPageView {
    type Event = ();
}

impl TypedActionView for ScriptingSettingsPageView {
    type Action = ScriptingSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ScriptingSettingsPageAction::Toggle(toggle) => {
                LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| match toggle {
                    ScriptingToggle::InsideWarpControl => {
                        report_if_error!(settings
                            .allow_inside_warp_control
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpControl => {
                        report_if_error!(settings
                            .allow_outside_warp_control
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::InsideWarpMetadataReads => {
                        report_if_error!(settings
                            .allow_inside_warp_metadata_reads
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpMetadataReads => {
                        report_if_error!(settings
                            .allow_outside_warp_metadata_reads
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::InsideWarpUnderlyingDataReads => {
                        report_if_error!(settings
                            .allow_inside_warp_underlying_data_reads
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpUnderlyingDataReads => {
                        report_if_error!(settings
                            .allow_outside_warp_underlying_data_reads
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::InsideWarpAppStateMutations => {
                        report_if_error!(settings
                            .allow_inside_warp_app_state_mutations
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpAppStateMutations => {
                        report_if_error!(settings
                            .allow_outside_warp_app_state_mutations
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::InsideWarpMetadataConfigurationMutations => {
                        report_if_error!(settings
                            .allow_inside_warp_metadata_configuration_mutations
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpMetadataConfigurationMutations => {
                        report_if_error!(settings
                            .allow_outside_warp_metadata_configuration_mutations
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::InsideWarpUnderlyingDataMutations => {
                        report_if_error!(settings
                            .allow_inside_warp_underlying_data_mutations
                            .toggle_and_save_value(ctx));
                    }
                    ScriptingToggle::OutsideWarpUnderlyingDataMutations => {
                        report_if_error!(settings
                            .allow_outside_warp_underlying_data_mutations
                            .toggle_and_save_value(ctx));
                    }
                });
                ctx.notify();
            }
        }
    }
}

impl View for ScriptingSettingsPageView {
    fn ui_name() -> &'static str {
        "ScriptingSettingsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for ScriptingSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::Scripting
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        cfg!(not(target_family = "wasm")) && FeatureFlag::WarpControlCli.is_enabled()
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<ScriptingSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<ScriptingSettingsPageView>) -> Self {
        SettingsPageViewHandle::Scripting(view_handle)
    }
}

struct ScriptingIntroWidget;

impl SettingsWidget for ScriptingIntroWidget {
    type View = ScriptingSettingsPageView;

    fn search_terms(&self) -> &str {
        "scripting warp control automation warpctrl local cli inside outside permissions"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        render_settings_info_banner(
            "Warp control lets local scripts automate allowlisted actions in a running Warp app.",
            Some("Enable Warp control within Warp for commands launched from Warp-managed terminals, or outside Warp for other local apps and scripts. Each scope has separate permission categories for metadata reads, underlying data reads, app-state mutations, metadata/configuration mutations, and underlying data mutations."),
            appearance,
        )
    }
}

struct ScriptingToggleWidget {
    toggle: ScriptingToggle,
    switch_state: SwitchStateHandle,
}

impl ScriptingToggleWidget {
    fn new(toggle: ScriptingToggle) -> Self {
        Self {
            toggle,
            switch_state: SwitchStateHandle::default(),
        }
    }
}

impl SettingsWidget for ScriptingToggleWidget {
    type View = ScriptingSettingsPageView;

    fn search_terms(&self) -> &str {
        self.toggle.search_terms()
    }

    fn should_render(&self, app: &AppContext) -> bool {
        let settings = LocalControlSettings::as_ref(app);
        match self.toggle.parent_context() {
            Some(context) => settings.is_context_enabled(context),
            None => true,
        }
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = LocalControlSettings::as_ref(app);
        let checked = self.toggle.value(settings);
        let toggle = self.toggle;

        let item = render_body_item::<ScriptingSettingsPageAction>(
            self.toggle.label().to_owned(),
            None,
            LocalOnlyIconState::for_setting(
                self.toggle.storage_key(),
                self.toggle.sync_to_cloud(),
                &mut view.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(checked)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(ScriptingSettingsPageAction::Toggle(toggle));
                })
                .finish(),
            Some(self.toggle.description().to_owned()),
        );
        if self.toggle.parent_context().is_some() {
            Container::new(item).with_margin_left(16.).finish()
        } else {
            item
        }
    }
}
