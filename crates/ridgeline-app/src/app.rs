use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    status_bar::StatusBar,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use ridgeline_core::config::model::{Config, TabConfig, TabSource};
use ridgeline_core::notifications::tracker::ChangeTracker;
use ridgeline_core::polling::poller::Poller;
use ridgeline_core::providers::traits::PrProvider;
use ridgeline_core::providers::types::*;
use ridgeline_core::util::time::relative_time;
use ridgeline_core::CachedBuildStatus;

use crate::settings_window::SettingsWindow;
use crate::*;

const DETAIL_PANEL_WIDTH: f32 = 380.;

pub struct Ridgeline {
    focus: FocusHandle,
    config: Config,
    providers: Vec<(Arc<dyn PrProvider>, UserId)>,

    pub reviewing: Vec<PullRequest>,
    pub authored: Vec<PullRequest>,
    errors: Vec<ridgeline_core::polling::error::PollError>,
    selected_pr: Option<PrId>,
    selected_detail: Option<PrDetail>,
    active_tab: usize,

    search: Entity<InputState>,
    search_text: String,

    last_poll: Option<chrono::DateTime<chrono::Utc>>,
    status_message: Option<String>,

    change_tracker: ChangeTracker,
    build_cache: HashMap<String, CachedBuildStatus>,

    init_task: Option<Task<()>>,
    poll_task: Option<Task<()>>,
    detail_task: Option<Task<()>>,
}

impl Ridgeline {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Filter PRs..."));

        let _ = cx.subscribe_in(&search, window, |this, _, ev: &InputEvent, _window, cx| {
            if matches!(ev, InputEvent::Change) {
                // Read text from the search entity next render
                this.search_text = this
                    .search
                    .read(cx)
                    .value()
                    .to_string();
                cx.notify();
            }
        });

        let config = ridgeline_core::config::loader::load_config().unwrap_or_else(|_| Config {
            general: Default::default(),
            providers: Vec::new(),
        });

        cx.set_global(crate::settings_window::AppConfig(config.clone()));

        let mut this = Self {
            focus: cx.focus_handle(),
            config,
            providers: Vec::new(),
            reviewing: Vec::new(),
            authored: Vec::new(),
            errors: Vec::new(),
            selected_pr: None,
            selected_detail: None,
            active_tab: 0,
            search,
            search_text: String::new(),
            last_poll: None,
            status_message: Some("Connecting to providers...".into()),
            change_tracker: ChangeTracker::new(),
            build_cache: HashMap::new(),
            init_task: None,
            poll_task: None,
            detail_task: None,
        };

