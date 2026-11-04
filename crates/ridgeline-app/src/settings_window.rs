use std::collections::HashMap;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    setting::{
        NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage,
        Settings as SettingsView,
    },
    v_flex,
};
use gpui_kit::*;

use ridgeline_core::config::model::{AzureDevOpsConfig, Config, ProviderConfig};
use ridgeline_core::config::{credentials, loader};
use ridgeline_core::providers::azure_devops::AzureDevOpsProvider;
use ridgeline_core::providers::traits::PrProvider;

use crate::settings::{Appearance, Settings};
use crate::theme::InstalledThemes;

// --- Globals ---

#[derive(Clone)]
pub struct AppConfig(pub Config);
impl Global for AppConfig {}

#[derive(Clone, Default)]
struct ConnectionResults(HashMap<String, String>);
impl Global for ConnectionResults {}

/// Signals the settings window to open a provider dialog on next render.
#[derive(Default)]
struct PendingProviderEdit(Option<Option<usize>>);
impl Global for PendingProviderEdit {}

// --- Settings Window ---

pub struct SettingsWindow {
    _main: WeakEntity<crate::app::Ridgeline>,
    edit_name: Entity<InputState>,
    edit_url: Entity<InputState>,
    edit_pat: Entity<InputState>,
}

impl SettingsWindow {
    pub fn open(main: WeakEntity<crate::app::Ridgeline>, cx: &mut App) {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(680.), px(560.)), cx)),
            window_min_size: Some(size(px(480.), px(360.))),
            titlebar: Some(TitlebarOptions {
                title: Some("Settings".into()),
                ..TitleBar::title_bar_options()
            }),
            ..TitleBar::window_options()
        };

        if !cx.has_global::<ConnectionResults>() {
            cx.set_global(ConnectionResults::default());
        }
        if !cx.has_global::<PendingProviderEdit>() {
            cx.set_global(PendingProviderEdit::default());
        }

        let _ = gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| SettingsWindow {
                _main: main,
                edit_name: cx.new(|cx| InputState::new(window, cx).placeholder("my-org")),
                edit_url: cx
                    .new(|cx| InputState::new(window, cx).placeholder("https://dev.azure.com/org")),
                edit_pat: cx.new(|cx| InputState::new(window, cx).placeholder("Paste PAT here")),
            })
        });
    }

    fn check_pending_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pending = cx
            .try_global::<PendingProviderEdit>()
            .and_then(|p| p.0);
        if let Some(editing_idx) = pending {
            cx.set_global(PendingProviderEdit(None));
            self.open_provider_dialog(editing_idx, window, cx);
        }
    }

    fn open_provider_dialog(
        &mut self,
        editing_idx: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let config = cx
            .try_global::<AppConfig>()
            .map(|c| c.0.clone())
            .unwrap_or_default();

        if let Some(idx) = editing_idx {
            if let Some(ProviderConfig::AzureDevOps(ado)) = config.providers.get(idx) {
                self.edit_name
                    .update(cx, |s, cx| s.set_value(&ado.name, window, cx));
                self.edit_url
                    .update(cx, |s, cx| s.set_value(&ado.url, window, cx));
                let has_token = credentials::get_token(&ado.name).is_ok();
                self.edit_pat.update(cx, |s, cx| {
                    s.set_value(if has_token { "••••••••" } else { "" }, window, cx)
                });
            }
        } else {
            self.edit_name
                .update(cx, |s, cx| s.set_value("", window, cx));
            self.edit_url
                .update(cx, |s, cx| s.set_value("https://dev.azure.com/", window, cx));
            self.edit_pat
                .update(cx, |s, cx| s.set_value("", window, cx));
        }

        let name_input = self.edit_name.clone();
        let url_input = self.edit_url.clone();
        let pat_input = self.edit_pat.clone();

        let title = if editing_idx.is_some() {
            "Edit Provider"
        } else {
            "Add Azure DevOps Provider"
        };

        window.open_dialog(cx, move |dialog, _, _| {
            let name_input = name_input.clone();
            let url_input = url_input.clone();
            let pat_input = pat_input.clone();

            dialog
                .title(title)
                .w(px(460.))
                .child(
                    v_flex()
                        .gap_3()
                        .p_2()
                        .child(labeled_field("Name", &name_input, "A short label for this connection"))
                        .child(labeled_field("Organization URL", &url_input, "e.g. https://dev.azure.com/your-org"))
                        .child(labeled_field("Personal Access Token", &pat_input, "Needs Code (Read) scope")),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
                        .child(DialogAction::new().child(
                            Button::new("save")
                                .label(if editing_idx.is_some() { "Save" } else { "Add" })
                                .primary(),
                        )),
                )
                .on_ok({
                    let name_input = name_input.clone();
                    let url_input = url_input.clone();
                    let pat_input = pat_input.clone();
                    move |_, _, cx| {
                        let name = name_input.read(cx).value().to_string();
                        let url = url_input.read(cx).value().to_string();
                        let pat = pat_input.read(cx).value().to_string();

                        if name.is_empty() || url.is_empty() {
                            return false;
                        }

                        let config = cx.global_mut::<AppConfig>();
                        if let Some(idx) = editing_idx {
                            if let Some(ProviderConfig::AzureDevOps(ado)) =
                                config.0.providers.get_mut(idx)
                            {
                                let old_name = ado.name.clone();
                                ado.name = name.clone();
                                ado.url = url;
                                if old_name != name {
                                    if let Ok(old_token) = credentials::get_token(&old_name) {
                                        let _ = credentials::store_token(&name, &old_token);
                                        let _ = credentials::delete_token(&old_name);
                                    }
                                }
                            }
                        } else {
                            config.0.providers.push(ProviderConfig::AzureDevOps(
                                AzureDevOpsConfig {
                                    name: name.clone(),
                                    url,
                                    color: None,
                                    projects: Vec::new(),
                                },
                            ));
                        }

                        if !pat.is_empty() && !pat.starts_with("••") {
                            let _ = credentials::store_token(&name, &pat);
                        }

                        let _ = loader::save_config(&config.0);
                        cx.refresh_windows();
                        true
                    }
                })
        });

        self.edit_name
            .update(cx, |state, cx| state.focus(window, cx));
    }

    fn providers_page(&self, cx: &Context<Self>) -> SettingPage {
        let config = cx
            .try_global::<AppConfig>()
            .map(|c| c.0.clone())
            .unwrap_or_default();

        let results = cx
            .try_global::<ConnectionResults>()
            .cloned()
            .unwrap_or_default();

        let mut groups = Vec::new();

        for (idx, provider) in config.providers.iter().enumerate() {
            let ProviderConfig::AzureDevOps(ado) = provider;
            let name = ado.name.clone();
            let url = ado.url.clone();
            let has_token = credentials::get_token(&name).is_ok();
            let test_result = results.0.get(&name).cloned();

            let status = if let Some(ref result) = test_result {
                result.clone()
            } else if has_token {
                "Token stored".into()
            } else {
                "No token".into()
            };

            groups.push(
                SettingGroup::new()
                    .title(format!("Azure DevOps — {}", name))
                    .items(vec![
                        SettingItem::new(
                            "URL",
                            SettingField::render({
                                let url = url.clone();
                                move |_, _, cx| {
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(url.clone())
                                }
                            }),
                        ),
                        SettingItem::new(
                            "Status",
                            SettingField::render({
                                let status = status.clone();
                                move |_, _, cx| {
                                    let color = if status.starts_with("Connected:") {
                                        gpui_kit::green()
                                    } else if status.starts_with("Error:") || status == "No token" {
                                        gpui_kit::red()
                                    } else {
                                        cx.theme().muted_foreground
                                    };
                                    div().text_sm().text_color(color).child(status.clone())
                                }
                            }),
                        ),
                        SettingItem::new(
                            "",
                            SettingField::render({
                                let name = name.clone();
                                let url = url.clone();
                                move |options, _, _cx| {
                                    let mut row = h_flex().gap_2();
                                    row = row.child(
                                        Button::new(("edit", idx))
                                            .outline()
                                            .with_size(options.size())
                                            .label("Edit")
                                            .on_click(move |_, _w, cx| {
                                                cx.global_mut::<PendingProviderEdit>().0 =
                                                    Some(Some(idx));
                                                cx.refresh_windows();
                                            }),
                                    );
                                    if has_token {
                                        row = row.child(
                                            Button::new(("test", idx))
                                                .outline()
                                                .with_size(options.size())
                                                .label("Test")
                                                .on_click({
                                                    let name = name.clone();
                                                    let url = url.clone();
                                                    move |_, _w, cx| {
                                                        test_connection(&name, &url, cx);
                                                    }
                                                }),
                                        );
                                    }
                                    row = row.child(
                                        Button::new(("remove", idx))
                                            .ghost()
                                            .with_size(options.size())
                                            .label("Remove")
                                            .on_click(move |_, _w, cx| {
                                                remove_provider(idx, cx);
                                            }),
                                    );
                                    row
                                }
                            }),
                        ),
                    ]),
            );
        }

        groups.push(SettingGroup::new().title("").items(vec![SettingItem::new(
            "",
            SettingField::render(move |options, _, _cx| {
                Button::new("add-ado")
                    .outline()
                    .with_size(options.size())
                    .icon(Icon::new(IconName::Plus).size_3())
                    .label("Add Azure DevOps Provider")
                    .on_click(|_, _w, cx| {
                        cx.global_mut::<PendingProviderEdit>().0 = Some(None);
                        cx.refresh_windows();
                    })
            }),
        )]));

        SettingPage::new("Providers")
            .icon(Icon::new(IconName::Settings))
            .groups(groups)
    }

    fn general_page(&self) -> SettingPage {
        SettingPage::new("General")
            .icon(Icon::new(IconName::Settings))
            .groups(vec![
                SettingGroup::new().title("Polling").items(vec![
                    SettingItem::new("Refresh interval", SettingField::number_input(
                        NumberFieldOptions { min: 10., max: 600., step: 10. },
                        |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.refresh_interval_secs as f64).unwrap_or(60.),
                        |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.refresh_interval_secs = v as u64; let _ = loader::save_config(&c.0); },
                    )).description("Seconds between polls"),
                ]),
                SettingGroup::new().title("PR Age Indicators").items(vec![
                    SettingItem::new("Warning after (hours)", SettingField::number_input(
                        NumberFieldOptions { min: 1., max: 720., step: 12. },
                        |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.age_warning_hours as f64).unwrap_or(48.),
                        |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.age_warning_hours = v as u64; let _ = loader::save_config(&c.0); },
                    )).description("Age turns yellow"),
                    SettingItem::new("Danger after (hours)", SettingField::number_input(
                        NumberFieldOptions { min: 1., max: 2160., step: 24. },
                        |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.age_danger_hours as f64).unwrap_or(144.),
                        |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.age_danger_hours = v as u64; let _ = loader::save_config(&c.0); },
                    )).description("Age turns red"),
                ]),
                SettingGroup::new().title("Display").items(vec![
                    SettingItem::new("Show project name", SettingField::switch(
                        |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.show_project_name).unwrap_or(true),
                        |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.show_project_name = v; let _ = loader::save_config(&c.0); },
                    )),
                    SettingItem::new("Parse conventional commits", SettingField::switch(
                        |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.parse_conventional_commits).unwrap_or(false),
                        |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.parse_conventional_commits = v; let _ = loader::save_config(&c.0); },
                    )),
                ]),
            ])
    }

    fn notifications_page(&self) -> SettingPage {
        SettingPage::new("Notifications")
            .icon(Icon::new(IconName::Bell))
            .groups(vec![SettingGroup::new().title("Desktop Notifications").items(vec![
                SettingItem::new("Enabled", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications_enabled).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications_enabled = v; let _ = loader::save_config(&c.0); },
                )).description("Master toggle"),
                SettingItem::new("New pull request", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications.new_pr).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications.new_pr = v; let _ = loader::save_config(&c.0); },
                )),
                SettingItem::new("Vote changed", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications.vote_changed).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications.vote_changed = v; let _ = loader::save_config(&c.0); },
                )),
                SettingItem::new("Waiting for author", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications.waiting_for_author).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications.waiting_for_author = v; let _ = loader::save_config(&c.0); },
                )),
                SettingItem::new("Build failed", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications.build_failed).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications.build_failed = v; let _ = loader::save_config(&c.0); },
                )),
                SettingItem::new("PR completed", SettingField::switch(
                    |cx| cx.try_global::<AppConfig>().map(|c| c.0.general.notifications.completed).unwrap_or(true),
                    |v, cx| { let c = cx.global_mut::<AppConfig>(); c.0.general.notifications.completed = v; let _ = loader::save_config(&c.0); },
                )),
            ])])
    }

    fn appearance_page(&self, cx: &Context<Self>) -> SettingPage {
        let appearance_items: Vec<(SharedString, SharedString)> = Appearance::ALL.iter().map(|a| (a.label().into(), a.label().into())).collect();
        let dark_themes: Vec<(SharedString, SharedString)> = cx.try_global::<InstalledThemes>().map(|i| i.themes.iter().filter(|(_, dark)| *dark).map(|(n, _)| (n.clone().into(), n.clone().into())).collect()).unwrap_or_default();
        let light_themes: Vec<(SharedString, SharedString)> = cx.try_global::<InstalledThemes>().map(|i| i.themes.iter().filter(|(_, dark)| !*dark).map(|(n, _)| (n.clone().into(), n.clone().into())).collect()).unwrap_or_default();

        SettingPage::new("Appearance")
            .icon(Icon::new(IconName::Palette))
            .groups(vec![SettingGroup::new().title("Theme").items(vec![
                SettingItem::new("Mode", SettingField::dropdown(appearance_items,
                    |cx| Settings::get(cx).appearance.label().into(),
                    |label, cx| Settings::update(cx, |s| s.appearance = Appearance::from_label(&label)),
                )).description("System follows your OS preference"),
                SettingItem::new("Dark Theme", SettingField::dropdown(dark_themes,
                    |cx| Settings::get(cx).dark_theme_name(cx).into(),
                    |label, cx| Settings::update(cx, |s| s.dark_theme = Some(label.to_string())),
                )),
                SettingItem::new("Light Theme", SettingField::dropdown(light_themes,
                    |cx| Settings::get(cx).light_theme_name(cx).into(),
                    |label, cx| Settings::update(cx, |s| s.light_theme = Some(label.to_string())),
                )),
            ])])
    }
}

