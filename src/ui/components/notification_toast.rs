use iocraft::prelude::*;

use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct NotificationToastProps {
    pub message: String,
    pub visible: bool,
    pub theme: Theme,
}

#[component]
pub fn NotificationToast(props: &NotificationToastProps) -> impl Into<AnyElement<'static>> {
    if !props.visible || props.message.is_empty() {
        return element! { View {} };
    }

    let theme = props.theme;

    element! {
        View(
            position: Position::Absolute,
            top: 1,
            right: 2,
            border_style: BorderStyle::Round,
            border_color: theme.accent,
            padding_left: 1,
            padding_right: 1,
            background_color: theme.bg,
        ) {
            Text(content: props.message.clone(), color: theme.fg)
        }
    }
}
