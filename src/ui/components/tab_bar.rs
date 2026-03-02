use iocraft::prelude::*;

use crate::state::app_state::ActiveTab;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct TabBarProps {
    pub active_tab: Option<ActiveTab>,
    pub reviewing_count: u32,
    pub authored_count: u32,
    pub theme: Theme,
}

#[component]
pub fn TabBar(props: &TabBarProps) -> impl Into<AnyElement<'static>> {
    let active = props.active_tab.unwrap_or(ActiveTab::Reviewing);
    let theme = props.theme;

    let (rev_color, auth_color) = match active {
        ActiveTab::Reviewing => (theme.tab_active, theme.tab_inactive),
        ActiveTab::Authored => (theme.tab_inactive, theme.tab_active),
    };

    let (rev_weight, auth_weight) = match active {
        ActiveTab::Reviewing => (Weight::Bold, Weight::Normal),
        ActiveTab::Authored => (Weight::Normal, Weight::Bold),
    };

    element! {
        View(
            border_style: BorderStyle::Single,
            border_color: theme.border,
            border_edges: Edges::Bottom,
            padding_left: 1,
            padding_right: 1,
        ) {
            Text(
                content: format!(" Reviewing ({}) ", props.reviewing_count),
                color: rev_color,
                weight: rev_weight,
            )
            Text(content: "  │  ", color: theme.fg_muted)
            Text(
                content: format!(" My PRs ({}) ", props.authored_count),
                color: auth_color,
                weight: auth_weight,
            )
            Text(content: "    ", color: theme.fg_muted)
            Text(content: "Tab to switch", color: theme.fg_muted)
        }
    }
}
