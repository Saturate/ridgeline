use iocraft::prelude::*;

/// Actions that can be triggered by key events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    JumpTop,
    JumpBottom,
    OpenDetail,
    OpenBrowser,
    SwitchTab,
    FocusSearch,
    ForceRefresh,
    ProviderFilter(usize),
    ClearProviderFilter,
    ToggleHelp,
    Quit,
    Back,
    None,
}

/// Map a key event to an action. Context-aware: `filter_active` changes
/// how character keys are interpreted.
pub fn map_key(event: &KeyEvent, filter_active: bool) -> Action {
    if event.kind == KeyEventKind::Release {
        return Action::None;
    }

    // Ctrl+C always maps to Quit regardless of context
    if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }

    // When filter is active, only handle escape and enter specially
    if filter_active {
        return match event.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Back, // commit filter and unfocus
            _ => Action::None,              // let TextInput handle the rest
        };
    }

    match event.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Enter => Action::OpenDetail,
        KeyCode::Char('o') => Action::OpenBrowser,
        KeyCode::Tab => Action::SwitchTab,
        KeyCode::Char('/') => Action::FocusSearch,
        KeyCode::Char('r') => Action::ForceRefresh,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Esc => Action::Back,
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let n = c.to_digit(10).unwrap() as usize;
            if n == 0 {
                Action::ClearProviderFilter
            } else {
                Action::ProviderFilter(n - 1)
            }
        }
        _ => Action::None,
    }
}

/// Keybinding entries for the help overlay
pub fn keybinding_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("j/↓", "Move down"),
        ("k/↑", "Move up"),
        ("g/G", "Jump to top/bottom"),
        ("Enter", "Open PR detail"),
        ("o", "Open in browser"),
        ("Tab", "Switch Reviewing/My PRs"),
        ("/", "Search filter"),
        ("r", "Force refresh"),
        ("1-9", "Filter by provider"),
        ("0", "Clear provider filter"),
        ("?", "Toggle help"),
        ("q", "Quit"),
        ("Esc", "Back / clear filter"),
    ]
}
