use crate::state::app_state::EnrichedPr;

pub fn apply_filters<'a>(
    prs: &'a [EnrichedPr],
    search: &str,
    provider_filter: Option<&str>,
) -> Vec<&'a EnrichedPr> {
    let search_lower = search.to_lowercase();

    prs.iter()
        .filter(|epr| {
            // Provider filter
            if let Some(provider) = provider_filter {
                if !provider.is_empty() && epr.pr.id.provider != provider {
                    return false;
                }
            }

            // Text search
            if !search_lower.is_empty() {
                let matches_title = epr.pr.title.to_lowercase().contains(&search_lower);
                let matches_author = epr
                    .pr
                    .author
                    .display_name
                    .to_lowercase()
                    .contains(&search_lower);
                let matches_repo = epr
                    .pr
                    .repository
                    .name
                    .to_lowercase()
                    .contains(&search_lower);
                let matches_branch = epr
                    .pr
                    .source_branch
                    .to_lowercase()
                    .contains(&search_lower);
                let matches_id = epr.pr.id.number.to_string().contains(&search_lower);

                if !(matches_title
                    || matches_author
                    || matches_repo
                    || matches_branch
                    || matches_id)
                {
                    return false;
                }
            }

            true
        })
        .collect()
}
