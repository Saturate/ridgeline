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

use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tooltip::Tooltip;
use ridgeline_core::config::model::{Config, ProviderConfig, ProviderIndicator, TabConfig, TabDisplay, TabSource, WaitingOn};
use ridgeline_core::notifications::tracker::ChangeTracker;
use ridgeline_core::polling::poller::Poller;
use ridgeline_core::providers::traits::PrProvider;
use ridgeline_core::providers::types::*;
use ridgeline_core::util::time::relative_time;
use ridgeline_core::CachedBuildStatus;

use crate::pr_row::{group_label, initials, is_group_reviewer, parse_conventional_commit, parse_hex_color};
use crate::settings_window::SettingsWindow;
use crate::*;

const DETAIL_PANEL_WIDTH: f32 = 380.;

#[derive(Clone, PartialEq)]
enum LoadPhase {
    Connecting,
    FirstPoll,
    Ready,
    NoProviders,
    Failed(String),
}

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
    phase: LoadPhase,
    refreshing: bool,
    /// Providers that failed to connect, with the reason. Retried on refresh.
    offline: Vec<String>,

    change_tracker: ChangeTracker,
    build_cache: HashMap<String, CachedBuildStatus>,

    init_task: Option<Task<()>>,
    retry_task: Option<Task<()>>,
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
        crate::settings_window::remember_config_mtime(cx);
        cx.observe_global::<crate::settings_window::AppConfig>(Self::apply_config)
            .detach();
        // A replaced token doesn't change the config, but the providers were
        // built with the old one baked in.
        cx.observe_global::<crate::settings_window::CredentialsVersion>(|this, cx| {
            this.start_init(false, cx);
            cx.notify();
        })
        .detach();
        // A stat every couple of seconds is cheaper than a filesystem-watcher
        // dependency, and config edits don't need to land faster than that.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let alive = this
                .update(cx, |_, cx| crate::settings_window::reload_config_if_changed(cx))
                .is_ok();
            if !alive {
                break;
            }
        })
        .detach();

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
            status_message: None,
            phase: LoadPhase::Connecting,
            refreshing: false,
            offline: Vec::new(),
            change_tracker: ChangeTracker::new(),
            build_cache: HashMap::new(),
            init_task: None,
            retry_task: None,
            poll_task: None,
            detail_task: None,
        };

        this.start_init(false, cx);
        this
    }

    /// Settings edits land in the `AppConfig` global; mirror them here so the
    /// main window doesn't need a restart to see new tabs or providers.
    fn apply_config(&mut self, cx: &mut Context<Self>) {
        let Some(new) = cx
            .try_global::<crate::settings_window::AppConfig>()
            .map(|c| c.0.clone())
        else {
            return;
        };
        let providers_changed = serde_json::to_string(&new.providers).ok()
            != serde_json::to_string(&self.config.providers).ok();
        self.config = new;

        let tab_count = self.enabled_tabs().len();
        if self.active_tab >= tab_count {
            self.active_tab = tab_count.saturating_sub(1);
        }

        if providers_changed {
            self.poll_task = None;
            self.providers.clear();
            self.reviewing.clear();
            self.authored.clear();
            self.build_cache.clear();
            self.phase = LoadPhase::Connecting;
            self.start_init(false, cx);
        }
        cx.notify();
    }

    /// Connects every configured provider. With `keep_current`, a reconnect
    /// that fails everywhere leaves the existing providers polling (a refresh
    /// while the network is down shouldn't stop everything); otherwise the
    /// old poll task is stopped first so a replaced token can't keep working.
    fn start_init(&mut self, keep_current: bool, cx: &mut Context<Self>) {
        if !keep_current {
            // Clearing providers too, so a refresh during this reconnect
            // can't restart polling with the old (possibly replaced) token.
            self.poll_task = None;
            self.providers.clear();
        }
        self.retry_task = None;
        let config = self.config.clone();
        self.init_task = Some(cx.spawn(async move |this, cx| {
            let providers_result = io(async move {
                Ok(ridgeline_core::init_providers(&config).await)
            })
            .await;

            let results = match providers_result {
                Ok(results) => results,
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.refreshing = false;
                        this.phase = LoadPhase::Failed(e.to_string());
                        this.schedule_retry(cx);
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };

            let mut health = crate::settings_window::ProviderHealth::default();
            let mut failures: Vec<String> = Vec::new();
            let mut providers: Vec<(Arc<dyn PrProvider>, UserId)> = Vec::new();
            for init in results {
                match init.result {
                    Ok((provider, user)) => {
                        health.0.insert(init.name, Ok(user.display_name.clone()));
                        providers.push((provider, user));
                    }
                    Err(e) => {
                        failures.push(format!("{}: {e}", init.name));
                        health.0.insert(init.name, Err(e));
                    }
                }
            }

            this.update(cx, |this, cx| {
                let mut providers = providers;
                if keep_current {
                    // Per provider: one that's already polling and failed to
                    // reconnect (a blip during refresh) keeps its working
                    // connection and isn't reported offline.
                    let mut still_offline = Vec::new();
                    for failure in failures.drain(..) {
                        let name = failure.split(": ").next().unwrap_or_default();
                        match this.providers.iter().find(|(p, _)| p.name() == name) {
                            Some(existing) => {
                                health.0.remove(name);
                                providers.push(existing.clone());
                            }
                            None => still_offline.push(failure),
                        }
                    }
                    failures = still_offline;
                    if let Some(previous) =
                        cx.try_global::<crate::settings_window::ProviderHealth>()
                    {
                        for (name, state) in &previous.0 {
                            health.0.entry(name.clone()).or_insert_with(|| state.clone());
                        }
                    }
                }
                cx.set_global(health);
                cx.refresh_windows();
                this.offline = failures.clone();

                if this.config.providers.is_empty() {
                    this.providers.clear();
                    this.phase = LoadPhase::NoProviders;
                    this.refreshing = false;
                } else if !providers.is_empty() {
                    this.providers = providers;
                    // Re-connecting from a refresh keeps the current list on
                    // screen instead of flashing skeletons.
                    if this.phase != LoadPhase::Ready {
                        this.phase = LoadPhase::FirstPoll;
                    }
                    this.start_polling(cx);
                } else {
                    this.providers.clear();
                    this.phase = LoadPhase::Failed(failures.join("\n"));
                    this.refreshing = false;
                    this.schedule_retry(cx);
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// Nothing is polling after a total connect failure, so try again on the
    /// normal interval instead of waiting for a manual refresh.
    fn schedule_retry(&mut self, cx: &mut Context<Self>) {
        let secs = self.config.general.refresh_interval_secs.max(10);
        self.retry_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(secs)).await;
            this.update(cx, |this, cx| this.start_init(false, cx)).ok();
        }));
    }

    fn start_polling(&mut self, cx: &mut Context<Self>) {
        let providers = self.providers.clone();

        self.poll_task = Some(cx.spawn(async move |this, cx| {
            let mut first_poll = true;
            loop {
                // Cloned, not taken: a refresh drops this future mid-poll and
                // the cache would go with it.
                let (cache, previous_threads) = this
                    .update(cx, |this, _| {
                        let threads: HashMap<PrId, u32> = this
                            .authored
                            .iter()
                            .map(|pr| (pr.id.clone(), pr.open_threads))
                            .collect();
                        (this.build_cache.clone(), threads)
                    })
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
                        // Provider health comes from the list queries only;
                        // a thread fetch failing for one PR doesn't make the
                        // whole organization unreachable.
                        let mut provider_errors: HashMap<String, String> = HashMap::new();
                        for e in &result.errors {
                            provider_errors
                                .entry(e.provider.clone())
                                .or_insert_with(|| e.message.clone());
                        }
                        // Every list query failed: there's nothing new to show.
                        let all_failed = !providers.is_empty()
                            && providers.iter().all(|(p, _)| {
                                result.failed_reviewing.iter().any(|n| n == p.name())
                                    && result.failed_authored.iter().any(|n| n == p.name())
                            });
                        let thread_errors = ridgeline_core::enrich_open_threads(
                            &mut result.authored,
                            &providers,
                            &previous_threads,
                        )
                        .await;
                        result.errors.extend(thread_errors);
                        Ok((result, cache, all_failed, provider_errors))
                    }
                })
                .await;

                match poll_result {
                    Ok((mut result, cache, all_failed, provider_errors)) => {
                        this.update(cx, |this, cx| {
                            // A failed query means that provider's slice of the
                            // list is unknown, not empty. Keep the previous PRs
                            // for it so they don't vanish from the tab and then
                            // come back as "new PR" notifications.
                            let keep = |previous: &[PullRequest], failed: &[String]| {
                                previous
                                    .iter()
                                    .filter(|pr| failed.contains(&pr.id.provider))
                                    .cloned()
                                    .collect::<Vec<_>>()
                            };
                            result
                                .reviewing
                                .extend(keep(&this.reviewing, &result.failed_reviewing));
                            result
                                .authored
                                .extend(keep(&this.authored, &result.failed_authored));

                            let mut health = cx
                                .try_global::<crate::settings_window::ProviderHealth>()
                                .cloned()
                                .unwrap_or_default();
                            for (provider, user) in &this.providers {
                                health.0.insert(
                                    provider.name().to_string(),
                                    match provider_errors.get(provider.name()) {
                                        Some(e) => Err(e.clone()),
                                        None => Ok(user.display_name.clone()),
                                    },
                                );
                            }
                            cx.set_global(health);
                            cx.refresh_windows();

                            this.build_cache = cache;
                            this.refreshing = false;
                            if all_failed {
                                let reason = result
                                    .errors
                                    .iter()
                                    .map(|e| format!("{}: {}", e.provider, e.message))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                if this.phase == LoadPhase::Ready {
                                    this.status_message = Some(
                                        "Couldn't reach any provider; showing the last results"
                                            .into(),
                                    );
                                } else {
                                    this.phase = LoadPhase::Failed(reason);
                                }
                                this.errors = result.errors;
                                cx.notify();
                                return;
                            }

                            let all_prs: Vec<_> = result
                                .reviewing
                                .iter()
                                .chain(result.authored.iter())
                                .cloned()
                                .collect();
                            let changes = this.change_tracker.detect_changes(&all_prs);
                            if !first_poll && this.config.general.notifications_enabled {
                                let notif_config = this.config.general.notifications.clone();
                                for change in &changes {
                                    if ridgeline_core::should_notify(change, &notif_config) {
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
                            this.last_poll = Some(chrono::Utc::now());
                            this.status_message = None;
                            this.phase = LoadPhase::Ready;
                            first_poll = false;
                            cx.notify();
                        })
                        .ok();
                    }
                    Err(e) => {
                        this.update(cx, |this, cx| {
                            this.refreshing = false;
                            // Keep showing the last good data; only an empty
                            // window turns into an error screen.
                            if this.phase == LoadPhase::Ready {
                                this.status_message = Some(format!("Poll error: {e}"));
                            } else {
                                this.phase = LoadPhase::Failed(e.to_string());
                            }
                            cx.notify();
                        })
                        .ok();
                    }
                }

                // Sleep in short steps and re-read the interval each time, so
                // lowering it in settings takes effect without waiting out the
                // old (possibly 10 minute) sleep or restarting the poll.
                let mut waited = 0u64;
                loop {
                    let Ok(interval) = this
                        .update(cx, |this, _| this.config.general.refresh_interval_secs.max(10))
                    else {
                        return;
                    };
                    if waited >= interval {
                        break;
                    }
                    let step = (interval - waited).min(5);
                    cx.background_executor().timer(Duration::from_secs(step)).await;
                    waited += step;
                }
            }
        }));
    }

    fn refresh(&mut self, _: &Refresh, _window: &mut Window, cx: &mut Context<Self>) {
        self.refreshing = true;
        if matches!(self.phase, LoadPhase::Failed(_)) {
            self.phase = LoadPhase::FirstPoll;
        }
        if self.providers.is_empty() || !self.offline.is_empty() {
            // Reconnect so a provider that failed at startup (network down,
            // expired `az login`) gets another chance. Whatever is connected
            // keeps polling until the reconnect succeeds.
            self.start_init(true, cx);
        } else {
            self.poll_task = None;
            self.start_polling(cx);
        }
        cx.notify();
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
        self.prs_for_tab(tab, true)
    }

    fn prs_for_tab(&self, tab: &TabConfig, apply_search: bool) -> Vec<&PullRequest> {
        // Configured projects pull your own PRs into the reviewing list too;
        // they belong under the authored tabs, and "Everything" lists each once.
        let authored_ids: std::collections::HashSet<&PrId> =
            self.authored.iter().map(|pr| &pr.id).collect();
        let source_prs: Vec<&PullRequest> = match tab.source {
            TabSource::Reviewing => self
                .reviewing
                .iter()
                .filter(|pr| !authored_ids.contains(&pr.id))
                .collect(),
            TabSource::Authored => self.authored.iter().collect(),
            TabSource::All => self
                .reviewing
                .iter()
                .filter(|pr| !authored_ids.contains(&pr.id))
                .chain(self.authored.iter())
                .collect(),
        };

        source_prs
            .into_iter()
            .filter(|pr| {
                // Thread counts only exist for authored PRs, so the filter
                // only means something there (the dialog hides it elsewhere).
                if let Some(waiting_on) = tab.filter.waiting_on.as_ref().filter(|_| tab.source == TabSource::Authored) {
                    let on_author = pr.is_waiting_on_author();
                    if on_author != (*waiting_on == WaitingOn::Author) {
                        return false;
                    }
                }
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
                if !tab.filter.cc_types.is_empty() {
                    let kind = parse_conventional_commit(&pr.title).map(|cc| cc.kind);
                    let wanted = kind.is_some_and(|k| {
                        tab.filter.cc_types.iter().any(|t| t.eq_ignore_ascii_case(k))
                    });
                    if !wanted {
                        return false;
                    }
                }
                if let Some(ref prefix) = tab.filter.branch_prefix {
                    if !pr
                        .source_branch
                        .to_lowercase()
                        .starts_with(&prefix.to_lowercase())
                    {
                        return false;
                    }
                }
                if apply_search && !self.search_text.is_empty() {
                    let q = self.search_text.trim().to_lowercase();
                    let matches = pr.title.to_lowercase().contains(&q)
                        || pr.author.display_name.to_lowercase().contains(&q)
                        || pr.repository.name.to_lowercase().contains(&q)
                        || pr.repository.project.to_lowercase().contains(&q)
                        || pr.source_branch.to_lowercase().contains(&q)
                        || q.trim_start_matches('#') == pr.id.number.to_string();
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
                            .loading(self.refreshing || self.is_loading())
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
                let display = tabs
                    .get(self.active_tab)
                    .map(|t| t.display.clone())
                    .unwrap_or(TabDisplay::Reviewing);
                match &self.phase {
                    LoadPhase::Connecting | LoadPhase::FirstPoll => {
                        list = list.children((0..6).map(|i| skeleton_row(i, cx)));
                    }
                    LoadPhase::NoProviders => {
                        list = list.child(self.render_list_state(
                            IconName::Inbox,
                            "No providers yet",
                            Some("Add an Azure DevOps organization to start tracking pull requests.".into()),
                            Some(("Open settings", true)),
                            cx,
                        ));
                    }
                    LoadPhase::Failed(e) => {
                        list = list.child(self.render_list_state(
                            IconName::CircleAlert,
                            "Couldn't load pull requests",
                            Some(e.clone()),
                            Some(("Try again", false)),
                            cx,
                        ));
                    }
                    LoadPhase::Ready if prs.is_empty() => {
                        let (title, detail) = self.empty_tab_text(tabs.get(self.active_tab).copied());
                        list = list.child(self.render_list_state(
                            if self.search_text.trim().is_empty() { IconName::Inbox } else { IconName::Search },
                            title,
                            detail,
                            None,
                            cx,
                        ));
                    }
                    LoadPhase::Ready => {
                        for pr in &prs {
                            list = list.child(self.render_pr_row(pr, &display, cx));
                        }
                    }
                }
                list
            })
    }

    fn is_loading(&self) -> bool {
        matches!(self.phase, LoadPhase::Connecting | LoadPhase::FirstPoll)
    }

    fn empty_tab_text(&self, tab: Option<&TabConfig>) -> (&'static str, Option<String>) {
        if !self.search_text.trim().is_empty() {
            return (
                "No matches",
                Some(format!("Nothing in this tab matches \"{}\".", self.search_text)),
            );
        }
        // waiting_on only applies to authored tabs (see prs_for_tab).
        let detail = match tab.map(|t| {
            let waiting_on = t.filter.waiting_on.as_ref().filter(|_| t.source == TabSource::Authored);
            (&t.source, waiting_on)
        }) {
            Some((_, Some(&WaitingOn::Author))) => "Nothing needs you right now.",
            Some((_, Some(&WaitingOn::Reviewers))) => "No PRs are waiting on reviewers.",
            Some((TabSource::Reviewing, None)) => "No reviews assigned to you.",
            Some((TabSource::Authored, None)) => "You have no open PRs.",
            _ => "No pull requests match this tab's filters.",
        };
        ("All clear", Some(detail.into()))
    }

    /// Centered message for the empty, error and no-provider states. The
    /// action is either "open settings" (true) or "retry" (false).
    fn render_list_state(
        &self,
        icon: IconName,
        title: &'static str,
        detail: Option<String>,
        action: Option<(&'static str, bool)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_2()
            .py(px(64.))
            .px_8()
            .child(Icon::new(icon).size_8().text_color(theme.muted_foreground))
            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
            .when_some(detail, |el, d| {
                el.child(
                    div()
                        .max_w(px(420.))
                        .text_xs()
                        .text_center()
                        .text_color(theme.muted_foreground)
                        .child(d),
                )
            })
            .when_some(action, |el, (label, settings)| {
                el.child(
                    div().pt_2().child(
                        Button::new("list-state-action")
                            .outline()
                            .small()
                            .label(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if settings {
                                    this.open_settings(&OpenSettings, window, cx);
                                } else {
                                    this.refresh(&Refresh, window, cx);
                                }
                            })),
                    ),
                )
            })
    }

    fn count_for_tab(&self, tab: &TabConfig) -> usize {
        self.prs_for_tab(tab, false).len()
    }

    fn provider_color(&self, provider: &str) -> Option<Hsla> {
        self.config
            .providers
            .iter()
            .find_map(|p| match p {
                ProviderConfig::AzureDevOps(c) if c.name == provider => c.color.as_deref(),
                _ => None,
            })
            .and_then(parse_hex_color)
    }

    fn render_pr_row(
        &self,
        pr: &PullRequest,
        display: &TabDisplay,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let general = &self.config.general;
        let is_selected = self.selected_pr.as_ref() == Some(&pr.id);
        let pr_id = pr.id.clone();
        let web_url = pr.web_url.clone();
        let provider_color = self.provider_color(&pr.id.provider);
        let indicator = &general.provider_indicator;
        let cc = general
            .parse_conventional_commits
            .then(|| parse_conventional_commit(&pr.title))
            .flatten();
        let title = cc.as_ref().map_or(pr.title.as_str(), |c| c.description).to_string();
        let repo_label = if general.show_project_name {
            format!("{}/{}", pr.repository.project, pr.repository.name)
        } else {
            pr.repository.name.clone()
        };
        let is_reviewing = *display == TabDisplay::Reviewing;
        let row_id = SharedString::from(format!(
            "pr-{}-{}-{}",
            pr.id.provider, pr.id.repository, pr.id.number
        ));

        let title_line = h_flex()
            .gap_2()
            .min_w_0()
            .when_some(
                provider_color.filter(|_| *indicator == ProviderIndicator::Badge),
                |el, color| {
                    el.child(
                        div()
                            .flex_none()
                            .px(px(6.))
                            .rounded(px(4.))
                            .text_size(px(10.))
                            .font_weight(FontWeight::MEDIUM)
                            .bg(color)
                            .text_color(gpui_kit::white())
                            .child(pr.id.provider.clone()),
                    )
                },
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .truncate()
                    .child(title),
            )
            .when(pr.is_draft, |el| {
                el.child(
                    h_flex()
                        .flex_none()
                        .gap_1()
                        .px_1()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(theme.border)
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(Icon::new(Lucide::FilePen).size(px(12.)))
                        .child("Draft"),
                )
            });

        let cc_line = cc.as_ref().map(|cc| {
            let kind_color = commit_type_color(cc.kind, theme);
            h_flex()
                .gap_1()
                .child(pill(cc.kind.to_string(), kind_color, theme).font_weight(FontWeight::MEDIUM))
                .when_some(cc.scope, |el, scope| {
                    el.child(pill(scope.to_string(), None, theme))
                })
                .when(cc.breaking, |el| {
                    el.child(
                        pill("breaking".into(), Some(theme.red), theme)
                            .font_weight(FontWeight::MEDIUM),
                    )
                })
        });

        let meta_line = h_flex()
            .gap_2()
            .min_w_0()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .flex_none()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground.opacity(0.7))
                    .child(repo_label),
            )
            .child(div().flex_none().child(format!("#{}", pr.id.number)))
            .when(is_reviewing, |el| {
                el.child(
                    div()
                        .flex_none()
                        .child(format!("by {}", pr.author.display_name)),
                )
            })
            .child(
                h_flex()
                    .id(SharedString::from(format!("{row_id}-branch")))
                    .min_w_0()
                    .gap_1()
                    .child(Icon::new(Lucide::GitBranch).size(px(12.)))
                    .child(div().truncate().child(short_branch(&pr.source_branch).to_string()))
                    .tooltip({
                        let text = format!(
                            "{} → {}",
                            short_branch(&pr.source_branch),
                            short_branch(&pr.target_branch)
                        );
                        move |window, cx| Tooltip::new(text.clone()).build(window, cx)
                    }),
            );

        let mut right = h_flex().flex_none().items_center().gap_2();
        if is_reviewing {
            if pr.reviewers.iter().any(|r| r.vote == Vote::WaitingForAuthor) {
                right = right.child(outline_badge(
                    Lucide::Clock,
                    "Waiting for author".into(),
                    theme.yellow,
                ));
            }
            right = right.child(h_flex().gap(px(2.)).children(
                pr.reviewers
                    .iter()
                    .enumerate()
                    .map(|(i, r)| vote_chip(r, format!("{row_id}-vote-{i}"), theme)),
            ));
        } else {
            right = right.child(status_summary(pr, theme));
        }
        right = right
            .children(open_threads_badge(pr.open_threads, theme))
            .children(merge_status_badge(pr.merge_status.as_ref(), theme))
            .children(build_status_badge(pr.build_status.as_ref(), theme))
            .child(self.age_indicator(pr, is_reviewing, theme))
            .child(
                Button::new(SharedString::from(format!("{row_id}-open")))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::ExternalLink).size(px(14.)))
                    .tooltip("Open in browser")
                    .on_click(cx.listener(move |_, _, _window, cx| {
                        cx.stop_propagation();
                        let _ = open::that(&web_url);
                    })),
            );

        h_flex()
            .id(row_id.clone())
            .w_full()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .relative()
            .cursor_pointer()
            // GPUI has no per-side border color, so the provider stripe is an overlay.
            .when_some(
                provider_color.filter(|_| *indicator == ProviderIndicator::Border),
                |el, color| {
                    el.child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .bottom_0()
                            .w(px(3.))
                            .bg(color),
                    )
                },
            )
            .when(is_selected, |el| el.bg(theme.sidebar_primary))
            .hover(|s| s.bg(theme.sidebar_accent))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.select_pr(pr_id.clone(), cx);
            }))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(title_line)
                    .children(cc_line)
                    .child(meta_line),
            )
            .child(right)
    }

    fn age_indicator(
        &self,
        pr: &PullRequest,
        is_reviewing: bool,
        theme: &gpui_kit::component::Theme,
    ) -> impl IntoElement {
        let age_hours = chrono::Utc::now()
            .signed_duration_since(pr.created_at)
            .num_hours()
            .max(0) as u64;
        let color = if age_hours >= self.config.general.age_danger_hours {
            theme.red
        } else if age_hours >= self.config.general.age_warning_hours {
            theme.warning
        } else {
            theme.muted_foreground
        };
        let created = pr.created_at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M");
        let tooltip = if is_reviewing {
            format!("Waiting for review since {created}")
        } else {
            format!("Open since {created}")
        };
        h_flex()
            .id(SharedString::from(format!(
                "age-{}-{}-{}",
                pr.id.provider, pr.id.repository, pr.id.number
            )))
            .gap_1()
            .text_xs()
            .text_color(color)
            .child(
                Icon::new(if is_reviewing { Lucide::Hourglass } else { Lucide::Clock })
                    .size(px(12.)),
            )
            .child(relative_time(&pr.created_at))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
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
        let busy = self.refreshing || self.is_loading();
        let left = if let Some(msg) = &self.status_message {
            msg.clone()
        } else {
            match (&self.phase, &self.last_poll) {
                (LoadPhase::Connecting, _) => "Connecting to providers…".into(),
                (LoadPhase::FirstPoll, _) => "Loading pull requests…".into(),
                (LoadPhase::NoProviders, _) => "No providers configured".into(),
                (LoadPhase::Failed(_), _) => "Couldn't load pull requests".into(),
                _ if self.refreshing => "Refreshing…".into(),
                (_, Some(last)) => match relative_time(last).as_str() {
                    "now" => "Updated just now".into(),
                    t => format!("Updated {t} ago"),
                },
                (_, None) => String::new(),
            }
        };

        // Same rule as the Reviewing tab: your own PRs count as authored.
        let reviewing_count = self
            .reviewing
            .iter()
            .filter(|pr| !self.authored.iter().any(|a| a.id == pr.id))
            .count();
        let authored_count = self.authored.len();
        let error_count = self.errors.len();

        let mut right_parts = vec![
            format!("{reviewing_count} reviewing"),
            format!("{authored_count} authored"),
        ];
        if !self.offline.is_empty() {
            right_parts.push(format!("{} offline", self.offline.len()));
        }
        if error_count > 0 {
            right_parts.push(format!("{error_count} errors"));
        }
        // Zero counts during the first load read as "you have nothing".
        let right = if self.phase == LoadPhase::Ready {
            right_parts.join(" · ")
        } else {
            String::new()
        };

        StatusBar::new()
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .when(busy, |el| el.child(Spinner::new().xsmall()))
                    .child(div().text_xs().child(left)),
            )
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

