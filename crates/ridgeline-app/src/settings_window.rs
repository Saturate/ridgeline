use std::collections::HashMap;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    setting::{
        NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage,
        Settings as SettingsView,
    },
    Sizable as _,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use ridgeline_core::config::model::{AuthMethod, AzureDevOpsConfig, Config, ProviderConfig, ProviderIndicator};
use ridgeline_core::config::{credentials, loader};
use ridgeline_core::providers::azure_devops::AzureDevOpsProvider;
use ridgeline_core::providers::traits::PrProvider;

use crate::pr_row::parse_hex_color;
use crate::settings::{Appearance, Settings};

mod tabs;
use tabs::PendingTabEdit;
use crate::theme::InstalledThemes;

// --- Globals ---

#[derive(Clone)]
pub struct AppConfig(pub Config);
impl Global for AppConfig {}

/// Modification time of config.toml as of the last load, so hand edits made
/// while the app is open get picked up instead of overwritten by the next save.
#[derive(Default)]
struct ConfigFileStamp(Option<std::time::SystemTime>);
impl Global for ConfigFileStamp {}

fn config_mtime() -> Option<std::time::SystemTime> {
    let path = loader::config_path().ok()?;
    std::fs::metadata(path).ok()?.modified().ok()
}

pub fn remember_config_mtime(cx: &mut App) {
    cx.set_global(ConfigFileStamp(config_mtime()));
}

/// Polled from the main window.
pub fn reload_config_if_changed(cx: &mut App) {
    let current = config_mtime();
    let known = cx.try_global::<ConfigFileStamp>().and_then(|s| s.0);
    if current.is_none() || current == known {
        return;
    }
    cx.set_global(ConfigFileStamp(current));
    match loader::load_config() {
        Ok(config) => {
            cx.set_global(AppConfig(config));
            cx.refresh_windows();
        }
        Err(e) => tracing::warn!("config.toml changed but didn't parse, keeping the old one: {e}"),
    }
}

/// Live connection state per provider name, written by the main window after
/// connecting and after each poll.
#[derive(Clone, Default)]
pub struct ProviderHealth(pub HashMap<String, Result<String, String>>);
impl Global for ProviderHealth {}

/// Bumped whenever a token is written, so the main window reconnects with it.
#[derive(Default)]
pub struct CredentialsVersion(pub u64);
impl Global for CredentialsVersion {}

#[derive(Clone, Default)]
struct ConnectionResults(HashMap<String, String>);
impl Global for ConnectionResults {}

/// Signals the settings window to open a provider dialog on next render.
#[derive(Default)]
struct PendingProviderEdit(Option<Option<usize>>);
impl Global for PendingProviderEdit {}

struct ProviderDraft {
    auth: AuthMethod,
}


// --- Settings Window ---

pub struct SettingsWindow {
    _main: WeakEntity<crate::app::Ridgeline>,
    edit_name: Entity<InputState>,
    edit_url: Entity<InputState>,
    edit_pat: Entity<InputState>,
    edit_color: Entity<InputState>,
    tab_name: Entity<InputState>,
    tab_max_reviewers: Entity<InputState>,
    tab_branch_prefix: Entity<InputState>,
}