        this.start_init(cx);
        this
    }

    fn start_init(&mut self, cx: &mut Context<Self>) {
        let config = self.config.clone();
        self.init_task = Some(cx.spawn(async move |this, cx| {
            let providers_result = io(async move {
                Ok(ridgeline_core::init_providers(&config).await)
            })
            .await;

            match providers_result {
                Ok(results) => {
                    let providers: Vec<(Arc<dyn PrProvider>, UserId)> = results
                        .iter()
                        .map(|(p, u, _)| (p.clone(), u.clone()))
                        .collect();
                    let names: Vec<String> = results.iter().map(|(_, _, n)| n.clone()).collect();

                    this.update(cx, |this, cx| {
                        this.providers = providers;
                        if names.is_empty() {
                            this.status_message =
                                Some("No providers configured. Open Settings to add one.".into());
                        } else {
                            this.status_message =
                                Some(format!("Connected: {}", names.join(", ")));
                            this.start_polling(cx);
                        }
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.status_message = Some(format!("Init failed: {e}"));
                        cx.notify();
                    })
                    .ok();
                }
            }
        }));
    }

    fn start_polling(&mut self, cx: &mut Context<Self>) {
        let providers = self.providers.clone();
        let interval_secs = self.config.general.refresh_interval_secs;

        self.poll_task = Some(cx.spawn(async move |this, cx| {
            let mut first_poll = true;
            loop {
                let cache = this
                    .update(cx, |this, _| std::mem::take(&mut this.build_cache))
                    .unwrap_or_default();

                let poll_result = io({
                    let providers = providers.clone();
                    let mut cache = cache;
                    async move {
                        let poller = Poller::new(providers.clone());
                        let mut result = poller.poll_once().await;
                        ridgeline_core::enrich_build_status(
                            &mut result.reviewing,
                            &providers,
                            &mut cache,
                        )
                        .await;
                        ridgeline_core::enrich_build_status(
                            &mut result.authored,
                            &providers,
                            &mut cache,
                        )
                        .await;
                        Ok((result, cache))
                    }
                })
                .await;

                match poll_result {
                    Ok((result, cache)) => {

                        this.update(cx, |this, cx| {
                            let all_prs: Vec<_> = result
                                .reviewing
                                .iter()
                                .chain(result.authored.iter())
                                .cloned()
                                .collect();
                            let changes = this.change_tracker.detect_changes(&all_prs);

                            if !first_poll {
                                let notif_config = (
                                    this.config.general.notifications_enabled,
                                    this.config.general.notifications.clone(),
                                );
                                for change in &changes {
                                    if notif_config.0
                                        && ridgeline_core::should_notify(change, &notif_config.1)
                                    {
                                        ridgeline_core::notifications::sender::send_with_url(
                                            &change.notification_title(),
                                            &change.notification_body(),
                                            change.web_url(),
                                        );
                                    }
                                }
                            }

                            this.reviewing = result.reviewing;
                            this.authored = result.authored;
                            this.errors = result.errors;
                            this.build_cache = cache;
                            this.last_poll = Some(chrono::Utc::now());
                            this.status_message = None;
                            first_poll = false;
                            cx.notify();
                        })
                        .ok();
                    }
                    Err(e) => {
                        this.update(cx, |this, cx| {
                            this.status_message = Some(format!("Poll error: {e}"));
                            cx.notify();
                        })
                        .ok();
                    }
                }

                let sleep = io(async move {
                    tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                    Ok(())
                })
                .await;
                if sleep.is_err() {
                    break;
                }
            }
        }));
    }

    fn refresh(&mut self, _: &Refresh, _window: &mut Window, cx: &mut Context<Self>) {
        self.poll_task = None;
        self.status_message = Some("Refreshing...".into());
        cx.notify();
        self.start_polling(cx);
    }

    fn open_settings(&mut self, _: &OpenSettings, _window: &mut Window, cx: &mut Context<Self>) {
        SettingsWindow::open(cx.entity().downgrade(), cx);
    }

    fn open_in_browser(&mut self, _: &OpenInBrowser, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(pr_id) = &self.selected_pr {
            if let Some((provider, _)) = self.providers.iter().find(|(p, _)| p.name() == pr_id.provider) {
                let url = provider.web_url(pr_id);
                let _ = open::that(&url);
            }
        }
    }

    fn next_tab(&mut self, _: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        let tab_count = self.enabled_tabs().len();
        if tab_count > 0 {
            self.active_tab = (self.active_tab + 1) % tab_count;
            self.selected_pr = None;
            self.selected_detail = None;
            cx.notify();
        }
    }

    fn previous_tab(&mut self, _: &PreviousTab, _window: &mut Window, cx: &mut Context<Self>) {
        let tab_count = self.enabled_tabs().len();
        if tab_count > 0 {
            self.active_tab = if self.active_tab == 0 {
                tab_count - 1
            } else {
                self.active_tab - 1
            };
            self.selected_pr = None;
            self.selected_detail = None;
            cx.notify();
        }
    }

    fn select_pr(&mut self, pr_id: PrId, cx: &mut Context<Self>) {
        self.selected_pr = Some(pr_id.clone());
        self.selected_detail = None;
        cx.notify();

        let providers = self.providers.clone();
        self.detail_task = Some(cx.spawn(async move |this, cx| {
            let provider = providers
                .iter()
                .find(|(p, _)| p.name() == pr_id.provider)
                .map(|(p, _)| p.clone());

            if let Some(provider) = provider {
                let detail = io({
                    let provider = provider.clone();
                    let pr_id = pr_id.clone();
                    async move { provider.get_detail(&pr_id).await.map_err(|e| anyhow::anyhow!("{e}")) }
                })
                .await;

                if let Ok(detail) = detail {
                    this.update(cx, |this, cx| {
                        if this.selected_pr.as_ref() == Some(&pr_id) {
                            this.selected_detail = Some(detail);
                            cx.notify();
                        }
                    })
                    .ok();
                }
            }
        }));
    }

    fn enabled_tabs(&self) -> Vec<&TabConfig> {
        self.config.general.tabs.iter().filter(|t| t.enabled).collect()
    }

    fn prs_for_active_tab(&self) -> Vec<&PullRequest> {
        let tabs = self.enabled_tabs();
        let Some(tab) = tabs.get(self.active_tab) else {
            return Vec::new();
        };

        let source_prs: Vec<&PullRequest> = match tab.source {
            TabSource::Reviewing => self.reviewing.iter().collect(),
            TabSource::Authored => self.authored.iter().collect(),
            TabSource::All => self
                .reviewing
                .iter()
                .chain(self.authored.iter())
                .collect(),
        };

        source_prs
            .into_iter()
            .filter(|pr| {
                if let Some(max) = tab.filter.max_reviewers {
                    if pr.reviewers.len() as u32 > max {
                        return false;
                    }
                }
                if let Some(ref drafts) = tab.filter.drafts {
                    use ridgeline_core::config::model::DraftFilter;
                    match drafts {
                        DraftFilter::Hide if pr.is_draft => return false,
                        DraftFilter::Only if !pr.is_draft => return false,
                        _ => {}
                    }
                }
                if let Some(ref prefix) = tab.filter.branch_prefix {
                    if !pr.source_branch.starts_with(prefix) {
                        return false;
                    }
                }
                if !self.search_text.is_empty() {
                    let q = self.search_text.to_lowercase();
                    let matches = pr.title.to_lowercase().contains(&q)
                        || pr.author.display_name.to_lowercase().contains(&q)
                        || pr.repository.name.to_lowercase().contains(&q)
                        || pr.repository.project.to_lowercase().contains(&q);
                    if !matches {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("Ridgeline v{}", env!("CARGO_PKG_VERSION"))),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new("refresh")
                            .ghost()
                            .compact()
                            .icon(Icon::new(IconName::RefreshCw).size_4())
                            .tooltip("Refresh (Cmd+R)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.refresh(&Refresh, window, cx);
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .ghost()
                            .compact()
                            .icon(Icon::new(IconName::Settings).size_4())
                            .tooltip("Settings (Cmd+,)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_settings(&OpenSettings, window, cx);
                            })),
                    ),
            )
    }

    fn render_main_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_detail = self.selected_pr.is_some();
        let mut row = h_flex()
            .flex_1()
            .min_h_0()
            .child(self.render_pr_list(cx));

        if has_detail {
            if let Some(panel) = self.render_detail_panel(cx) {
                row = row.child(panel);
            }
        }
        row
    }

    fn render_pr_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let tabs = self.enabled_tabs();
        let prs = self.prs_for_active_tab();

        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.background)
            .child(
                h_flex()
                    .px_2()
                    .py_1()
                    .gap_1()
                    .border_b_1()
                    .border_color(theme.border)
                    .children(tabs.iter().enumerate().map(|(i, tab)| {
                        let is_active = i == self.active_tab;
                        let label = tab.label.clone();
                        let count = self.count_for_tab(tab);
                        Button::new(("tab", i))
                            .ghost()
                            .compact()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(div().text_xs().child(label))
                                    .when(count > 0, |el| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .px_1()
                                                .rounded(px(8.))
                                                .bg(if is_active {
                                                    theme.accent
                                                } else {
                                                    theme.muted
                                                })
                                                .text_color(if is_active {
                                                    theme.accent_foreground
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .child(format!("{count}")),
                                        )
                                    }),
                            )
                            .when(is_active, |el| {
                                el.bg(theme.sidebar_primary)
                            })
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.active_tab = i;
                                this.selected_pr = None;
                                this.selected_detail = None;
                                cx.notify();
                            }))
                    })),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .child(Input::new(&self.search).small()),
            )
            .child({
                let mut list = v_flex()
                    .id("pr-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll();
                for pr in &prs {
                    list = list.child(self.render_pr_row(pr, cx));
                }
                list
            })
    }

    fn count_for_tab(&self, tab: &TabConfig) -> usize {
        match tab.source {
            TabSource::Reviewing => self.reviewing.len(),
            TabSource::Authored => self.authored.len(),
            TabSource::All => self.reviewing.len() + self.authored.len(),
        }
    }

    fn render_pr_row(&self, pr: &PullRequest, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_selected = self.selected_pr.as_ref() == Some(&pr.id);
        let pr_id = pr.id.clone();
        let age = relative_time(&pr.created_at);
        let age_hours = chrono::Utc::now()
            .signed_duration_since(pr.created_at)
            .num_hours() as u64;
        let warning_hours = self.config.general.age_warning_hours;
        let danger_hours = self.config.general.age_danger_hours;

        let age_color = if age_hours >= danger_hours {
            gpui_kit::red()
        } else if age_hours >= warning_hours {
            gpui_kit::yellow()
        } else {
            theme.muted_foreground
        };

        let vote_summary = vote_summary_text(&pr.reviewers);
        let build_icon = build_status_icon(pr.build_status.as_ref());

        div()
            .id(("pr", pr.id.number))
            .w_full()
            .px_2()
            .py_1()
            .cursor_pointer()
            .when(is_selected, |el| el.bg(theme.sidebar_primary))
            .hover(|s| s.bg(theme.sidebar_accent))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.select_pr(pr_id.clone(), cx);
            }))
            .child(
                v_flex()
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .truncate()
                                    .child(format!(
                                        "{}/{}",
                                        pr.repository.project, pr.repository.name
                                    )),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(age_color)
                                            .child(age),
                                    )
                                    .children(build_icon),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .truncate()
                            .when(pr.is_draft, |el| {
                                el.text_color(theme.muted_foreground)
                            })
                            .child(pr.title.clone()),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(pr.author.display_name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(vote_summary),
                            ),
                    ),
            )
    }

    fn render_detail_panel(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let theme = cx.theme();
        let pr_id = self.selected_pr.as_ref()?;
        let pr = self
            .reviewing
            .iter()
            .chain(self.authored.iter())
            .find(|p| &p.id == pr_id)?;

        Some(self.render_pr_detail(pr, cx))
    }

    fn render_pr_detail(&self, pr: &PullRequest, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let detail = self.selected_detail.as_ref();
        let web_url = pr.web_url.clone();

        v_flex()
            .id("pr-detail")
            .w(px(DETAIL_PANEL_WIDTH))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .p_4()
            .gap_3()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .child(pr.title.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("by {}", pr.author.display_name))
                            .child("·")
                            .child(format!(
                                "{}/{}",
                                pr.repository.project, pr.repository.name
                            ))
                            .child("·")
                            .child(format!(
                                "{} → {}",
                                short_branch(&pr.source_branch),
                                short_branch(&pr.target_branch)
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("Created {}", relative_time(&pr.created_at)))
                            .when(pr.is_draft, |el| {
                                el.child(
                                    div()
                                        .px_1()
                                        .rounded(px(4.))
                                        .bg(theme.muted)
                                        .child("Draft"),
                                )
                            })
                            .when_some(pr.merge_status.as_ref(), |el, status| {
                                el.child(merge_status_label(status))
                            }),
                    ),
            )
            .child(
                Button::new("open-browser")
                    .ghost()
                    .small()
                    .icon(Icon::new(IconName::ExternalLink).size_3())
                    .child("Open in Browser")
                    .on_click(cx.listener(move |_, _, _window, _cx| {
                        let _ = open::that(&web_url);
                    })),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Reviewers"),
                    )
                    .children(pr.reviewers.iter().map(|r| {
                        h_flex()
                            .gap_2()
                            .py(px(2.))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(vote_color(&r.vote, theme))
                                    .child(r.vote.symbol()),
                            )
                            .child(div().text_sm().child(r.user.display_name.clone()))
                            .when(r.is_required, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child("(required)"),
                                )
                            })
                    })),
            )
            .when_some(pr.build_status.as_ref(), |el, status| {
                el.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Build:"),
                        )
                        .child(build_status_label(status, theme)),
                )
            })
            .when_some(detail, |el, detail| {
                if detail.policies.is_empty() {
                    el
                } else {
                    el.child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Policies"),
                            )
                            .children(detail.policies.iter().map(|p| {
                                h_flex()
                                    .gap_2()
                                    .py(px(2.))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(policy_color(&p.status, theme))
                                            .child(policy_symbol(&p.status)),
                                    )
                                    .child(div().text_sm().child(p.name.clone()))
                                    .when(p.is_blocking, |el| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("(blocking)"),
                                        )
                                    })
                            })),
                    )
                }
            })
            .when_some(pr.description.as_ref(), |el, desc| {
                if desc.trim().is_empty() {
                    el
                } else {
                    el.child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Description"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(desc.clone()),
                            ),
                    )
                }
            })
    }

    fn render_status_bar(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let left = if let Some(msg) = &self.status_message {
            msg.clone()
        } else if let Some(last) = &self.last_poll {
            format!("Last poll: {} ago", relative_time(last))
        } else {
            "Not polled yet".into()
        };

        let reviewing_count = self.reviewing.len();
        let authored_count = self.authored.len();
        let error_count = self.errors.len();

        let mut right_parts = vec![
            format!("{reviewing_count} reviewing"),
            format!("{authored_count} authored"),
        ];
        if error_count > 0 {
            right_parts.push(format!("{error_count} errors"));
        }
        let right = right_parts.join(" · ");

        StatusBar::new()
            .child(div().text_xs().child(left))
            .child(div().text_xs().child(right))
    }
}

