// Release builds on Windows would otherwise open a console next to the window.
// --mcp-stdio still works: MCP clients start us with piped handles.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;
mod log;
mod palette;
mod pr_row;
mod settings;
mod settings_window;
mod theme;

use std::future::Future;
use std::sync::OnceLock;

use gpui_kit::*;

actions!(
    ridgeline,
    [
        Quit,
        Refresh,
        OpenSettings,
        CommandPalette,
        NextTab,
        PreviousTab,
        OpenInBrowser,
        CycleTheme,
    ]
);

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn io<T: Send + 'static>(
    future: impl Future<Output = anyhow::Result<T>> + Send + 'static,
) -> impl Future<Output = anyhow::Result<T>> {
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("cannot start the network runtime: {e}"))
    });
    struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);
    impl<T> Drop for AbortOnDrop<T> {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let mut handle = AbortOnDrop(runtime.spawn(future));
    async move {
        (&mut handle.0)
            .await
            .map_err(|e| anyhow::anyhow!("background task stopped: {e}"))?
    }
}

fn window_size() -> Size<Pixels> {
    size(px(1100.), px(720.))
}

fn main() {
    log::init();

    if std::env::args().any(|a| a == "--mcp-stdio") {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
            unsafe {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(ridgeline_core::mcp::run_stdio())
            .unwrap();
        return;
    }

    gpui_kit::application()
        .with_assets(assets::AppAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            theme::install(cx);
            cx.set_global(settings::Settings::load());
            settings::Settings::apply(cx);

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.set_menus(vec![Menu {
                name: "Ridgeline".into(),
                items: vec![
                    MenuItem::action("Command Palette...", CommandPalette),
                    MenuItem::action("Settings...", OpenSettings),
                    MenuItem::separator(),
                    MenuItem::action("Quit Ridgeline", Quit),
                ],
                disabled: false,
            }]);
            cx.bind_keys([
                KeyBinding::new("cmd-k", CommandPalette, None),
                KeyBinding::new("cmd-,", OpenSettings, None),
                KeyBinding::new("cmd-r", Refresh, None),
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-]", NextTab, None),
                KeyBinding::new("cmd-[", PreviousTab, None),
                KeyBinding::new("cmd-o", OpenInBrowser, None),
            ]);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(window_size(), cx)),
                window_min_size: Some(size(px(720.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Ridgeline".into()),
                    ..gpui_kit::component::TitleBar::title_bar_options()
                }),
                ..gpui_kit::component::TitleBar::window_options()
            };
            if let Err(e) = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| app::Ridgeline::new(window, cx))
            }) {
                eprintln!("ridgeline: cannot open window: {e}");
                cx.quit();
            }
        });
}
