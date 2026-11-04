use gpui_kit::component::{
    WindowExt as _,
    command::{Command, CommandGroup, CommandItem, CommandState},
};
use gpui_kit::*;

use crate::app::Ridgeline;

pub fn open(app: &mut Ridgeline, window: &mut Window, cx: &mut Context<Ridgeline>) {
    let palette = cx.new(|cx| CommandState::new(window, cx));

    let pr_labels: Vec<String> = app
        .reviewing
        .iter()
        .chain(app.authored.iter())
        .map(|pr| pr.title.clone())
        .collect();

    window.open_dialog(cx, move |dialog, _, _| {
        let palette = palette.clone();
        let pr_labels = pr_labels.clone();

        dialog
            .close_button(false)
            .p_0()
            .w(px(520.))
            .content(move |content, _window, _cx| {
                let pr_items: Vec<CommandItem> = pr_labels
                    .iter()
                    .map(|title| {
                        CommandItem::new()
                            .label(title.clone())
                            .keywords(vec![SharedString::from(title.clone())])
                    })
                    .collect();

                let command_items = vec![
                    CommandItem::new()
                        .label("Refresh")
                        .keywords(vec![SharedString::from("refresh")]),
                    CommandItem::new()
                        .label("Settings")
                        .keywords(vec![SharedString::from("settings")]),
                    CommandItem::new()
                        .label("Open in Browser")
                        .keywords(vec![SharedString::from("open")]),
                ];

                let command = Command::new(&palette)
                    .group(
                        CommandGroup::new()
                            .label("Commands")
                            .items(command_items),
                    )
                    .group(
                        CommandGroup::new()
                            .label("Pull Requests")
                            .items(pr_items),
                    )
                    .bordered(false)
                    .max_h(px(400.))
                    .placeholder("Search commands and PRs...")
                    .on_confirm(move |index, _window, cx| {
                        if index.section == 0 {
                            match index.row {
                                0 => cx.dispatch_action(&crate::Refresh),
                                1 => cx.dispatch_action(&crate::OpenSettings),
                                2 => cx.dispatch_action(&crate::OpenInBrowser),
                                _ => {}
                            }
                        }
                    });

                content.child(command)
            })
    });
}
