use iocraft::prelude::*;

use crate::providers::types::Vote;
use crate::state::app_state::EnrichedPr;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct PrRowProps {
    pub pr: Option<EnrichedPr>,
    pub is_selected: bool,
    pub theme: Theme,
}

#[component]
pub fn PrRow(props: &PrRowProps) -> impl Into<AnyElement<'static>> {
    let epr = match &props.pr {
        Some(p) => p,
        None => {
            return element! { View {} };
        }
    };

    let theme = props.theme;
    let pr = &epr.pr;

    let bg = if props.is_selected {
        Some(theme.selection_bg)
    } else {
        None
    };

    let selector = if props.is_selected { "▸ " } else { "  " };

    let title_color = if pr.is_draft {
        theme.draft
    } else {
        theme.fg
    };

    let draft_tag = if pr.is_draft { " [draft]" } else { "" };

    let age_color = if epr.is_stale {
        theme.stale
    } else {
        theme.fg_dim
    };

    let vote_parts: Vec<(String, Color)> = pr
        .reviewers
        .iter()
        .filter(|r| r.vote != Vote::NoVote)
        .map(|r| {
            let display = r.vote.initials_display(&r.user);
            let color = theme.vote_color(&r.vote);
            (display, color)
        })
        .collect();

    let active_votes = pr.reviewers.iter().filter(|r| r.vote != Vote::NoVote).count();

    let source = epr.source_display.clone();
    let id_str = format!("#{}", pr.id.number);
    let title_str = format!("{}{}", pr.title, draft_tag);
    let age_str = epr.age_display.clone();
    let votes_str = if active_votes > 0 {
        format!("●{}", active_votes)
    } else {
        String::new()
    };

    element! {
        View(background_color: bg) {
            Text(content: selector.to_string(), color: theme.accent)
            View(width: 28pct, overflow: Overflow::Hidden) {
                Text(content: source, color: theme.fg_dim)
            }
            View(width: 6pct) {
                Text(content: id_str, color: theme.accent)
            }
            View(width: 36pct, overflow: Overflow::Hidden) {
                Text(content: title_str, color: title_color)
            }
            View(width: 5pct) {
                Text(content: age_str, color: age_color)
            }
            View(width: 20pct, overflow: Overflow::Hidden) {
                #(vote_parts.into_iter().enumerate().map(|(i, (text, color))| {
                    let sep = if i > 0 { " " } else { "" };
                    element! {
                        Text(content: format!("{sep}{text}"), color)
                    }
                }))
            }
            View(width: 5pct) {
                Text(content: votes_str, color: theme.fg_dim)
            }
        }
    }
}
