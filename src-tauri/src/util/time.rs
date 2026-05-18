use chrono::{DateTime, Utc};

/// Format a timestamp as a relative time string like "2h", "3d", "1w"
pub fn relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;

    if minutes < 1 {
        "now".to_string()
    } else if minutes < 60 {
        format!("{}m", minutes)
    } else if hours < 24 {
        format!("{}h", hours)
    } else if days < 7 {
        format!("{}d", days)
    } else if weeks < 52 {
        format!("{}w", weeks)
    } else {
        format!("{}y", days / 365)
    }
}

pub fn is_stale(dt: &DateTime<Utc>, threshold_hours: u64) -> bool {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);
    duration.num_hours() as u64 >= threshold_hours
}