impl Render for Ridgeline {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .size_full()
            .track_focus(&self.focus)
            .key_context("Ridgeline")
            .on_action(cx.listener(Self::refresh))
            .on_action(cx.listener(Self::open_settings))
            .on_action(cx.listener(Self::open_in_browser))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::previous_tab))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| {
                crate::palette::open(this, window, cx);
            }))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_title_bar(cx))
                    .child(self.render_main_content(cx))
                    .child(self.render_status_bar(cx)),
            )
    }
}

fn short_branch(branch: &str) -> &str {
    branch.strip_prefix("refs/heads/").unwrap_or(branch)
}

fn vote_summary_text(reviewers: &[Reviewer]) -> String {
    if reviewers.is_empty() {
        return String::new();
    }
    let approved = reviewers
        .iter()
        .filter(|r| matches!(r.vote, Vote::Approved | Vote::ApprovedWithSuggestions))
        .count();
    let rejected = reviewers.iter().filter(|r| r.vote == Vote::Rejected).count();
    let waiting = reviewers
        .iter()
        .filter(|r| r.vote == Vote::WaitingForAuthor)
        .count();

    let mut parts = Vec::new();
    if approved > 0 {
        parts.push(format!("{approved}✓"));
    }
    if rejected > 0 {
        parts.push(format!("{rejected}✗"));
    }
    if waiting > 0 {
        parts.push(format!("{waiting}⏳"));
    }
    parts.join(" ")
}

