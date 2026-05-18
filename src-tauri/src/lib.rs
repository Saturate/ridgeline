mod commands;
mod config;
mod notifications;
mod polling;
mod providers;
mod state;
mod util;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config::loader::load_config().unwrap_or_else(|_| config::model::Config {
        general: Default::default(),
        providers: Vec::new(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new(config))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::store_token,
            commands::delete_token,
            commands::test_connection,
            commands::init_providers,
            commands::poll_all,
            commands::get_pr_detail,
            commands::list_projects,
            commands::open_url,
            commands::start_polling,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
