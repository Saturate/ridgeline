//! Tab filter matching that doesn't depend on UI state.

use chrono::{DateTime, Utc};

use crate::config::model::{Assigned, DraftFilter, TabFilter};
use crate::providers::types::{PullRequest, Vote};

/// Matches `candidates` against include/exclude patterns, case-insensitively
/// by substring. Patterns starting with `!` exclude. With no include
/// patterns everything not excluded passes; with some, at least one has to
/// hit. "renovate" catches "Renovate Bot", which is the point.
pub fn matches_patterns(patterns: &[String], candidates: &[&str]) -> bool {
    let candidates: Vec<String> = candidates.iter().map(|c| c.to_lowercase()).collect();
    let hit = |pattern: &str| {
        let p = pattern.trim().to_lowercase();
        !p.is_empty() && candidates.iter().any(|c| c.contains(&p))
    };

    let mut has_include = false;
    let mut included = false;
    for pattern in patterns {
        let pattern = pattern.trim();
        if let Some(excluded) = pattern.strip_prefix('!') {
            if hit(excluded) {
                return false;
            }
        } else if !pattern.is_empty() {
            has_include = true;
            included |= hit(pattern);
        }
    }
    !has_include || included
}

/// Splits a comma-separated settings field into patterns.
pub fn parse_patterns(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != "!")
        .map(String::from)
        .collect()
}

