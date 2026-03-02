use std::sync::Arc;
use std::time::Duration;

use async_compat::CompatExt;
use iocraft::prelude::*;

use crate::notifications::desktop;
use crate::notifications::tracker::ChangeTracker;
use crate::polling::poller::Poller;
use crate::state::app_state::{ActiveTab, ActiveView, EnrichedPr};
use crate::ui::components::notification_toast::NotificationToast;
use crate::ui::components::status_bar::StatusBar;
use crate::ui::keybindings::{self, Action};
use crate::ui::theme::Theme;
use crate::ui::views::dashboard::Dashboard;
use crate::ui::views::detail::DetailView;
use crate::ui::views::help::HelpOverlay;
use crate::util::time::relative_time;

#[derive(Props)]
pub struct AppProps {
    pub poller: Arc<Poller>,
    pub refresh_interval_secs: u64,
    pub stale_threshold_hours: u64,
    pub notifications_enabled: bool,
    pub provider_names: Vec<String>,
}

impl Default for AppProps {
    fn default() -> Self {
        Self {
            poller: Arc::new(Poller::new(Vec::new())),
            refresh_interval_secs: 60,
            stale_threshold_hours: 48,
            notifications_enabled: false,
            provider_names: Vec::new(),
        }
    }
}

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();

    // Scalar/Copy state — use .get()
    let mut active_tab = hooks.use_state(|| ActiveTab::Reviewing);
    let mut active_view = hooks.use_state(|| ActiveView::Dashboard);
    let mut selected_index = hooks.use_state(|| 0usize);
    let mut scroll_offset = hooks.use_state(|| 0usize);
    let mut filter_active = hooks.use_state(|| false);
    let mut provider_filter = hooks.use_state(|| Option::<usize>::None);
    let mut loading = hooks.use_state(|| true);
    let mut should_exit = hooks.use_state(|| false);
    let mut quit_pending = hooks.use_state(|| false);
    let mut show_help = hooks.use_state(|| false);
    let mut toast_visible = hooks.use_state(|| false);
    let mut spinner_frame = hooks.use_state(|| 0usize);
    let mut force_refresh = hooks.use_state(|| false);

    // Non-Copy state — use .read() for access, .set() for mutation
    let mut reviewing = hooks.use_state(|| Vec::<EnrichedPr>::new());
    let mut authored = hooks.use_state(|| Vec::<EnrichedPr>::new());
    let mut filter_text = hooks.use_state(|| String::new());
    let mut last_refresh = hooks.use_state(|| String::new());
    let mut errors = hooks.use_state(|| Vec::<String>::new());
    let mut toast_msg = hooks.use_state(|| String::new());

    let stale_threshold = props.stale_threshold_hours;
    let provider_names = props.provider_names.clone();
    let notifications_enabled = props.notifications_enabled;

    // Background polling loop
    let poller = Arc::clone(&props.poller);
    let interval = props.refresh_interval_secs;
    hooks.use_future(async move {
        let mut tracker = ChangeTracker::new();
        let mut first_run = true;

        loop {
            loading.set(true);

            let result = poller.poll_once().compat().await;

            let new_reviewing: Vec<EnrichedPr> = result
                .reviewing
                .into_iter()
                .map(|pr| EnrichedPr::from_pr(pr, stale_threshold))
                .collect();

            let new_authored: Vec<EnrichedPr> = result
                .authored
                .into_iter()
                .map(|pr| EnrichedPr::from_pr(pr, stale_threshold))
                .collect();

            // Change detection for notifications
            let all_prs: Vec<_> = new_reviewing
                .iter()
                .chain(new_authored.iter())
                .map(|e| e.pr.clone())
                .collect();

            let changes = tracker.detect_changes(&all_prs);
            if !first_run && notifications_enabled {
                for change in &changes {
                    let _ = desktop::send_notification(
                        &change.notification_title(),
                        &change.notification_body(),
                    );
                }
            }

            reviewing.set(new_reviewing);
            authored.set(new_authored);

            let new_errors: Vec<String> =
                result.errors.iter().map(|e| e.to_string()).collect();
            errors.set(new_errors);

            last_refresh.set(relative_time(&chrono::Utc::now()));
            loading.set(false);
            first_run = false;

            // Wait for interval or force refresh
            let mut elapsed = 0u64;
            while elapsed < interval {
                smol::Timer::after(Duration::from_secs(1)).await;
                elapsed += 1;
                if force_refresh.get() {
                    force_refresh.set(false);
                    break;
                }
            }
        }
    });

    // Spinner animation
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(80)).await;
            spinner_frame.set(spinner_frame.get() + 1);
        }
    });

    // Toast auto-hide + quit confirmation reset
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_secs(3)).await;
            if toast_visible.get() {
                toast_visible.set(false);
                toast_msg.set(String::new());
            }
            if quit_pending.get() {
                quit_pending.set(false);
            }
        }
    });

    // Read state into local variables for rendering
    let reviewing_data: Vec<EnrichedPr> = reviewing.read().clone();
    let authored_data: Vec<EnrichedPr> = authored.read().clone();
    let filter_str = filter_text.to_string();
    let error_list: Vec<String> = errors.read().clone();

    let current_prs: &[EnrichedPr] = match active_tab.get() {
        ActiveTab::Reviewing => &reviewing_data,
        ActiveTab::Authored => &authored_data,
    };

    let prov_filter_name = provider_filter
        .get()
        .and_then(|i| provider_names.get(i))
        .cloned()
        .unwrap_or_default();

    let filtered: Vec<EnrichedPr> = {
        let refs = crate::state::filters::apply_filters(
            current_prs,
            &filter_str,
            if prov_filter_name.is_empty() {
                None
            } else {
                Some(prov_filter_name.as_str())
            },
        );
        refs.into_iter().cloned().collect()
    };

    let filtered_len = filtered.len();

    // Keyboard handling
    hooks.use_terminal_events({
        let provider_names = provider_names.clone();
        move |event| {
            if let TerminalEvent::Key(key_event) = event {
                let action = keybindings::map_key(&key_event, filter_active.get());

                match action {
                    Action::Quit => {
                        if quit_pending.get() {
                            should_exit.set(true);
                        } else {
                            quit_pending.set(true);
                            toast_msg.set("Press again to quit".to_string());
                            toast_visible.set(true);
                        }
                    }
                    Action::MoveDown => {
                        if filtered_len > 0 {
                            let new_idx = (selected_index.get() + 1).min(filtered_len - 1);
                            selected_index.set(new_idx);
                            let visible_h = height.saturating_sub(6) as usize;
                            if new_idx >= scroll_offset.get() + visible_h {
                                scroll_offset.set(new_idx.saturating_sub(visible_h - 1));
                            }
                        }
                    }
                    Action::MoveUp => {
                        let new_idx = selected_index.get().saturating_sub(1);
                        selected_index.set(new_idx);
                        if new_idx < scroll_offset.get() {
                            scroll_offset.set(new_idx);
                        }
                    }
                    Action::JumpTop => {
                        selected_index.set(0);
                        scroll_offset.set(0);
                    }
                    Action::JumpBottom => {
                        if filtered_len > 0 {
                            selected_index.set(filtered_len - 1);
                            let visible_h = height.saturating_sub(6) as usize;
                            if filtered_len > visible_h {
                                scroll_offset.set(filtered_len - visible_h);
                            }
                        }
                    }
                    Action::OpenDetail => {
                        if filtered_len > 0 {
                            active_view.set(ActiveView::Detail(selected_index.get()));
                        }
                    }
                    Action::OpenBrowser => {
                        let idx = selected_index.get();
                        let data = match active_tab.get() {
                            ActiveTab::Reviewing => reviewing.read(),
                            ActiveTab::Authored => authored.read(),
                        };
                        if idx < data.len() {
                            let _ = crate::util::browser::open_url(&data[idx].pr.web_url);
                        }
                    }
                    Action::SwitchTab => {
                        let new_tab = match active_tab.get() {
                            ActiveTab::Reviewing => ActiveTab::Authored,
                            ActiveTab::Authored => ActiveTab::Reviewing,
                        };
                        active_tab.set(new_tab);
                        selected_index.set(0);
                        scroll_offset.set(0);
                    }
                    Action::FocusSearch => {
                        filter_active.set(true);
                    }
                    Action::ForceRefresh => {
                        force_refresh.set(true);
                        toast_msg.set("Refreshing...".to_string());
                        toast_visible.set(true);
                    }
                    Action::ProviderFilter(n) => {
                        if n < provider_names.len() {
                            provider_filter.set(Some(n));
                            selected_index.set(0);
                            scroll_offset.set(0);
                        }
                    }
                    Action::ClearProviderFilter => {
                        provider_filter.set(None);
                        selected_index.set(0);
                        scroll_offset.set(0);
                    }
                    Action::ToggleHelp => {
                        show_help.set(!show_help.get());
                    }
                    Action::Back => {
                        if filter_active.get() {
                            filter_active.set(false);
                        } else if show_help.get() {
                            show_help.set(false);
                        } else if active_view.get() != ActiveView::Dashboard {
                            active_view.set(ActiveView::Dashboard);
                        } else {
                            filter_text.set(String::new());
                        }
                    }
                    Action::None => {}
                }
            }
        }
    });

    if should_exit.get() {
        system.exit();
    }

    let theme = Theme::dark();

    let last_error = error_list.last().cloned().unwrap_or_default();
    let error_count = error_list.len() as u32;
    let visible_height = (height as u32).saturating_sub(6);
    let reviewing_count = reviewing_data.len() as u32;
    let authored_count = authored_data.len() as u32;

    let content: AnyElement = match active_view.get() {
        ActiveView::Dashboard | ActiveView::Help => {
            element! {
                Dashboard(
                    filtered_prs: filtered.clone(),
                    active_tab: Some(active_tab.get()),
                    reviewing_count,
                    authored_count,
                    selected_index: selected_index.get() as u32,
                    scroll_offset: scroll_offset.get() as u32,
                    visible_height,
                    filter_text: filter_str.clone(),
                    filter_active: filter_active.get(),
                    theme,
                    on_filter_change: move |val: String| filter_text.set(val),
                )
            }
            .into()
        }
        ActiveView::Detail(idx) => {
            let detail_pr = filtered.get(idx).cloned();
            element! {
                DetailView(
                    pr: detail_pr,
                    theme,
                )
            }
            .into()
        }
    };

    element! {
        View(
            width,
            height,
            flex_direction: FlexDirection::Column,
        ) {
            View(flex_grow: 1.0, flex_direction: FlexDirection::Column) {
                #(std::iter::once(content))
            }
            StatusBar(
                last_refresh: last_refresh.to_string(),
                error_count,
                last_error,
                loading: loading.get(),
                spinner_frame: spinner_frame.get(),
                provider_filter: prov_filter_name.clone(),
                theme,
            )
            HelpOverlay(visible: show_help.get(), theme)
            NotificationToast(
                message: toast_msg.to_string(),
                visible: toast_visible.get(),
                theme,
            )
        }
    }
}
