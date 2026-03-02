use iocraft::prelude::*;

use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct FilterBarProps {
    pub filter_text: String,
    pub is_active: bool,
    pub theme: Theme,
    pub on_change: HandlerMut<'static, String>,
}

#[component]
pub fn FilterBar(props: &mut FilterBarProps) -> impl Into<AnyElement<'static>> {
    if !props.is_active && props.filter_text.is_empty() {
        return element! { View {} };
    }

    let border_color = if props.is_active {
        props.theme.border_active
    } else {
        props.theme.border
    };

    let theme = props.theme;

    element! {
        View(
            padding_left: 1,
            padding_right: 1,
            border_style: BorderStyle::Single,
            border_color,
            border_edges: Edges::Bottom,
        ) {
            Text(content: "/ ", color: theme.accent)
            TextInput(
                has_focus: props.is_active,
                value: props.filter_text.clone(),
                on_change: props.on_change.take(),
            )
        }
    }
}
