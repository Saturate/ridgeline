use iocraft::prelude::*;

use crate::providers::types::Vote;
use crate::state::app_state::EnrichedPr;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct DetailViewProps {
    pub pr: Option<EnrichedPr>,
    pub theme: Theme,
}

#[component]
pub fn DetailView(props: &DetailViewProps) -> impl Into<AnyElement<'static>> {
    let epr = match &props.pr {
        Some(p) => p,
        None => {
            return element! {
                View(
                    flex_grow: 1.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                ) {
                    Text(content: "PR not found")
                }
            };
        }
    };

    let theme = props.theme;
    let pr = &epr.pr;

    let status_color = match pr.status {
        crate::providers::types::PrStatus::Active => theme.success,
        crate::providers::types::PrStatus::Completed => theme.accent,
        crate::providers::types::PrStatus::Abandoned => theme.fg_dim,
    };

    let description = pr
        .description
        .as_deref()
        .unwrap_or("No description provided.")
        .to_string();

    let header_line1 = format!("#{} {}", pr.id.number, pr.title);
    let header_line2_status = pr.status.to_string();
    let header_line2_author = pr.author.display_name.clone();
    let header_line2_age = epr.age_display.clone();
    let branch_line = format!("{} → {}", pr.source_branch, pr.target_branch);

    let reviewer_lines: Vec<(String, Color)> = pr
        .reviewers
        .iter()
        .map(|r| {
            let vote_color = theme.vote_color(&r.vote);
            let required = if r.is_required { " (required)" } else { "" };
            let vote_text = match &r.vote {
                Vote::Approved => "Approved",
                Vote::ApprovedWithSuggestions => "Approved with suggestions",
                Vote::NoVote => "No vote",
                Vote::WaitingForAuthor => "Waiting for author",
                Vote::Rejected => "Rejected",
            };
            (
                format!(
                    "  {} {} - {vote_text}{required}",
                    r.vote.symbol(),
                    r.user.display_name,
                ),
                vote_color,
            )
        })
        .collect();

    element! {
        View(flex_direction: FlexDirection::Column, flex_grow: 1.0, padding: 1) {
            View(
                border_style: BorderStyle::Round,
                border_color: theme.border_active,
                padding: 1,
                flex_direction: FlexDirection::Column,
            ) {
                Text(content: header_line1, color: theme.fg, weight: Weight::Bold)
                View(margin_top: 1) {
                    Text(content: "Status: ", color: theme.fg_dim)
                    Text(content: header_line2_status, color: status_color)
                    Text(content: "  │  ", color: theme.fg_muted)
                    Text(content: "Author: ", color: theme.fg_dim)
                    Text(content: header_line2_author, color: theme.fg)
                    Text(content: "  │  ", color: theme.fg_muted)
                    Text(content: "Age: ", color: theme.fg_dim)
                    Text(content: header_line2_age, color: theme.fg)
                }
                Text(content: branch_line, color: theme.fg_dim)
            }

            View(
                margin_top: 1,
                border_style: BorderStyle::Single,
                border_color: theme.border,
                padding: 1,
                flex_direction: FlexDirection::Column,
            ) {
                Text(content: "Description", color: theme.fg, weight: Weight::Bold)
                View(margin_top: 1) {
                    Text(content: description, color: theme.fg_dim, wrap: TextWrap::Wrap)
                }
            }

            View(
                margin_top: 1,
                border_style: BorderStyle::Single,
                border_color: theme.border,
                padding: 1,
                flex_direction: FlexDirection::Column,
            ) {
                Text(content: "Reviewers", color: theme.fg, weight: Weight::Bold)
                #(reviewer_lines.into_iter().map(|(text, color)| {
                    element! {
                        Text(content: text, color)
                    }
                }))
            }

            View(margin_top: 1) {
                Text(content: "Esc back  │  o open in browser", color: theme.fg_muted)
            }
        }
    }
}