fn vote_color(vote: &Vote, theme: &gpui_kit::component::Theme) -> Hsla {
    match vote {
        Vote::Approved | Vote::ApprovedWithSuggestions => gpui_kit::green(),
        Vote::Rejected => gpui_kit::red(),
        Vote::WaitingForAuthor => gpui_kit::yellow(),
        Vote::NoVote => theme.muted_foreground,
    }
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

/// Placeholder shaped like a PR row (title, pills, meta line, chips) so the
/// list doesn't jump when real rows arrive.
fn skeleton_row(i: usize, cx: &App) -> impl IntoElement {
    // Vary widths a little so the rows don't read as a striped table.
    let title_w = [0.55, 0.42, 0.62, 0.38, 0.5, 0.46][i % 6];
    h_flex()
        .px_4()
        .py_3()
        .gap_4()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            v_flex()
                .flex_1()
                .gap_2()
                .child(Skeleton::new().h_4().w(relative(title_w)).rounded_sm())
                .child(
                    h_flex()
                        .gap_1()
                        .child(Skeleton::new().secondary().h_4().w(px(36.)).rounded_sm())
                        .child(Skeleton::new().secondary().h_4().w(px(72.)).rounded_sm()),
                )
                .child(Skeleton::new().secondary().h_3().w(relative(title_w * 0.9)).rounded_sm()),
        )
        .child(
            h_flex()
                .gap_2()
                .child(Skeleton::new().secondary().h_5().w(px(48.)).rounded_full())
                .child(Skeleton::new().secondary().h_4().w(px(28.)).rounded_sm()),
        )
}

