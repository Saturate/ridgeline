use iocraft::prelude::*;

use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct AgeIndicatorProps {
    pub age: String,
    pub is_stale: bool,
    pub theme: Theme,
}

#[component]
pub fn AgeIndicator(props: &AgeIndicatorProps) -> impl Into<AnyElement<'static>> {
    let color = if props.is_stale {
        props.theme.stale
    } else {
        props.theme.fg_dim
    };

    element! {
        Text(content: props.age.clone(), color)
    }
}
