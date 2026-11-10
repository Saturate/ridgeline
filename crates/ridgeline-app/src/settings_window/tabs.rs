use gpui_kit::component::Disableable as _;
use gpui_kit::component::{
    Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    setting::{SettingGroup, SettingItem, SettingPage},
    switch::Switch,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use gpui_kit::component::input::InputState;
use ridgeline_core::config::loader;
use ridgeline_core::config::model::{
    Assigned, DraftFilter, TabConfig, TabDisplay, TabFilter, TabSource, WaitingOn,
};

use super::{AppConfig, SettingsWindow, choice, field_label, footer_slot, labeled_field, list_row};

/// Same pending-dialog handoff as providers: setting fields can't reach the
/// window entity, so they leave a note and the window opens the dialog on render.
#[derive(Default)]
pub(super) struct PendingTabEdit(pub Option<Option<usize>>);
impl Global for PendingTabEdit {}

const CC_TYPES: [&str; 7] = ["feat", "fix", "refactor", "perf", "docs", "test", "chore"];

fn source_label(source: &TabSource) -> &'static str {
    match source {
        TabSource::Reviewing => "Reviewing",
        TabSource::Authored => "My PRs",
        TabSource::All => "Everything",
    }
}

fn summary(tab: &TabConfig) -> String {
    let mut parts = vec![source_label(&tab.source).to_string()];
    parts.push(
        match tab.display {
            TabDisplay::Reviewing => "vote chips",
            TabDisplay::Authored => "status summary",
        }
        .into(),
    );
    match tab.filter.waiting_on {
        Some(WaitingOn::Author) => parts.push("your turn".into()),
        Some(WaitingOn::Reviewers) => parts.push("waiting on reviewers".into()),
        None => {}
    }
    match tab.filter.drafts {
        Some(DraftFilter::Hide) => parts.push("no drafts".into()),
        Some(DraftFilter::Only) => parts.push("drafts only".into()),
        _ => {}
    }
    if tab.filter.hide_approved_by_me {
        parts.push("hides your approvals".into());
    }
    match tab.filter.assigned {
        Some(Assigned::Direct) => parts.push("assigned to you".into()),
        Some(Assigned::Indirect) => parts.push("via team".into()),
        None => {}
    }
    if tab.filter.required_only {
        parts.push("you're required".into());
    }
    if !tab.filter.repos.is_empty() {
        parts.push(format!("repos {}", tab.filter.repos.join(", ")));
    }
    if !tab.filter.authors.is_empty() {
        parts.push(format!("authors {}", tab.filter.authors.join(", ")));
    }
    match (tab.filter.min_age_days, tab.filter.max_age_days) {
        (Some(min), Some(max)) => parts.push(format!("{min}-{max} days old")),
        (Some(min), None) => parts.push(format!("older than {min} days")),
        (None, Some(max)) => parts.push(format!("newer than {max} days")),
        (None, None) => {}
    }
    if let Some(target) = &tab.filter.target_branch {
        parts.push(format!("into {target}*"));
    }
    if let Some(max) = tab.filter.max_reviewers {
        parts.push(format!("max {max} reviewers"));
    }
    if let Some(prefix) = &tab.filter.branch_prefix {
        parts.push(format!("branch {prefix}*"));
    }
    if !tab.filter.cc_types.is_empty() {
        parts.push(tab.filter.cc_types.join("/"));
    }
    parts.join(" · ")
}

fn update_tabs(cx: &mut App, f: impl FnOnce(&mut Vec<TabConfig>)) {
    let config = cx.global_mut::<AppConfig>();
    f(&mut config.0.general.tabs);
    let _ = loader::save_config(&config.0);
    cx.refresh_windows();
}

fn pair(left: impl IntoElement, right: impl IntoElement) -> impl IntoElement {
    h_flex()
        .gap_4()
        .items_start()
        .child(div().flex_1().min_w_0().child(left))
        .child(div().flex_1().min_w_0().child(right))
}