type ThemeRef<'a> = &'a gpui_kit::component::Theme;

/// Small rounded label; `tint` colors text and a 20% background, `None` uses muted.
fn pill(label: String, tint: Option<Hsla>, theme: ThemeRef) -> Div {
    let (bg, fg) = match tint {
        Some(c) => (c.opacity(0.2), c),
        None => (theme.muted, theme.muted_foreground),
    };
    div()
        .flex_none()
        .px(px(6.))
        .rounded(px(4.))
        .text_size(px(10.))
        .bg(bg)
        .text_color(fg)
        .child(label)
}

fn commit_type_color(kind: &str, theme: ThemeRef) -> Option<Hsla> {
    match kind {
        "feat" => Some(theme.green),
        "fix" => Some(theme.red),
        "refactor" => Some(theme.magenta),
        "perf" => Some(theme.warning),
        "docs" => Some(theme.blue),
        "test" => Some(theme.cyan),
        _ => None,
    }
}

fn outline_badge(icon: Lucide, label: SharedString, color: Hsla) -> impl IntoElement {
    h_flex()
        .flex_none()
        .gap_1()
        .px_2()
        .py(px(1.))
        .rounded_full()
        .border_1()
        .border_color(color.opacity(0.5))
        .text_xs()
        .text_color(color)
        .child(Icon::new(icon).size(px(12.)))
        .child(label)
}