impl SettingsWindow {
    pub fn open(main: WeakEntity<crate::app::Ridgeline>, cx: &mut App) {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(720.), px(640.)), cx)),
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
        if !cx.has_global::<PendingTabEdit>() {
            cx.set_global(PendingTabEdit::default());
        }

        let _ = gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| SettingsWindow {
                _main: main,
                edit_name: cx.new(|cx| InputState::new(window, cx).placeholder("my-org")),
                edit_url: cx
                    .new(|cx| InputState::new(window, cx).placeholder("https://dev.azure.com/org")),
                edit_pat: cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Paste PAT here")
                        .masked(true)
                }),
                edit_color: cx.new(|cx| InputState::new(window, cx).placeholder("#3b82f6")),
                tab_name: cx.new(|cx| InputState::new(window, cx).placeholder("e.g. Up for grabs")),
                tab_max_reviewers: cx.new(|cx| InputState::new(window, cx).placeholder("any")),
                tab_branch_prefix: cx.new(|cx| InputState::new(window, cx).placeholder("feature/")),
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
        let pending_tab = cx.try_global::<PendingTabEdit>().and_then(|p| p.0);
        if let Some(editing_idx) = pending_tab {
            cx.set_global(PendingTabEdit(None));
            self.open_tab_dialog(editing_idx, window, cx);
        }
    }

    fn open_provider_dialog(
        &mut self,
        editing_idx: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let existing = editing_idx.and_then(|idx| {
            cx.try_global::<AppConfig>().and_then(|c| match c.0.providers.get(idx) {
                Some(ProviderConfig::AzureDevOps(ado)) => Some(ado.clone()),
                None => None,
            })
        });

        let (name, url, color, auth) = match &existing {
            Some(ado) => (
                ado.name.clone(),
                ado.url.clone(),
                ado.color.clone().unwrap_or_default(),
                ado.auth.clone(),
            ),
            None => (
                String::new(),
                "https://dev.azure.com/".to_string(),
                String::new(),
                AuthMethod::Pat,
            ),
        };
        let has_token = existing.is_some() && credentials::get_token(&name).is_ok();
        // Entries are found by name when saving, not by position: config.toml
        // can be reloaded from disk while the dialog is open.
        let original_name: Option<String> = existing.as_ref().map(|a| a.name.clone());

        self.edit_name.update(cx, |s, cx| s.set_value(&name, window, cx));
        self.edit_url.update(cx, |s, cx| s.set_value(&url, window, cx));
        self.edit_color.update(cx, |s, cx| s.set_value(&color, window, cx));
        // Leave the PAT field empty for an existing token; saving an empty
        // field keeps the stored one.
        self.edit_pat.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.set_placeholder(
                if has_token { "Stored. Paste a new one to replace it" } else { "Paste PAT here" },
                window,
                cx,
            );
        });

        let draft = cx.new(|_| ProviderDraft { auth });
        let name_input = self.edit_name.clone();
        let url_input = self.edit_url.clone();
        let pat_input = self.edit_pat.clone();
        let color_input = self.edit_color.clone();

        let title = if editing_idx.is_some() {
            "Edit provider"
        } else {
            "Add Azure DevOps provider"
        };

        window.open_dialog(cx, move |dialog, _, cx| {
            let auth = draft.read(cx).auth.clone();
            let color_preview = parse_hex_color(color_input.read(cx).value().trim());

            let mut fields = v_flex()
                .gap_3()
                .p_2()
                .child(labeled_field("Name", &name_input, "A short label, shown on badges and in errors"))
                .child(labeled_field("Organization URL", &url_input, "e.g. https://dev.azure.com/your-org"))
                .child(choice(
                    "Sign in with",
                    "provider-auth",
                    vec![
                        (AuthMethod::Pat, "Personal access token"),
                        (AuthMethod::AzureCli, "Azure CLI"),
                    ],
                    auth.clone(),
                    draft.clone(),
                    |d, v| d.auth = v,
                ));

            fields = match auth {
                AuthMethod::Pat => fields.child(labeled_field(
                    "Personal access token",
                    &pat_input,
                    "Needs Code (Read) scope. Saved as a plain file in Ridgeline's config folder.",
                )),
                AuthMethod::AzureCli => fields.child(
                    div()
                        .text_xs()
                        .child("Uses the account from `az login`. Run it in a terminal first if you haven't."),
                ),
                AuthMethod::DeviceCode => fields.child(div().text_xs().child(
                    "Browser sign-in has no sign-in flow in the app yet. Pick another method.",
                )),
            };

            fields = fields.child(
                v_flex()
                    .gap_1()
                    .child(field_label("Color"))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().flex_1().child(Input::new(&color_input).small()))
                            .child(
                                div()
                                    .size(px(22.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .when_some(color_preview, |d, c| d.bg(c)),
                            ),
                    )
                    .child(div().text_xs().child("Hex like #3b82f6. Used for the stripe or badge on each PR row.")),
            );

            dialog
                .title(title)
                .w(px(480.))
                .child(fields)
                .footer(
                    DialogFooter::new()
                        .when_some(original_name.clone(), |footer, original| {
                            footer.child(
                                h_flex().flex_1().child(
                                    Button::new("remove-provider")
                                        .ghost()
                                        .label("Remove provider")
                                        .on_click(move |_, window, cx| {
                                            remove_provider(&original, cx);
                                            window.close_dialog(cx);
                                        }),
                                ),
                            )
                        })
                        .child(footer_slot(DialogClose::new().child(Button::new("cancel").label("Cancel").outline())))
                        .child(footer_slot(DialogAction::new().child(
                            Button::new("save")
                                .label(if editing_idx.is_some() { "Save" } else { "Add" })
                                .primary(),
                        ))),
                )
                .on_ok({
                    let original_name = original_name.clone();
                    let draft = draft.clone();
                    let name_input = name_input.clone();
                    let url_input = url_input.clone();
                    let pat_input = pat_input.clone();
                    let color_input = color_input.clone();
                    move |_, _, cx| {
                        let name = name_input.read(cx).value().trim().to_string();
                        let url = url_input.read(cx).value().trim().to_string();
                        let pat = pat_input.read(cx).value().trim().to_string();
                        let color = color_input.read(cx).value().trim().to_string();
                        let auth = draft.read(cx).auth.clone();

                        if name.is_empty() || url.is_empty() {
                            return false;
                        }
                        if !color.is_empty() && parse_hex_color(&color).is_none() {
                            return false;
                        }
                        let color = (!color.is_empty()).then_some(color);

                        let config = cx.global_mut::<AppConfig>();
                        let position = |config: &AppConfig, wanted: &str| {
                            config.0.providers.iter().position(|p| {
                                let ProviderConfig::AzureDevOps(a) = p;
                                a.name == wanted
                            })
                        };
                        // Tokens, health and PR ids are all keyed by name, and token
                        // files live on case-insensitive filesystems on macOS and
                        // Windows, so "Work" and "work" would share one file.
                        let clashes = config.0.providers.iter().any(|p| {
                            let ProviderConfig::AzureDevOps(a) = p;
                            a.name.to_lowercase() == name.to_lowercase()
                                && original_name.as_deref() != Some(a.name.as_str())
                        });
                        if clashes {
                            return false;
                        }

                        let mut token_changed = false;
                        if let Some(original) = &original_name {
                            let Some(idx) = position(config, original) else {
                                // Removed from config.toml while the dialog was open.
                                return true;
                            };
                            if *original != name
                                && credentials::rename_token(original, &name).is_err()
                            {
                                // Keep the old name rather than leave the
                                // provider without a token under the new one.
                                return false;
                            }
                            let ProviderConfig::AzureDevOps(ado) = &mut config.0.providers[idx];
                            ado.name = name.clone();
                            ado.url = url;
                            ado.color = color;
                            ado.auth = auth.clone();
                        } else {
                            config.0.providers.push(ProviderConfig::AzureDevOps(
                                AzureDevOpsConfig {
                                    name: name.clone(),
                                    url,
                                    color,
                                    auth: auth.clone(),
                                    projects: Vec::new(),
                                },
                            ));
                        }

                        if auth == AuthMethod::Pat && !pat.is_empty() {
                            token_changed = credentials::store_token(&name, &pat).is_ok();
                        }

                        let _ = loader::save_config(&config.0);
                        let results = &mut cx.global_mut::<ConnectionResults>().0;
                        results.remove(&name);
                        if let Some(original) = &original_name {
                            results.remove(original);
                        }
                        if token_changed {
                            let version = cx.default_global::<CredentialsVersion>();
                            version.0 += 1;
                        }
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

        let mut rows: Vec<SettingItem> = config
            .providers
            .iter()
            .enumerate()
            .map(|(idx, provider)| {
                let ProviderConfig::AzureDevOps(ado) = provider;
                provider_row(idx, ado.clone(), results.0.get(&ado.name).cloned())
            })
            .collect();

        if rows.is_empty() {
            rows.push(empty_row("No providers yet. Add one to start polling."));
        }

        SettingPage::new("Providers")
            .icon(Icon::new(IconName::Globe))
            .title_suffix(|_, _| {
                Button::new("add-ado")
                    .outline()
                    .small()
                    .icon(Icon::new(IconName::Plus).size_3())
                    .label("Add provider")
                    .on_click(|_, _, cx| {
                        cx.global_mut::<PendingProviderEdit>().0 = Some(None);
                        cx.refresh_windows();
                    })
            })
            .group(SettingGroup::new().items(rows))
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
                    SettingItem::new("Provider indicator", SettingField::dropdown(
                        vec![
                            ("Border".into(), "Color stripe".into()),
                            ("Badge".into(), "Name badge".into()),
                            ("Off".into(), "Off".into()),
                        ],
                        // The dropdown matches this against option values, not labels.
                        |cx| match cx.try_global::<AppConfig>().map(|c| c.0.general.provider_indicator.clone()) {
                            Some(ProviderIndicator::Badge) => "Badge".into(),
                            Some(ProviderIndicator::Off) => "Off".into(),
                            _ => "Border".into(),
                        },
                        |label, cx| {
                            let indicator = match label.as_ref() {
                                "Name badge" | "Badge" => ProviderIndicator::Badge,
                                "Off" => ProviderIndicator::Off,
                                _ => ProviderIndicator::Border,
                            };
                            let c = cx.global_mut::<AppConfig>();
                            c.0.general.provider_indicator = indicator;
                            let _ = loader::save_config(&c.0);
                        },
                    )).description("How each PR row shows which organization it's from"),
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
                SettingItem::new("Test", SettingField::render(|options, _, _| {
                    Button::new("test-notification")
                        .outline()
                        .with_size(options.size())
                        .label("Send test notification")
                        .on_click(|_, _, _| {
                            ridgeline_core::notifications::sender::send_simple(
                                "Ridgeline",
                                "Notifications are working.",
                            );
                        })
                })).description("Check that your OS lets Ridgeline show notifications"),
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

/// DialogClose/DialogAction fill their parent, so without a fixed slot the
/// footer buttons stretch across the dialog.
fn footer_slot(button: impl IntoElement) -> impl IntoElement {
    div().w(px(96.)).flex_none().child(button)
}

fn field_label(label: &str) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.to_string())
}

fn labeled_field(label: &str, input: &Entity<InputState>, hint: &str) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(field_label(label))
        .child(Input::new(input).small())
        .child(div().text_xs().child(hint.to_string()))
}

/// A row of toggle buttons for picking one value inside a dialog. The dialog
/// rebuilds from `draft` on every render, so the click just updates it and
/// asks for a repaint.
fn choice<D: 'static, T: Clone + PartialEq + 'static>(
    label: &'static str,
    id: &'static str,
    options: Vec<(T, &'static str)>,
    current: T,
    draft: Entity<D>,
    set: fn(&mut D, T),
) -> impl IntoElement {
    v_flex().gap_1().child(field_label(label)).child(
        h_flex()
            .gap_1()
            .flex_wrap()
            .children(options.into_iter().enumerate().map(|(i, (value, text))| {
                let selected = value == current;
                let draft = draft.clone();
                let button = Button::new((id, i)).small().label(text);
                let button = if selected { button.primary() } else { button.outline() };
                button.on_click(move |_, window, cx| {
                    draft.update(cx, |d, _| set(d, value.clone()));
                    window.refresh();
                })
            })),
    )
}

/// One line in a settings list: the whole row opens the edit dialog, and
/// `trailing` holds the inline actions that shouldn't.
fn list_row(
    id: impl Into<ElementId>,
    leading: Option<AnyElement>,
    title: SharedString,
    subtitle: SharedString,
    trailing: AnyElement,
    on_open: impl Fn(&mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    h_flex()
        .id(id.into())
        .w_full()
        .gap_3()
        .px_2()
        .py_1p5()
        .rounded_md()
        .items_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.secondary))
        .on_click(move |_, _, cx| on_open(cx))
        .children(leading)
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).truncate().child(title))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(subtitle),
                ),
        )
        .child(trailing)
        .child(
            Icon::new(IconName::ChevronRight)
                .size_4()
                .text_color(theme.muted_foreground),
        )
}