fn labeled_field(label: &str, input: &Entity<InputState>, hint: &str) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(label.to_string()))
        .child(Input::new(input).small())
        .child(div().text_xs().child(hint.to_string()))
}

fn remove_provider(idx: usize, cx: &mut App) {
    let config = cx.global_mut::<AppConfig>();
    if idx < config.0.providers.len() {
        let ProviderConfig::AzureDevOps(ado) = &config.0.providers[idx];
        let _ = credentials::delete_token(&ado.name);
        config.0.providers.remove(idx);
        let _ = loader::save_config(&config.0);
        cx.refresh_windows();
    }
}

fn test_connection(name: &str, url: &str, cx: &mut App) {
    let name = name.to_string();
    let url = url.to_string();

    let results = cx.global_mut::<ConnectionResults>();
    results.0.insert(name.clone(), "Testing...".into());
    cx.refresh_windows();

    let token = match credentials::get_token(&name) {
        Ok(t) => t,
        Err(_) => {
            let results = cx.global_mut::<ConnectionResults>();
            results.0.insert(name, "Error: no token stored".into());
            cx.refresh_windows();
            return;
        }
    };

    let provider = AzureDevOpsProvider::new_from_params(&name, &url, &token);
    cx.spawn(async move |cx| {
        let result = crate::io(async move {
            provider
                .get_current_user()
                .await
                .map(|u| u.display_name)
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .await;

        let _ = cx.update(|cx| {
            let msg = match result {
                Ok(name) => format!("Connected: {name}"),
                Err(e) => format!("Error: {e}"),
            };
            cx.global_mut::<ConnectionResults>().0.insert(name, msg);
            cx.refresh_windows();
        });
    })
    .detach();
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.check_pending_dialog(window, cx);

        let theme = cx.theme();
        let pages = vec![
            self.providers_page(cx),
            self.general_page(),
            self.notifications_page(),
            self.appearance_page(cx),
        ];

        v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(TitleBar::new().child(div().text_sm().child("Ridgeline Settings")))
            .child(SettingsView::new("ridgeline-settings").small().pages(pages))
    }
}