fn solid_badge(icon: Option<Lucide>, label: SharedString, bg: Hsla, fg: Hsla) -> Stateful<Div> {
    h_flex()
        .id(label.clone())
        .flex_none()
        .gap_1()
        .px_2()
        .py(px(1.))
        .rounded_full()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .bg(bg)
        .text_color(fg)
        .children(icon.map(|i| Icon::new(i).size(px(12.))))
        .child(label)
}

fn vote_chip(reviewer: &Reviewer, id: String, theme: ThemeRef) -> impl IntoElement {
    let (icon, color) = match reviewer.vote {
        Vote::Approved | Vote::ApprovedWithSuggestions => (Lucide::Check, Some(theme.green)),
        Vote::Rejected => (Lucide::X, Some(theme.red)),
        Vote::WaitingForAuthor => (Lucide::Clock, Some(theme.yellow)),
        Vote::NoVote => (Lucide::Minus, None),
    };
    let (bg, fg) = match color {
        Some(c) if reviewer.vote == Vote::ApprovedWithSuggestions => (c.opacity(0.1), c),
        Some(c) => (c.opacity(0.2), c),
        None => (theme.muted, theme.muted_foreground),
    };
    let name = &reviewer.user.display_name;
    let group = is_group_reviewer(name);
    let tooltip = format!(
        "{}{}",
        if group { group_label(name) } else { name },
        if reviewer.is_required { " (required)" } else { "" }
    );

    h_flex()
        .id(SharedString::from(id))
        .flex_none()
        .h(px(22.))
        .gap(px(2.))
        .px(px(6.))
        .rounded_full()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .bg(bg)
        .text_color(fg)
        .when(reviewer.is_required, |el| el.border_1().border_color(fg.opacity(0.3)))
        .child(Icon::new(icon).size(px(12.)))
        .map(|el| {
            if group {
                el.child(Icon::new(Lucide::Users).size(px(12.)))
            } else {
                el.child(initials(name))
            }
        })
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
}