fn empty_row(text: &'static str) -> SettingItem {
    SettingItem::render(move |_, _, cx| {
        div()
            .py_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(text)
    })
}

fn status_dot(color: Hsla) -> Div {
    div().size(px(8.)).flex_none().rounded_full().bg(color)
}

fn provider_row(idx: usize, ado: AzureDevOpsConfig, result: Option<String>) -> SettingItem {
    SettingItem::render(move |_, _, cx| {
        let theme = cx.theme();
        let has_token = credentials::get_token(&ado.name).is_ok();
        let auth_label = match ado.auth {
            AuthMethod::Pat => "Token",
            AuthMethod::AzureCli => "Azure CLI",
            AuthMethod::DeviceCode => "Browser sign-in",
        };
        let can_test = match ado.auth {
            AuthMethod::Pat => has_token,
            AuthMethod::AzureCli => true,
            AuthMethod::DeviceCode => false,
        };
        // A manual test result wins until the next edit; otherwise show what
        // the main window saw when it last talked to this provider.
        let live = cx
            .try_global::<ProviderHealth>()
            .and_then(|h| h.0.get(&ado.name).cloned());
        // (short label, full text for the tooltip, color)
        let (status, detail, color): (SharedString, SharedString, Hsla) =
            match (&result, live, &ado.auth) {
                (Some(r), _, _) if r.starts_with("Connected") => {
                    ("Connected".into(), r.clone().into(), theme.green)
                }
                (Some(r), _, _) if r.starts_with("Error") => {
                    ("Error".into(), r.clone().into(), theme.red)
                }
                (Some(r), _, _) => (r.clone().into(), r.clone().into(), theme.muted_foreground),
                (None, Some(Ok(user)), _) => {
                    ("Connected".into(), format!("Connected as {user}").into(), theme.green)
                }
                (None, Some(Err(e)), _) => ("Error".into(), e.into(), theme.red),
                (None, None, AuthMethod::DeviceCode) => (
                    "Not supported".into(),
                    "Browser sign-in isn't available yet. Edit and pick another method.".into(),
                    theme.red,
                ),
                (None, None, AuthMethod::Pat) if !has_token => {
                    ("No token".into(), "No token stored. Edit to add one.".into(), theme.red)
                }
                (None, None, _) => ("Connecting…".into(), "Connecting…".into(), theme.muted_foreground),
            };

        let accent = ado
            .color
            .as_deref()
            .and_then(parse_hex_color)
            .unwrap_or(theme.muted_foreground);

        let trailing = h_flex()
            .gap_3()
            .items_center()
            .flex_none()
            .child(
                div()
                    .px_1p5()
                    .rounded_sm()
                    .bg(theme.secondary)
                    .text_xs()
                    .text_color(theme.secondary_foreground)
                    .child(auth_label),
            )
            .child({
                let full = detail.clone();
                h_flex()
                    .id(("provider-status", idx))
                    .gap_1p5()
                    .items_center()
                    .max_w(px(160.))
                    .child(status_dot(color))
                    .child(div().text_xs().truncate().child(status))
                    .tooltip(move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(full.clone()).build(window, cx)
                    })
            })
            .when(can_test, |row| {
                let ado = ado.clone();
                row.child(
                    Button::new(("test", idx))
                        .ghost()
                        .xsmall()
                        .icon(Icon::new(IconName::RefreshCw))
                        .tooltip("Test connection")
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            test_connection(&ado, cx);
                        }),
                )
            })
            .into_any_element();

        list_row(
            ("provider", idx),
            Some(
                div()
                    .w(px(4.))
                    .h(px(28.))
                    .rounded_sm()
                    .bg(accent)
                    .into_any_element(),
            ),
            ado.name.clone().into(),
            ado.url.clone().into(),
            trailing,
            move |cx| {
                cx.global_mut::<PendingProviderEdit>().0 = Some(Some(idx));
                cx.refresh_windows();
            },
            cx,
        )
    })
}