/// The filters that only need the PR and who you are on its provider.
/// `me` is your user id there, when known.
pub fn passes(filter: &TabFilter, pr: &PullRequest, me: Option<&str>, now: DateTime<Utc>) -> bool {
    if !filter.repos.is_empty()
        && !matches_patterns(
            &filter.repos,
            &[
                &pr.id.provider,
                &pr.repository.project,
                &pr.repository.name,
                &format!("{}/{}", pr.repository.project, pr.repository.name),
            ],
        )
    {
        return false;
    }

    if !filter.authors.is_empty() {
        let unique = pr.author.unique_name.as_deref().unwrap_or_default();
        if !matches_patterns(&filter.authors, &[&pr.author.display_name, unique]) {
            return false;
        }
    }

    let mine = me.and_then(|id| pr.reviewers.iter().find(|r| r.user.id == id));
    if filter.hide_approved_by_me && me.is_some_and(|id| pr.is_approved_by(id)) {
        return false;
    }
    let sent_back = mine.is_some_and(|r| matches!(r.vote, Vote::WaitingForAuthor | Vote::Rejected));
    match filter.sent_back {
        Some(DraftFilter::Hide) if sent_back => return false,
        Some(DraftFilter::Only) if !sent_back => return false,
        _ => {}
    }
    match filter.assigned {
        Some(Assigned::Direct) if mine.is_none() => return false,
        // Your own PRs never list you as a reviewer, but they didn't reach
        // you through a team either.
        Some(Assigned::Indirect) if mine.is_some() || me == Some(pr.author.id.as_str()) => {
            return false;
        }
        _ => {}
    }
    if filter.required_only && !mine.is_some_and(|r| r.is_required) {
        return false;
    }

    let age_days = (now - pr.created_at).num_days().max(0) as u64;
    if filter
        .min_age_days
        .is_some_and(|min| age_days < u64::from(min))
    {
        return false;
    }
    if filter
        .max_age_days
        .is_some_and(|max| age_days > u64::from(max))
    {
        return false;
    }

    if let Some(prefix) = filter
        .target_branch
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if !pr
            .target_branch
            .to_lowercase()
            .starts_with(&prefix.to_lowercase())
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::*;

    fn p(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn patterns_include_and_exclude() {
        assert!(matches_patterns(&[], &["anything"]));
        assert!(matches_patterns(&p(&["dcc"]), &["DCC Energi"]));
        assert!(!matches_patterns(&p(&["dcc"]), &["NORRIQ"]));
        assert!(!matches_patterns(&p(&["!renovate"]), &["Renovate Bot"]));
        assert!(matches_patterns(&p(&["!renovate"]), &["Kristaps"]));
        // Exclude wins over include.
        assert!(!matches_patterns(
            &p(&["dcc", "!frontends"]),
            &["DCC Energi", "DCC.Frontends"]
        ));
    }

    #[test]
    fn parses_comma_separated_fields() {
        assert_eq!(
            parse_patterns(" !renovate , dependabot,, ! "),
            p(&["!renovate", "dependabot"])
        );
        assert!(parse_patterns("").is_empty());
    }

    fn reviewer(id: &str, vote: Vote, required: bool) -> Reviewer {
        Reviewer {
            user: User {
                id: id.into(),
                display_name: id.into(),
                unique_name: None,
            },
            vote,
            is_required: required,
        }
    }

    fn pr(days_old: i64, reviewers: Vec<Reviewer>) -> PullRequest {
        PullRequest {
            id: PrId {
                provider: "DCC Energi".into(),
                project: "Integration Platform".into(),
                repository: "DCC.Frontends".into(),
                number: 1,
            },
            title: "t".into(),
            description: None,
            status: PrStatus::Active,
            is_draft: false,
            created_at: Utc::now() - chrono::Duration::days(days_old),
            author: User {
                id: "a".into(),
                display_name: "Renovate Bot".into(),
                unique_name: Some("renovate@example.com".into()),
            },
            source_branch: "renovate/foo".into(),
            target_branch: "release/1.2".into(),
            reviewers,
            repository: Repository {
                name: "DCC.Frontends".into(),
                project: "Integration Platform".into(),
            },
            labels: Vec::new(),
            merge_status: None,
            build_status: None,
            source_commit_id: None,
            web_url: String::new(),
            open_threads: 0,
        }
    }

    fn filter() -> TabFilter {
        TabFilter::default()
    }

    #[test]
    fn repo_and_author_filters() {
        let pr = pr(1, Vec::new());
        let now = Utc::now();
        assert!(passes(
            &TabFilter {
                repos: p(&["Integration Platform"]),
                ..filter()
            },
            &pr,
            None,
            now
        ));
        assert!(!passes(
            &TabFilter {
                repos: p(&["!dcc energi"]),
                ..filter()
            },
            &pr,
            None,
            now
        ));
        assert!(!passes(
            &TabFilter {
                authors: p(&["!renovate"]),
                ..filter()
            },
            &pr,
            None,
            now
        ));
        assert!(passes(
            &TabFilter {
                authors: p(&["example.com"]),
                ..filter()
            },
            &pr,
            None,
            now
        ));
    }

    #[test]
    fn assignment_and_required() {
        let now = Utc::now();
        let direct = pr(1, vec![reviewer("me", Vote::NoVote, true)]);
        let via_team = pr(1, vec![reviewer("team", Vote::NoVote, true)]);
        let only_direct = TabFilter {
            assigned: Some(Assigned::Direct),
            ..filter()
        };
        let only_indirect = TabFilter {
            assigned: Some(Assigned::Indirect),
            ..filter()
        };
        let required = TabFilter {
            required_only: true,
            ..filter()
        };

        assert!(passes(&only_direct, &direct, Some("me"), now));
        assert!(!passes(&only_direct, &via_team, Some("me"), now));
        assert!(passes(&only_indirect, &via_team, Some("me"), now));
        assert!(passes(&required, &direct, Some("me"), now));
        // A required team entry isn't you being required.
        assert!(!passes(&required, &via_team, Some("me"), now));
        // Your own PR (the fixture's author id is "a") isn't "via team".
        assert!(!passes(&only_indirect, &via_team, Some("a"), now));
    }

    #[test]
    fn age_and_target_branch() {
        let now = Utc::now();
        let old = pr(10, Vec::new());
        assert!(passes(
            &TabFilter {
                min_age_days: Some(7),
                ..filter()
            },
            &old,
            None,
            now
        ));
        assert!(!passes(
            &TabFilter {
                max_age_days: Some(7),
                ..filter()
            },
            &old,
            None,
            now
        ));
        assert!(passes(
            &TabFilter {
                target_branch: Some("Release/".into()),
                ..filter()
            },
            &old,
            None,
            now
        ));
        assert!(!passes(
            &TabFilter {
                target_branch: Some("main".into()),
                ..filter()
            },
            &old,
            None,
            now
        ));
    }

    #[test]
    fn hides_your_approvals() {
        let now = Utc::now();
        let approved = pr(1, vec![reviewer("me", Vote::Approved, false)]);
        let hide = TabFilter {
            hide_approved_by_me: true,
            ..filter()
        };
        assert!(!passes(&hide, &approved, Some("me"), now));
        assert!(passes(&hide, &approved, Some("someone-else"), now));
    }

    #[test]
    fn sent_back_by_you() {
        let now = Utc::now();
        let hide = TabFilter {
            sent_back: Some(DraftFilter::Hide),
            ..filter()
        };
        let only = TabFilter {
            sent_back: Some(DraftFilter::Only),
            ..filter()
        };
        let waiting = pr(1, vec![reviewer("me", Vote::WaitingForAuthor, false)]);
        let rejected = pr(1, vec![reviewer("me", Vote::Rejected, false)]);
        let untouched = pr(1, vec![reviewer("me", Vote::NoVote, false)]);
        // Someone else sending it back still leaves it waiting on you.
        let others = pr(
            1,
            vec![
                reviewer("me", Vote::NoVote, false),
                reviewer("bob", Vote::WaitingForAuthor, false),
            ],
        );

        for sent in [&waiting, &rejected] {
            assert!(!passes(&hide, sent, Some("me"), now));
            assert!(passes(&only, sent, Some("me"), now));
        }
        for open in [&untouched, &others] {
            assert!(passes(&hide, open, Some("me"), now));
            assert!(!passes(&only, open, Some("me"), now));
        }
        // Without knowing who you are, nothing counts as sent back by you.
        assert!(passes(&hide, &waiting, None, now));
        assert!(!passes(&only, &waiting, None, now));
    }

    #[test]
    fn default_reviewing_and_sent_back_tabs_split_reviews() {
        let tabs = crate::config::model::GeneralConfig::default().tabs;
        let reviewing = &tabs[0].filter;
        let sent_back = &tabs[1].filter;
        let now = Utc::now();
        for vote in [Vote::NoVote, Vote::WaitingForAuthor, Vote::Rejected] {
            let pr = pr(1, vec![reviewer("me", vote, false)]);
            assert_ne!(
                passes(reviewing, &pr, Some("me"), now),
                passes(sent_back, &pr, Some("me"), now)
            );
        }
        // Approved ones are done: in neither tab.
        let approved = pr(1, vec![reviewer("me", Vote::Approved, false)]);
        assert!(!passes(reviewing, &approved, Some("me"), now));
        assert!(!passes(sent_back, &approved, Some("me"), now));
    }
}

#[cfg(test)]
mod readme_example {
    use crate::config::model::{Assigned, TabConfig};

    /// The example in README's "Tab filters" section; keep the two in sync.
    #[test]
    fn readme_tab_example_parses() {
        #[derive(serde::Deserialize)]
        struct General {
            tabs: Vec<TabConfig>,
        }
        #[derive(serde::Deserialize)]
        struct Doc {
            general: General,
        }
        let doc: Doc = toml::from_str(
            r#"
[[general.tabs]]
label = "Stale DCC reviews"
source = "reviewing"

[general.tabs.filter]
repos = ["DCC Energi"]
authors = ["!renovate", "!dependabot"]
assigned = "direct"
min_age_days = 7
"#,
        )
        .unwrap();
        let filter = &doc.general.tabs[0].filter;
        assert_eq!(filter.repos, vec!["DCC Energi"]);
        assert_eq!(filter.assigned, Some(Assigned::Direct));
        assert_eq!(filter.min_age_days, Some(7));
        // Fields left out keep their defaults.
        assert!(doc.general.tabs[0].enabled);
    }
}