/// Authored-tab summary, mirroring the Tauri `PrStatusSummary` priority order.
fn status_summary(pr: &PullRequest, theme: ThemeRef) -> AnyElement {
    let total = pr.reviewers.len();
    if total == 0 {
        return outline_badge(Lucide::UserX, "No reviewers".into(), theme.warning).into_any_element();
    }
    let count = |f: fn(&Vote) -> bool| pr.reviewers.iter().filter(|r| f(&r.vote)).count();
    let approved = count(|v| matches!(v, Vote::Approved | Vote::ApprovedWithSuggestions));
    let rejected = count(|v| *v == Vote::Rejected);
    let waiting = count(|v| *v == Vote::WaitingForAuthor);

    if rejected > 0 {
        solid_badge(
            Some(Lucide::TriangleAlert),
            format!("{rejected} rejected").into(),
            theme.danger,
            theme.danger_foreground,
        )
        .into_any_element()
    } else if waiting > 0 {
        outline_badge(Lucide::Clock, "Waiting for you".into(), theme.yellow).into_any_element()
    } else if approved == total {
        solid_badge(
            Some(Lucide::CircleCheck),
            "All approved".into(),
            theme.success,
            theme.success_foreground,
        )
        .into_any_element()
    } else {
        solid_badge(
            Some(Lucide::CircleCheck),
            format!("{approved}/{total}").into(),
            theme.secondary,
            theme.secondary_foreground,
        )
        .into_any_element()
    }
}

