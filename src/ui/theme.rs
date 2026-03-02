use iocraft::prelude::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,
    pub border: Color,
    pub border_active: Color,
    pub accent: Color,
    pub selection_bg: Color,
    pub vote_approved: Color,
    pub vote_suggestions: Color,
    pub vote_pending: Color,
    pub vote_waiting: Color,
    pub vote_rejected: Color,
    pub stale: Color,
    pub error: Color,
    pub success: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub draft: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            fg_dim: Color::Grey,
            fg_muted: Color::DarkGrey,
            border: Color::DarkGrey,
            border_active: Color::Blue,
            accent: Color::Cyan,
            selection_bg: Color::DarkBlue,
            vote_approved: Color::Green,
            vote_suggestions: Color::DarkGreen,
            vote_pending: Color::Grey,
            vote_waiting: Color::Yellow,
            vote_rejected: Color::Red,
            stale: Color::Yellow,
            error: Color::Red,
            success: Color::Green,
            tab_active: Color::Cyan,
            tab_inactive: Color::Grey,
            draft: Color::DarkGrey,
        }
    }

    pub fn vote_color(&self, vote: &crate::providers::types::Vote) -> Color {
        use crate::providers::types::Vote;
        match vote {
            Vote::Approved => self.vote_approved,
            Vote::ApprovedWithSuggestions => self.vote_suggestions,
            Vote::NoVote => self.vote_pending,
            Vote::WaitingForAuthor => self.vote_waiting,
            Vote::Rejected => self.vote_rejected,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