fn tab_row(idx: usize, count: usize, tab: TabConfig) -> SettingItem {
    SettingItem::render(move |_, _, cx| {
        let enabled = tab.enabled;
        let trailing = h_flex()
            .gap_1()
            .items_center()
            .flex_none()
            .child(
                Button::new(("tab-up", idx))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::ChevronUp))
                    .tooltip("Move left")
                    .disabled(idx == 0)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        update_tabs(cx, |tabs| {
                            // idx is from the last render; a reload from disk
                            // can shrink the list before the click lands.
                            if idx > 0 && idx < tabs.len() {
                                tabs.swap(idx, idx - 1);
                            }
                        });
                    }),
            )
            .child(
                Button::new(("tab-down", idx))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::ChevronDown))
                    .tooltip("Move right")
                    .disabled(idx + 1 >= count)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        update_tabs(cx, |tabs| {
                            if idx + 1 < tabs.len() {
                                tabs.swap(idx, idx + 1);
                            }
                        });
                    }),
            )
            .into_any_element();

        let switch = div()
            .id(("tab-enabled-wrap", idx))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                Switch::new(("tab-enabled", idx))
                    .checked(enabled)
                    .small()
                    .on_click(move |checked, _, cx| {
                        let checked = *checked;
                        update_tabs(cx, |tabs| {
                            // Keep one tab on; the main window has nothing to
                            // show otherwise.
                            if !checked && enabled_count_without(tabs, idx) == 0 {
                                return;
                            }
                            if let Some(t) = tabs.get_mut(idx) {
                                t.enabled = checked;
                            }
                        });
                    }),
            )
            .into_any_element();

        list_row(
            ("tab", idx),
            Some(switch),
            tab.label.clone().into(),
            summary(&tab).into(),
            trailing,
            move |cx| {
                cx.global_mut::<PendingTabEdit>().0 = Some(Some(idx));
                cx.refresh_windows();
            },
            cx,
        )
        .when(!enabled, |row| row.opacity(0.6))
    })
}

/// The free-text filter fields added after the first set, bundled so the
/// dialog closures can carry them as one value.
#[derive(Clone)]
pub(super) struct TabMoreInputs {
    repos: Entity<InputState>,
    authors: Entity<InputState>,
    target_branch: Entity<InputState>,
    min_age: Entity<InputState>,
    max_age: Entity<InputState>,
}

impl TabMoreInputs {
    pub(super) fn new(window: &mut Window, cx: &mut App) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut App| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        Self {
            repos: input("e.g. DCC Energi, !BuildTemplates", window, cx),
            authors: input("e.g. !renovate, !dependabot", window, cx),
            target_branch: input("e.g. release/", window, cx),
            min_age: input("any", window, cx),
            max_age: input("any", window, cx),
        }
    }

    fn load(&self, filter: &TabFilter, window: &mut Window, cx: &mut App) {
        let num = |n: Option<u32>| n.map(|n| n.to_string()).unwrap_or_default();
        let set = |input: &Entity<InputState>, text: String, window: &mut Window, cx: &mut App| {
            input.update(cx, |s, cx| s.set_value(&text, window, cx));
        };
        set(&self.repos, filter.repos.join(", "), window, cx);
        set(&self.authors, filter.authors.join(", "), window, cx);
        set(
            &self.target_branch,
            filter.target_branch.clone().unwrap_or_default(),
            window,
            cx,
        );
        set(&self.min_age, num(filter.min_age_days), window, cx);
        set(&self.max_age, num(filter.max_age_days), window, cx);
    }

    /// Writes the fields into `filter`; false when an age isn't a number.
    fn store(&self, filter: &mut TabFilter, cx: &App) -> bool {
        let text = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let days = |input: &Entity<InputState>| -> Result<Option<u32>, ()> {
            let t = text(input);
            if t.is_empty() {
                Ok(None)
            } else {
                t.parse().map(Some).map_err(|_| ())
            }
        };
        let (Ok(min), Ok(max)) = (days(&self.min_age), days(&self.max_age)) else {
            return false;
        };
        filter.repos = ridgeline_core::filters::parse_patterns(&text(&self.repos));
        filter.authors = ridgeline_core::filters::parse_patterns(&text(&self.authors));
        let target = text(&self.target_branch);
        filter.target_branch = (!target.is_empty()).then_some(target);
        filter.min_age_days = min;
        filter.max_age_days = max;
        true
    }
}