fn remove_provider(name: &str, cx: &mut App) {
    let config = cx.global_mut::<AppConfig>();
    let Some(idx) = config.0.providers.iter().position(|p| {
        let ProviderConfig::AzureDevOps(a) = p;
        a.name == name
    }) else {
        return;
    };
    let _ = credentials::delete_token(name);
    config.0.providers.remove(idx);
    let _ = loader::save_config(&config.0);
    cx.refresh_windows();
}

fn test_connection(ado: &AzureDevOpsConfig, cx: &mut App) {
    let name = ado.name.clone();
    let provider = match ado.auth {
        AuthMethod::Pat => match credentials::get_token(&name) {
            Ok(token) => AzureDevOpsProvider::new(ado, &token),
            Err(_) => {
                cx.global_mut::<ConnectionResults>()
                    .0
                    .insert(name, "Error: no token stored".into());
                cx.refresh_windows();
                return;
            }
        },
        AuthMethod::AzureCli => AzureDevOpsProvider::with_credential(
            ado,
            std::sync::Arc::new(ridgeline_core::auth::AzureCliCredential),
        ),
        AuthMethod::DeviceCode => return,
    };

    cx.global_mut::<ConnectionResults>()
        .0
        .insert(name.clone(), "Testing...".into());
    cx.refresh_windows();

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
                Ok(user) => format!("Connected as {user}"),
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
            self.tabs_page(cx),
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
