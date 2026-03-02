use iocraft::prelude::*;

use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct StatusBarProps {
    pub last_refresh: String,
    pub error_count: u32,
    pub last_error: String,
    pub loading: bool,
    pub spinner_frame: usize,
    pub provider_filter: String,
    pub theme: Theme,
}

#[component]
pub fn StatusBar(props: &StatusBarProps) -> impl Into<AnyElement<'static>> {
    let theme = props.theme;

    let refresh_text = if props.loading {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame = frames[props.spinner_frame % frames.len()];
        format!("{frame} refreshing...")
    } else if props.last_refresh.is_empty() {
        "never refreshed".to_string()
    } else {
        format!("refreshed {}", props.last_refresh)
    };

    let filter_text = if props.provider_filter.is_empty() {
        String::new()
    } else {
        format!(" │ filter: {}", props.provider_filter)
    };

    let error_text = if props.error_count > 0 {
        format!(" │ {} error(s): {}", props.error_count, props.last_error)
    } else {
        String::new()
    };

    let error_color = if props.error_count > 0 {
        theme.error
    } else {
        theme.fg_dim
    };

    element! {
        View(
            border_style: BorderStyle::Single,
            border_color: theme.border,
            border_edges: Edges::Top,
            padding_left: 1,
            padding_right: 1,
            justify_content: JustifyContent::SpaceBetween,
        ) {
            View {
                Text(content: refresh_text, color: theme.fg_dim)
                Text(content: filter_text, color: theme.accent)
                Text(content: error_text, color: error_color)
            }
            Text(content: "? help  q quit", color: theme.fg_muted)
        }
    }
}