fn enabled_count_without(tabs: &[TabConfig], idx: usize) -> usize {
    tabs.iter()
        .enumerate()
        .filter(|(i, t)| *i != idx && t.enabled)
        .count()
}

/// Dialogs remember the tab they opened with instead of its position, since
/// config.toml can be reloaded (and reordered) while the dialog is open.
fn find_tab(tabs: &[TabConfig], original: &str) -> Option<usize> {
    tabs.iter()
        .position(|t| serde_json::to_string(t).ok().as_deref() == Some(original))
}

fn new_tab() -> TabConfig {
    TabConfig {
        label: String::new(),
        source: TabSource::All,
        display: TabDisplay::Reviewing,
        enabled: true,
        filter: TabFilter::default(),
    }
}

impl SettingsWindow {
    pub(super) fn tabs_page(&self, cx: &Context<Self>) -> SettingPage {
        let tabs = cx
            .try_global::<AppConfig>()
            .map(|c| c.0.general.tabs.clone())
            .unwrap_or_default();
        let count = tabs.len();

        let rows: Vec<SettingItem> = tabs
            .into_iter()
            .enumerate()
            .map(|(idx, tab)| tab_row(idx, count, tab))
            .collect();

        SettingPage::new("Tabs")
            .icon(Icon::new(IconName::LayoutDashboard))
            .title_suffix(|_, _| {
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("tab-open-file")
                            .ghost()
                            .small()
                            .icon(Icon::new(IconName::FileText).size_3())
                            .label("Edit as TOML")
                            .tooltip(
                                "Opens config.toml. Saved changes load within a couple of seconds.",
                            )
                            .on_click(|_, _, _| {
                                if let Ok(path) = loader::config_path() {
                                    let _ = open::that(path);
                                }
                            }),
                    )
                    .child(
                        Button::new("tab-add")
                            .outline()
                            .small()
                            .icon(Icon::new(IconName::Plus).size_3())
                            .label("Add tab")
                            .on_click(|_, _, cx| {
                                cx.global_mut::<PendingTabEdit>().0 = Some(None);
                                cx.refresh_windows();
                            }),
                    )
            })
            .group(SettingGroup::new().items(rows))
    }

    pub(super) fn open_tab_dialog(
        &mut self,
        editing_idx: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let existing = editing_idx.and_then(|idx| {
            cx.try_global::<AppConfig>()
                .and_then(|c| c.0.general.tabs.get(idx).cloned())
        });
        let original: Option<String> = existing
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok());
        let can_remove = editing_idx.is_some_and(|idx| {
            cx.try_global::<AppConfig>()
                .is_some_and(|c| enabled_count_without(&c.0.general.tabs, idx) > 0)
        });
        let mut initial = existing.unwrap_or_else(new_tab);
        // "Show" and "no filter" behave the same; collapse them so one button lights up.
        if initial.filter.drafts == Some(DraftFilter::Show) {
            initial.filter.drafts = None;
        }

        let max_text = initial
            .filter
            .max_reviewers
            .map(|m| m.to_string())
            .unwrap_or_default();
        let prefix_text = initial.filter.branch_prefix.clone().unwrap_or_default();
        self.tab_name
            .update(cx, |s, cx| s.set_value(&initial.label, window, cx));
        self.tab_max_reviewers
            .update(cx, |s, cx| s.set_value(&max_text, window, cx));
        self.tab_branch_prefix
            .update(cx, |s, cx| s.set_value(&prefix_text, window, cx));
        self.tab_more.load(&initial.filter, window, cx);
        let more = self.tab_more.clone();

        let draft = cx.new(|_| initial);
        let name_input = self.tab_name.clone();
        let max_input = self.tab_max_reviewers.clone();
        let prefix_input = self.tab_branch_prefix.clone();
        let title = if editing_idx.is_some() {
            "Edit tab"
        } else {
            "New tab"
        };

        window.open_dialog(cx, move |dialog, _, cx| {
            let tab = draft.read(cx).clone();

            let cc_buttons = h_flex().gap_1().flex_wrap().children(
                CC_TYPES.iter().enumerate().map(|(i, kind)| {
                    let selected = tab.filter.cc_types.iter().any(|t| t == kind);
                    let draft = draft.clone();
                    let button = Button::new(("tab-cc", i)).small().label(*kind);
                    let button = if selected { button.primary() } else { button.outline() };
                    button.on_click(move |_, window, cx| {
                        draft.update(cx, |t, _| {
                            let types = &mut t.filter.cc_types;
                            if let Some(pos) = types.iter().position(|t| t == kind) {
                                types.remove(pos);
                            } else {
                                types.push(kind.to_string());
                            }
                        });
                        window.refresh();
                    })
                }),
            );

            dialog
                .title(title)
                .w(px(560.))
                .child(
                    // Taller than the settings window otherwise; scroll inside
                    // the dialog so the footer stays reachable.
                    div()
                        .id("tab-form")
                        .max_h(px(400.))
                        .overflow_y_scroll()
                        .child(
                    v_flex()
                        .gap_3()
                        .p_2()
                        .child(labeled_field("Name", &name_input, "Shown on the tab"))
                        .child(pair(
                            choice(
                                "Show PRs from",
                                "tab-source",
                                vec![
                                    (TabSource::Reviewing, "Reviewing"),
                                    (TabSource::Authored, "My PRs"),
                                    (TabSource::All, "Everything"),
                                ],
                                tab.source.clone(),
                                draft.clone(),
                                |t, v| t.source = v,
                            ),
                            choice(
                                "Row style",
                                "tab-display",
                                vec![
                                    (TabDisplay::Reviewing, "Vote chips"),
                                    (TabDisplay::Authored, "Status summary"),
                                ],
                                tab.display.clone(),
                                draft.clone(),
                                |t, v| t.display = v,
                            ),
                        ))
                        .child(pair(
                            // Comment threads are only fetched for your own PRs,
                            // so "waiting on" is only accurate there.
                            if tab.source == TabSource::Authored {
                                choice(
                                "Waiting on",
                                "tab-waiting",
                                vec![
                                    (None, "Anyone"),
                                    (Some(WaitingOn::Author), "Author"),
                                    (Some(WaitingOn::Reviewers), "Reviewers"),
                                ],
                                tab.filter.waiting_on.clone(),
                                draft.clone(),
                                |t, v| t.filter.waiting_on = v,
                            )
                                .into_any_element()
                            } else {
                                div().into_any_element()
                            },
                            choice(
                                "Drafts",
                                "tab-drafts",
                                vec![
                                    (None, "Show"),
                                    (Some(DraftFilter::Hide), "Hide"),
                                    (Some(DraftFilter::Only), "Only"),
                                ],
                                tab.filter.drafts.clone(),
                                draft.clone(),
                                |t, v| t.filter.drafts = v,
                            ),
                        ))
                        .child(pair(
                            labeled_field("Max reviewers", &max_input, "Empty for any, 0 for unassigned"),
                            labeled_field("Branch prefix", &prefix_input, "e.g. feature/"),
                        ))
                        .child(pair(
                            labeled_field("Repos", &more.repos, "Provider, project or repo; ! excludes"),
                            labeled_field("Authors", &more.authors, "Name or email; ! excludes"),
                        ))
                        .child(pair(
                            choice(
                                "Assigned",
                                "tab-assigned",
                                vec![
                                    (None, "Any"),
                                    (Some(Assigned::Direct), "To me"),
                                    (Some(Assigned::Indirect), "Via team"),
                                ],
                                tab.filter.assigned.clone(),
                                draft.clone(),
                                |t, v| t.filter.assigned = v,
                            ),
                            choice(
                                "Required",
                                "tab-required",
                                vec![(false, "Any"), (true, "Only mine")],
                                tab.filter.required_only,
                                draft.clone(),
                                |t, v| t.filter.required_only = v,
                            ),
                        ))
                        .child(pair(
                            labeled_field("Older than (days)", &more.min_age, "e.g. 7 for a stale tab"),
                            labeled_field("Newer than (days)", &more.max_age, "Empty for any"),
                        ))
                        .child(pair(
                            labeled_field("Target branch", &more.target_branch, "Prefix, e.g. release/"),
                            choice(
                                "PRs you approved",
                                "tab-approved",
                                vec![(false, "Show"), (true, "Hide")],
                                tab.filter.hide_approved_by_me,
                                draft.clone(),
                                |t, v| t.filter.hide_approved_by_me = v,
                            ),
                        ))
                        .child(
                            v_flex()
                                .gap_1()
                                .child(field_label("Commit types"))
                                .child(cc_buttons)
                                .child(div().text_xs().child(
                                    "Only conventional-commit titles of these types. None selected shows all.",
                                )),
                        ),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .when_some(original.clone().filter(|_| can_remove), |footer, original| {
                            footer.child(
                                h_flex().flex_1().child(
                                    Button::new("tab-remove")
                                        .ghost()
                                        .label("Remove tab")
                                        .on_click(move |_, window, cx| {
                                            update_tabs(cx, |tabs| {
                                                if let Some(idx) = find_tab(tabs, &original) {
                                                    tabs.remove(idx);
                                                }
                                            });
                                            window.close_dialog(cx);
                                        }),
                                ),
                            )
                        })
                        .child(footer_slot(DialogClose::new().child(
                            Button::new("tab-cancel").label("Cancel").outline(),
                        )))
                        .child(footer_slot(DialogAction::new().child(
                            Button::new("tab-save")
                                .label(if editing_idx.is_some() { "Save" } else { "Add" })
                                .primary(),
                        ))),
                )
                .on_ok({
                    let original = original.clone();
                    let draft = draft.clone();
                    let name_input = name_input.clone();
                    let max_input = max_input.clone();
                    let prefix_input = prefix_input.clone();
                    let more = more.clone();
                    move |_, _, cx| {
                        let label = name_input.read(cx).value().trim().to_string();
                        if label.is_empty() {
                            return false;
                        }
                        let max_text = max_input.read(cx).value().trim().to_string();
                        let max_reviewers = if max_text.is_empty() {
                            None
                        } else {
                            match max_text.parse::<u32>() {
                                Ok(n) => Some(n),
                                Err(_) => return false,
                            }
                        };
                        let prefix = prefix_input.read(cx).value().trim().to_string();

                        let mut tab = draft.read(cx).clone();
                        tab.label = label;
                        tab.filter.max_reviewers = max_reviewers;
                        tab.filter.branch_prefix = (!prefix.is_empty()).then_some(prefix);
                        if !more.store(&mut tab.filter, cx) {
                            return false;
                        }
                        if tab.source != TabSource::Authored {
                            tab.filter.waiting_on = None;
                        }

                        update_tabs(cx, |tabs| match &original {
                            Some(original) => {
                                // Gone from config.toml since the dialog opened:
                                // drop the edit rather than overwrite another tab.
                                if let Some(idx) = find_tab(tabs, original) {
                                    tabs[idx] = tab;
                                }
                            }
                            None => tabs.push(tab),
                        });
                        true
                    }
                })
        });

        self.tab_name.update(cx, |s, cx| s.focus(window, cx));
    }
}
