use iocraft::prelude::*;

use crate::providers::types::{Reviewer, Vote};
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct VoteBadgeProps<'a> {
    pub reviewer: Option<&'a Reviewer>,
    pub theme: Theme,
}

#[component]
pub fn VoteBadge<'a>(props: &VoteBadgeProps<'a>) -> impl Into<AnyElement<'a>> {
    let reviewer = match props.reviewer {
        Some(r) => r,
        None => {
            return element! {
                Text(content: "")
            };
        }
    };

    let color = props.theme.vote_color(&reviewer.vote);
    let display = reviewer.vote.initials_display(&reviewer.user);
    let required_marker = if reviewer.is_required { "*" } else { "" };

    element! {
        Text(
            content: format!("{display}{required_marker}"),
            color,
        )
    }
}

/// Render all votes as a single inline string with ANSI coloring.
pub fn votes_display(reviewers: &[Reviewer], theme: &Theme) -> Vec<(String, Color)> {
    reviewers
        .iter()
        .filter(|r| r.vote != Vote::NoVote)
        .map(|r| {
            let display = r.vote.initials_display(&r.user);
            let color = theme.vote_color(&r.vote);
            (display, color)
        })
        .collect()
}
