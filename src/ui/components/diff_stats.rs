use iocraft::prelude::*;

use crate::providers::types::DiffStats as DiffStatsModel;

#[derive(Default, Props)]
pub struct DiffStatsProps<'a> {
    pub stats: Option<&'a DiffStatsModel>,
}

#[component]
pub fn DiffStats<'a>(props: &DiffStatsProps<'a>) -> impl Into<AnyElement<'a>> {
    let content = match props.stats {
        Some(stats) => format!("+{}/-{}", stats.additions, stats.deletions),
        None => String::new(),
    };

    let color = if props.stats.is_some() {
        Some(Color::Grey)
    } else {
        None
    };

    element! {
        Text(content, color)
    }
}
