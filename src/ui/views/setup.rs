use std::time::Duration;

use async_compat::CompatExt;
use iocraft::prelude::*;

use crate::config::{credentials, loader};
use crate::providers::azure_devops::client::AzureDevOpsClient;
use crate::ui::theme::Theme;

#[derive(Default, Props)]
pub struct SetupWizardProps {}

#[component]
pub fn SetupWizard(_props: &SetupWizardProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let theme = Theme::dark();

    // State: step 0=input, 1=connecting, 2=projects, 3=done
    let mut step = hooks.use_state(|| 0u8);
    let mut active_field = hooks.use_state(|| 0u8); // 0=URL, 1=PAT
    let mut org_url = hooks.use_state(|| String::from("https://dev.azure.com/"));
    let mut pat = hooks.use_state(|| String::new());
    let mut error_msg = hooks.use_state(|| String::new());
    let mut user_name = hooks.use_state(|| String::new());
    let mut projects = hooks.use_state(|| Vec::<(String, bool)>::new());
    let mut project_idx = hooks.use_state(|| 0usize);
    let mut project_scroll = hooks.use_state(|| 0usize);
    let mut should_exit = hooks.use_state(|| false);
    let mut quit_pending = hooks.use_state(|| false);
    let mut trigger_connect = hooks.use_state(|| false);
    let mut trigger_save = hooks.use_state(|| false);
    let mut search_text = hooks.use_state(|| String::new());
    let mut search_active = hooks.use_state(|| false);
    let mut spinner_frame = hooks.use_state(|| 0usize);

    // Quit confirmation auto-reset
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_secs(3)).await;
            if quit_pending.get() {
                quit_pending.set(false);
                error_msg.set(String::new());
            }
        }
    });

    // Spinner animation
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(120)).await;
            spinner_frame.set(spinner_frame.get() + 1);
        }
    });

    // Background async tasks: connection test + save
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(100)).await;

            if trigger_connect.get() {
                trigger_connect.set(false);
                step.set(1);
                error_msg.set(String::new());

                let url = org_url.read().clone();
                let token = pat.read().clone();
                let client = AzureDevOpsClient::new(&url, &token);

                match client.get_connection_data().compat().await {
                    Ok(conn) => {
                        if step.get() != 1 {
                            continue;
                        }

                        let name = conn
                            .authenticated_user
                            .properties
                            .as_ref()
                            .and_then(|p| p.account.as_ref())
                            .and_then(|a| a.value.clone())
                            .unwrap_or_else(|| conn.authenticated_user.id.clone());
                        user_name.set(name);

                        match client.list_projects().compat().await {
                            Ok(project_list) if project_list.is_empty() => {
                                if step.get() == 1 {
                                    error_msg.set(
                                        "No projects found. Check PAT permissions.".to_string(),
                                    );
                                    step.set(0);
                                }
                            }
                            Ok(project_list) => {
                                if step.get() != 1 {
                                    continue;
                                }
                                let items: Vec<(String, bool)> =
                                    project_list.into_iter().map(|p| (p, false)).collect();
                                projects.set(items);
                                project_idx.set(0);
                                project_scroll.set(0);
                                step.set(2);
                            }
                            Err(e) => {
                                if step.get() == 1 {
                                    error_msg.set(format!("Failed to list projects: {e}"));
                                    step.set(0);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if step.get() == 1 {
                            error_msg.set(format!("Authentication failed: {e}"));
                            step.set(0);
                        }
                    }
                }
            }

            if trigger_save.get() {
                trigger_save.set(false);

                let url = org_url.read().clone();
                let token = pat.read().clone();
                let proj_list = projects.read().clone();

                let provider_name = url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("default")
                    .to_lowercase();

                let selected: Vec<String> = proj_list
                    .iter()
                    .filter(|(_, sel)| *sel)
                    .map(|(name, _)| name.clone())
                    .collect();

                if let Err(e) = credentials::store_token(&provider_name, &token) {
                    error_msg.set(format!("Failed to store token: {e}"));
                    step.set(2);
                    continue;
                }

                if let Err(e) = loader::save_config(&url, &provider_name, &selected) {
                    error_msg.set(format!("Failed to save config: {e}"));
                    step.set(2);
                    continue;
                }

                step.set(3);
            }
        }
    });

    // Helper: compute filtered project count for keyboard handler
    let compute_filtered_len = {
        move || -> usize {
            let all = projects.read().clone();
            let search = search_text.read().clone();
            if search.is_empty() {
                all.len()
            } else {
                let lower = search.to_lowercase();
                all.iter()
                    .filter(|(n, _)| n.to_lowercase().contains(&lower))
                    .count()
            }
        }
    };

    // Helper: get filtered project name at index
    let get_filtered_name_at = {
        move |idx: usize| -> Option<String> {
            let all = projects.read().clone();
            let search = search_text.read().clone();
            let filtered: Vec<&(String, bool)> = if search.is_empty() {
                all.iter().collect()
            } else {
                let lower = search.to_lowercase();
                all.iter()
                    .filter(|(n, _)| n.to_lowercase().contains(&lower))
                    .collect()
            };
            filtered.get(idx).map(|(n, _)| n.clone())
        }
    };

    // Keyboard handling — all manual, no TextInput conflicts
    hooks.use_terminal_events({
        move |event| {
            if let TerminalEvent::Key(key_event) = event {
                // Ignore key release events to avoid double input
                if key_event.kind == KeyEventKind::Release {
                    return;
                }

                // Ctrl+C: double-press to quit from any step
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    if quit_pending.get() {
                        should_exit.set(true);
                    } else {
                        quit_pending.set(true);
                        error_msg.set("Press again to quit".to_string());
                    }
                    return;
                }

                let s = step.get();

                match s {
                    0 => {
                        // Input step - TextInput handles character input
                        match key_event.code {
                            KeyCode::Enter => {
                                let url = org_url.read().clone();
                                let token = pat.read().clone();
                                if !url.is_empty() && !token.is_empty() {
                                    trigger_connect.set(true);
                                } else if active_field.get() == 0 && !url.is_empty() {
                                    active_field.set(1);
                                }
                            }
                            KeyCode::Tab | KeyCode::BackTab => {
                                active_field
                                    .set(if active_field.get() == 0 { 1 } else { 0 });
                            }
                            KeyCode::Esc => {
                                if quit_pending.get() {
                                    should_exit.set(true);
                                } else {
                                    quit_pending.set(true);
                                    error_msg.set("Press Esc again to quit".to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                    2 => {
                        // Project selection — TextInput handles search characters
                        if search_active.get() {
                            match key_event.code {
                                KeyCode::Esc => {
                                    search_active.set(false);
                                    search_text.set(String::new());
                                    project_idx.set(0);
                                    project_scroll.set(0);
                                }
                                KeyCode::Enter => {
                                    search_active.set(false);
                                }
                                KeyCode::Down => {
                                    let filtered_len = compute_filtered_len();
                                    if filtered_len > 0 {
                                        let new_idx =
                                            (project_idx.get() + 1).min(filtered_len - 1);
                                        project_idx.set(new_idx);
                                        let vis_h = height.saturating_sub(12) as usize;
                                        if vis_h > 0
                                            && new_idx >= project_scroll.get() + vis_h
                                        {
                                            project_scroll
                                                .set(new_idx.saturating_sub(vis_h - 1));
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    let new_idx = project_idx.get().saturating_sub(1);
                                    project_idx.set(new_idx);
                                    if new_idx < project_scroll.get() {
                                        project_scroll.set(new_idx);
                                    }
                                }
                                _ => {} // TextInput handles character input
                            }
                            return;
                        }

                        // Not searching — navigation + selection keys
                        let filtered_len = compute_filtered_len();

                        match key_event.code {
                            KeyCode::Char('j') | KeyCode::Down => {
                                if filtered_len > 0 {
                                    let new_idx =
                                        (project_idx.get() + 1).min(filtered_len - 1);
                                    project_idx.set(new_idx);
                                    let vis_h = height.saturating_sub(12) as usize;
                                    if vis_h > 0
                                        && new_idx >= project_scroll.get() + vis_h
                                    {
                                        project_scroll
                                            .set(new_idx.saturating_sub(vis_h - 1));
                                    }
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                let new_idx = project_idx.get().saturating_sub(1);
                                project_idx.set(new_idx);
                                if new_idx < project_scroll.get() {
                                    project_scroll.set(new_idx);
                                }
                            }
                            KeyCode::Char('g') => {
                                project_idx.set(0);
                                project_scroll.set(0);
                            }
                            KeyCode::Char('G') => {
                                if filtered_len > 0 {
                                    project_idx.set(filtered_len - 1);
                                    let vis_h = height.saturating_sub(12) as usize;
                                    if vis_h > 0 && filtered_len > vis_h {
                                        project_scroll.set(filtered_len - vis_h);
                                    }
                                }
                            }
                            KeyCode::Char(' ') | KeyCode::Char('x') => {
                                if let Some(target) =
                                    get_filtered_name_at(project_idx.get())
                                {
                                    let mut all_mut = projects.read().clone();
                                    if let Some(item) =
                                        all_mut.iter_mut().find(|(n, _)| n == &target)
                                    {
                                        item.1 = !item.1;
                                    }
                                    projects.set(all_mut);
                                }
                            }
                            KeyCode::Char('a') => {
                                let mut all_mut = projects.read().clone();
                                let any_unselected =
                                    all_mut.iter().any(|(_, sel)| !sel);
                                for item in &mut all_mut {
                                    item.1 = any_unselected;
                                }
                                projects.set(all_mut);
                            }
                            KeyCode::Char('/') => {
                                search_active.set(true);
                                search_text.set(String::new());
                            }
                            KeyCode::Enter => {
                                trigger_save.set(true);
                            }
                            KeyCode::Esc => {
                                step.set(0);
                                error_msg.set(String::new());
                            }
                            _ => {}
                        }
                    }
                    3 => {
                        if key_event.code == KeyCode::Enter {
                            should_exit.set(true);
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    if should_exit.get() {
        system.exit();
    }

    // ── Render ──────────────────────────────────────────────────────────
    let current_step = step.get();
    let err = error_msg.read().clone();

    let content: AnyElement = match current_step {
        0 => {
            let url_val = org_url.read().clone();
            let pat_val = pat.read().clone();
            let url_focused = active_field.get() == 0;
            let pat_focused = active_field.get() == 1;

            let url_border = if url_focused {
                theme.border_active
            } else {
                theme.border
            };
            let pat_border = if pat_focused {
                theme.border_active
            } else {
                theme.border
            };

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                ) {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: theme.border,
                        padding: 2,
                        width: 70pct,
                    ) {
                        Text(
                            content: "ridgeline setup",
                            color: theme.accent,
                            weight: Weight::Bold,
                        )
                        View(margin_top: 1) {
                            Text(
                                content: "Configure your Azure DevOps connection",
                                color: theme.fg_dim,
                            )
                        }

                        View(margin_top: 2) {
                            Text(content: "Organization URL", color: theme.fg)
                        }
                        View(
                            border_style: BorderStyle::Single,
                            border_color: url_border,
                            padding_left: 1,
                            padding_right: 1,
                        ) {
                            TextInput(
                                has_focus: url_focused,
                                value: url_val,
                                on_change: move |val: String| org_url.set(val),
                            )
                        }

                        View(margin_top: 1) {
                            Text(content: "Personal Access Token", color: theme.fg)
                        }
                        View(
                            border_style: BorderStyle::Single,
                            border_color: pat_border,
                            padding_left: 1,
                            padding_right: 1,
                        ) {
                            TextInput(
                                has_focus: pat_focused,
                                value: pat_val,
                                on_change: move |val: String| pat.set(val),
                            )
                        }

                        Text(content: err.clone(), color: theme.error)

                        View(margin_top: 2) {
                            Text(
                                content: "Tab switch field  Enter connect  Esc quit",
                                color: theme.fg_muted,
                            )
                        }
                    }
                }
            }
            .into()
        }

        1 => {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = frames[spinner_frame.get() % frames.len()];
            let url_display = org_url.read().clone();

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                ) {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: theme.border,
                        padding: 2,
                        width: 70pct,
                    ) {
                        Text(
                            content: "ridgeline setup",
                            color: theme.accent,
                            weight: Weight::Bold,
                        )
                        View(margin_top: 2) {
                            Text(
                                content: format!("{frame} Connecting to {url_display}"),
                                color: theme.fg,
                            )
                        }
                    }
                }
            }
            .into()
        }

        2 => {
            // Project selection — no TextInput, fully manual rendering
            let uname = user_name.read().clone();
            let all_projects = projects.read().clone();
            let search_val = search_text.read().clone();
            let searching = search_active.get();

            let filtered: Vec<(String, bool)> = if search_val.is_empty() {
                all_projects.clone()
            } else {
                let lower = search_val.to_lowercase();
                all_projects
                    .iter()
                    .filter(|(n, _)| n.to_lowercase().contains(&lower))
                    .cloned()
                    .collect()
            };

            let proj_visible_h = height.saturating_sub(12) as usize;
            let offset = project_scroll.get();
            let end = (offset + proj_visible_h.max(1)).min(filtered.len());
            let idx = project_idx.get();

            let visible: Vec<(bool, String, bool)> = if end > offset {
                filtered[offset..end]
                    .iter()
                    .enumerate()
                    .map(|(i, (name, sel))| (offset + i == idx, name.clone(), *sel))
                    .collect()
            } else {
                Vec::new()
            };

            let selected_count =
                all_projects.iter().filter(|(_, sel)| *sel).count();
            let total_count = all_projects.len();
            let filtered_count = filtered.len();
            let header = format!(
                "Monitor extra projects ({selected_count} of {total_count} selected, {filtered_count} shown)"
            );
            let sub_header = "PRs you're assigned to review are always shown. Select projects to also monitor all their PRs.";

            let search_color = if searching {
                theme.accent
            } else {
                theme.fg_muted
            };

            let hints = if searching {
                "type to filter  Enter done  Esc clear"
            } else {
                "j/k move  space/x toggle  a all  / search  Enter save (skip ok)  Esc back"
            };

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    padding: 1,
                ) {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: theme.border,
                        padding: 1,
                        flex_grow: 1.0,
                    ) {
                        View() {
                            Text(content: "✓ ", color: theme.success)
                            Text(
                                content: format!("Connected as {uname}"),
                                color: theme.fg,
                            )
                        }
                        Text(
                            content: sub_header,
                            color: theme.fg_dim,
                        )
                        Text(
                            content: header,
                            color: theme.fg,
                            weight: Weight::Bold,
                        )
                        View(flex_direction: FlexDirection::Row) {
                            Text(content: "/ ", color: search_color)
                            TextInput(
                                has_focus: searching,
                                value: search_val.clone(),
                                on_change: move |val: String| {
                                    search_text.set(val);
                                    project_idx.set(0);
                                    project_scroll.set(0);
                                },
                            )
                        }

                        View(
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            overflow: Overflow::Hidden,
                            margin_top: 1,
                        ) {
                            #(visible.into_iter().map(|(is_cur, name, checked)| {
                                let marker = if is_cur { "▸" } else { " " };
                                let checkbox = if checked { "[x]" } else { "[ ]" };
                                let bg = if is_cur {
                                    Some(theme.selection_bg)
                                } else {
                                    None
                                };
                                let fg = if checked { theme.accent } else { theme.fg };
                                element! {
                                    View(background_color: bg) {
                                        Text(
                                            content: format!(
                                                " {marker} {checkbox} {name}"
                                            ),
                                            color: fg,
                                        )
                                    }
                                }
                            }))
                        }

                        Text(content: err.clone(), color: theme.error)
                        Text(content: hints, color: theme.fg_muted)
                    }
                }
            }
            .into()
        }

        3 => {
            let url = org_url.read().clone();
            let all_projects = projects.read().clone();
            let selected_count =
                all_projects.iter().filter(|(_, sel)| *sel).count();
            let selected_names: Vec<String> = all_projects
                .iter()
                .filter(|(_, sel)| *sel)
                .map(|(n, _)| n.clone())
                .collect();
            let provider_name = url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("default")
                .to_lowercase();
            let names_preview = if selected_names.is_empty() {
                "None — showing only PRs you're assigned to".to_string()
            } else if selected_names.len() <= 5 {
                selected_names.join(", ")
            } else {
                format!(
                    "{}, ... and {} more",
                    selected_names[..4].join(", "),
                    selected_names.len() - 4
                )
            };

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                ) {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: theme.success,
                        padding: 2,
                        width: 70pct,
                    ) {
                        Text(
                            content: "✓ Configuration saved!",
                            color: theme.success,
                            weight: Weight::Bold,
                        )
                        View(margin_top: 1) {
                            Text(content: "Provider: ", color: theme.fg_dim)
                            Text(content: provider_name, color: theme.fg)
                        }
                        View() {
                            Text(content: "Extra projects: ", color: theme.fg_dim)
                            Text(
                                content: if selected_count > 0 {
                                    format!("{selected_count} selected")
                                } else {
                                    "none".to_string()
                                },
                                color: theme.fg,
                            )
                        }
                        View() {
                            Text(
                                content: names_preview,
                                color: theme.fg_dim,
                            )
                        }
                        View(margin_top: 2) {
                            Text(
                                content: "Press Enter to launch ridgeline",
                                color: theme.accent,
                            )
                        }
                    }
                }
            }
            .into()
        }

        _ => element! { View {} }.into(),
    };

    element! {
        View(width, height, flex_direction: FlexDirection::Column) {
            #(std::iter::once(content))
        }
    }
}
