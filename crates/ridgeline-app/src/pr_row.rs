use gpui_kit::{Hsla, Rgba};

#[derive(Debug, PartialEq)]
pub struct ConventionalCommit<'a> {
    pub kind: &'a str,
    pub scope: Option<&'a str>,
    pub description: &'a str,
    pub breaking: bool,
}

/// Matches `type(scope)!: description`, same shape the Tauri version accepted.
pub fn parse_conventional_commit(title: &str) -> Option<ConventionalCommit<'_>> {
    let kind_end = title
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(title.len());
    if kind_end == 0 {
        return None;
    }
    let kind = &title[..kind_end];
    let mut rest = &title[kind_end..];

    let mut scope = None;
    if let Some(after_paren) = rest.strip_prefix('(') {
        let close = after_paren.find(')')?;
        if close == 0 {
            return None;
        }
        scope = Some(&after_paren[..close]);
        rest = &after_paren[close + 1..];
    }

    let breaking = rest.starts_with('!');
    if breaking {
        rest = &rest[1..];
    }

    let description = rest.strip_prefix(':')?.trim_start();
    if description.is_empty() {
        return None;
    }

    Some(ConventionalCommit {
        kind,
        scope,
        description,
        breaking,
    })
}

/// Azure DevOps team reviewers come through as `[Project]\Team Name`.
pub fn is_group_reviewer(display_name: &str) -> bool {
    display_name.starts_with('[') && display_name.contains("]\\")
}

pub fn group_label(display_name: &str) -> &str {
    display_name
        .split_once("]\\")
        .map(|(_, name)| name)
        .unwrap_or(display_name)
}

pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

pub fn parse_hex_color(hex: &str) -> Option<Hsla> {
    Rgba::try_from(hex).ok().map(Hsla::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_type_scope_and_description() {
        let cc = parse_conventional_commit("fix(heartbeat): bump heartbeat").unwrap();
        assert_eq!(cc.kind, "fix");
        assert_eq!(cc.scope, Some("heartbeat"));
        assert_eq!(cc.description, "bump heartbeat");
        assert!(!cc.breaking);
    }

    #[test]
    fn parses_breaking_without_scope() {
        let cc = parse_conventional_commit("feat!: drop v1 api").unwrap();
        assert_eq!(cc.kind, "feat");
        assert_eq!(cc.scope, None);
        assert!(cc.breaking);
    }

    #[test]
    fn rejects_plain_titles() {
        assert_eq!(parse_conventional_commit("Bump heartbeat to 8.19"), None);
        assert_eq!(parse_conventional_commit("fix(): empty scope"), None);
        assert_eq!(parse_conventional_commit("fix:"), None);
        assert_eq!(parse_conventional_commit(": nothing"), None);
    }

    #[test]
    fn detects_team_reviewers() {
        assert!(is_group_reviewer(r"[Integration Platform]\Integration Platform Team"));
        assert_eq!(
            group_label(r"[Integration Platform]\Integration Platform Team"),
            "Integration Platform Team"
        );
        assert!(!is_group_reviewer("Allan Kimmer Jensen"));
    }

    #[test]
    fn builds_initials() {
        assert_eq!(initials("Allan Kimmer Jensen"), "AK");
        assert_eq!(initials("kristaps"), "K");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn parses_provider_colors() {
        assert!(parse_hex_color("#3b82f6").is_some());
        assert!(parse_hex_color("not a color").is_none());
    }
}
