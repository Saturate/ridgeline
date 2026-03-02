use iocraft::prelude::*;

use crate::state::app_state::{ActiveTab, EnrichedPr};
use crate::ui::components::filter_bar::FilterBar;
use crate::ui::components::pr_list::PrList;
use crate::ui::components::tab_bar::TabBar;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct DashboardProps {
    pub filtered_prs: Vec<EnrichedPr>,
    pub active_tab: Option<ActiveTab>,
    pub reviewing_count: u32,
    pub authored_count: u32,
    pub selected_index: u32,
    pub scroll_offset: u32,
    pub visible_height: u32,
    pub filter_text: String,
    pub filter_active: bool,
    pub theme: Theme,
    pub on_filter_change: HandlerMut<'static, String>,
}

#[component]
pub fn Dashboard(props: &mut DashboardProps) -> impl Into<AnyElement<'static>> {
    let theme = props.theme;

    element! {
        View(flex_direction: FlexDirection::Column, flex_grow: 1.0) {
            TabBar(
                active_tab: props.active_tab,
                reviewing_count: props.reviewing_count,
                authored_count: props.authored_count,
                theme,
            )
            FilterBar(
                filter_text: props.filter_text.clone(),
                is_active: props.filter_active,
                theme,
                on_change: props.on_filter_change.take(),
            )
            PrList(
                prs: std::mem::take(&mut props.filtered_prs),
                selected_index: props.selected_index,
                scroll_offset: props.scroll_offset,
                visible_height: props.visible_height,
                theme,
            )
        }
    }
}