fn open_threads_badge(count: u32, theme: ThemeRef) -> Option<impl IntoElement> {
    if count == 0 {
        return None;
    }
    let label = if count == 1 { "1 comment".to_string() } else { format!("{count} comments") };
    Some(
        div()
            .id("open-threads")
            .child(outline_badge(Lucide::MessageSquare, label.into(), theme.warning))
            .tooltip(|window, cx| {
                Tooltip::new("Unresolved comment threads from reviewers").build(window, cx)
            }),
    )
}

fn merge_status_badge(status: Option<&MergeStatus>, theme: ThemeRef) -> Option<impl IntoElement> {
    let (icon, label, tip) = match status? {
        MergeStatus::Conflicts => (Lucide::GitMerge, "Conflicts", "This PR has merge conflicts"),
        MergeStatus::RejectedByPolicy => {
            (Lucide::TriangleAlert, "Policy blocked", "Blocked by branch policy")
        }
        _ => return None,
    };
    Some(
        solid_badge(Some(icon), label.into(), theme.danger, theme.danger_foreground)
            .tooltip(move |window, cx| Tooltip::new(tip).build(window, cx)),
    )
}

fn build_status_badge(status: Option<&BuildStatus>, theme: ThemeRef) -> Option<impl IntoElement> {
    match status? {
        BuildStatus::InProgress => Some(solid_badge(
            Some(Lucide::Loader),
            "Building".into(),
            theme.secondary,
            theme.secondary_foreground,
        )),
        BuildStatus::Failed => Some(solid_badge(
            None,
            "Build failed".into(),
            theme.danger,
            theme.danger_foreground,
        )),
        BuildStatus::Succeeded | BuildStatus::NotStarted => None,
    }
}
