use iocraft::prelude::*;

use crate::ui::keybindings;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct HelpOverlayProps {
    pub visible: bool,
    pub theme: Theme,
}

#[component]
pub fn HelpOverlay(props: &HelpOverlayProps) -> impl Into<AnyElement<'static>> {
    if !props.visible {
        return element! { View {} };
    }

    let theme = props.theme;
    let bindings = keybindings::keybinding_help();

    element! {
        View(
            position: Position::Absolute,
            top: 3,
            left: 4,
            right: 4,
            bottom: 3,
            border_style: BorderStyle::Round,
            border_color: theme.accent,
            background_color: Color::DarkGrey,
            padding: 2,
            flex_direction: FlexDirection::Column,
        ) {
            Text(
                content: "Keybindings",
                color: theme.accent,
                weight: Weight::Bold,
            )
            View(margin_top: 1, flex_direction: FlexDirection::Column) {
                #(bindings.iter().map(|(key, desc)| {
                    element! {
                        View {
                            View(width: 16) {
                                Text(content: key.to_string(), color: theme.fg, weight: Weight::Bold)
                            }
                            Text(content: desc.to_string(), color: theme.fg_dim)
                        }
                    }
                }))
            }
            View(margin_top: 2) {
                Text(content: "Press ? or Esc to close", color: theme.fg_muted)
            }
        }
    }
}
