use chrono::{DateTime, Utc};

use crate::providers::types::PullRequest;

/// Which tab is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Reviewing,
    Authored,
}

/// Which view is currently shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Dashboard,
    Detail(usize), // index into the current list
    Help,
}

/// PR enriched with computed display fields
#[derive(Debug, Clone)]
pub struct EnrichedPr {
    pub pr: PullRequest,
    pub age_display: String,
    pub is_stale: bool,
    pub vote_summary: String,
    pub source_display: String,
}

impl EnrichedPr {
    pub fn from_pr(pr: PullRequest, stale_threshold_hours: u64) -> Self {
        let age_display = crate::util::time::relative_time(&pr.created_at);
        let is_stale = crate::util::time::is_stale(&pr.created_at, stale_threshold_hours);

        let vote_summary = pr
            .reviewers
            .iter()
            .map(|r| r.vote.initials_display(&r.user))
            .collect::<Vec<_>>()
            .join(" ");

        let source_display = format!(
            "{}/{}/{}",
            pr.id.provider, pr.id.project, pr.id.repository
        );

        Self {
            pr,
            age_display,
            is_stale,
            vote_summary,
            source_display,
        }
    }
}

/// Central application state, shared via iocraft context
pub struct AppState {
    pub reviewing: Vec<EnrichedPr>,
    pub authored: Vec<EnrichedPr>,
    pub active_tab: ActiveTab,
    pub active_view: ActiveView,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub filter_text: String,
    pub filter_active: bool,
    pub provider_filter: Option<usize>,
    pub last_refresh: Option<DateTime<Utc>>,
    pub errors: Vec<String>,
    pub loading: bool,
    pub toast_message: Option<(String, std::time::Instant)>,
    pub provider_names: Vec<String>,
}

impl AppState {
    pub fn new(provider_names: Vec<String>) -> Self {
        Self {
            reviewing: Vec::new(),
            authored: Vec::new(),
            active_tab: ActiveTab::Reviewing,
            active_view: ActiveView::Dashboard,
            selected_index: 0,
            scroll_offset: 0,
            filter_text: String::new(),
            filter_active: false,
            provider_filter: None,
            last_refresh: None,
            errors: Vec::new(),
            loading: true,
            toast_message: None,
            provider_names,
        }
    }

    pub fn current_list(&self) -> &[EnrichedPr] {
        match self.active_tab {
            ActiveTab::Reviewing => &self.reviewing,
            ActiveTab::Authored => &self.authored,
        }
    }

    pub fn filtered_list(&self) -> Vec<&EnrichedPr> {
        crate::state::filters::apply_filters(
            self.current_list(),
            &self.filter_text,
            self.provider_filter.map(|i| {
                self.provider_names
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("")
            }),
        )
    }

    /// Ensure selected_index stays within bounds after data changes
    pub fn clamp_selection(&mut self) {
        let len = self.filtered_list().len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    pub fn show_toast(&mut self, message: String) {
        self.toast_message = Some((message, std::time::Instant::now()));
    }
}
