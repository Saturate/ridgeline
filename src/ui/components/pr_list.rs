use iocraft::prelude::*;

use crate::state::app_state::EnrichedPr;
use crate::ui::components::pr_row::PrRow;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct PrListProps {
    pub prs: Vec<EnrichedPr>,
    pub selected_index: u32,
    pub scroll_offset: u32,
    pub visible_height: u32,
    pub theme: Theme,
}

#[component]
pub fn PrList(props: &PrListProps) -> impl Into<AnyElement<'static>> {
    let theme = props.theme;

    if props.prs.is_empty() {
        return element! {
            View(
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            ) {
                Text(content: "No pull requests found", color: theme.fg_dim)
            }
        };
    }

    let offset = props.scroll_offset as usize;
    let visible = props.visible_height as usize;
    let end = (offset + visible).min(props.prs.len());
    let selected = props.selected_index as usize;

    // Clone visible PRs to own them (avoid borrowing from props)
    let visible_prs: Vec<EnrichedPr> = props.prs[offset..end].to_vec();

    element! {
        View(
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            overflow: Overflow::Hidden,
        ) {
            #(visible_prs.into_iter().enumerate().map(move |(i, epr)| {
                let actual_index = offset + i;
                let is_selected = actual_index == selected;
                element! {
                    PrRow(
                        pr: Some(epr),
                        is_selected,
                        theme,
                    )
                }
            }))
        }
    }
}