fn vote_color(vote: &Vote, theme: &gpui_kit::component::Theme) -> Hsla {
    match vote {
        Vote::Approved | Vote::ApprovedWithSuggestions => gpui_kit::green(),
        Vote::Rejected => gpui_kit::red(),
        Vote::WaitingForAuthor => gpui_kit::yellow(),
        Vote::NoVote => theme.muted_foreground,
    }
}

fn build_status_icon(status: Option<&BuildStatus>) -> Option<impl IntoElement> {
    let (icon_name, color) = match status? {
        BuildStatus::Succeeded => (IconName::CircleCheck, gpui_kit::green()),
        BuildStatus::Failed => (IconName::CircleX, gpui_kit::red()),
        BuildStatus::InProgress => (IconName::Loader, gpui_kit::yellow()),
        BuildStatus::NotStarted => return None,
    };
    Some(Icon::new(icon_name).size(px(12.)).text_color(color))
}

fn build_status_label(
    status: &BuildStatus,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let (label, color) = match status {
        BuildStatus::Succeeded => ("Succeeded", gpui_kit::green()),
        BuildStatus::Failed => ("Failed", gpui_kit::red()),
        BuildStatus::InProgress => ("In Progress", gpui_kit::yellow()),
        BuildStatus::NotStarted => ("Not Started", theme.muted_foreground),
    };
    div().text_sm().text_color(color).child(label)
}

fn merge_status_label(status: &MergeStatus) -> impl IntoElement {
    let label = match status {
        MergeStatus::Succeeded => "Merge OK",
        MergeStatus::Conflicts => "Conflicts",
        MergeStatus::RejectedByPolicy => "Blocked",
        MergeStatus::NotSet => "Not set",
        MergeStatus::Queued => "Queued",
    };
    div().text_xs().child(label)
}

fn policy_symbol(status: &PolicyEvaluation) -> &'static str {
    match status {
        PolicyEvaluation::Approved => "✓",
        PolicyEvaluation::Rejected => "✗",
        PolicyEvaluation::Running => "⟳",
        PolicyEvaluation::Queued => "·",
        PolicyEvaluation::NotApplicable => "-",
    }
}

fn policy_color(status: &PolicyEvaluation, theme: &gpui_kit::component::Theme) -> Hsla {
    match status {
        PolicyEvaluation::Approved => gpui_kit::green(),
        PolicyEvaluation::Rejected => gpui_kit::red(),
        PolicyEvaluation::Running | PolicyEvaluation::Queued => gpui_kit::yellow(),
        PolicyEvaluation::NotApplicable => theme.muted_foreground,
    }
}
