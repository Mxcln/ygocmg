#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use tauri::Manager;
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::tauri_commands;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|source| source.to_string())?;
            let state = build_app_state(PathBuf::from(app_data_dir)).map_err(|source| source.to_string())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tauri_commands::initialize,
            tauri_commands::load_config,
            tauri_commands::save_config,
            tauri_commands::list_recent_workspaces,
            tauri_commands::create_workspace,
            tauri_commands::open_workspace,
            tauri_commands::delete_workspace,
            tauri_commands::create_pack,
            tauri_commands::open_pack,
            tauri_commands::list_pack_overviews,
            tauri_commands::list_cards,
            tauri_commands::create_card,
            tauri_commands::update_card,
            tauri_commands::delete_card,
            tauri_commands::suggest_card_code
        ])
        .run(tauri::generate_context!())
        .expect("failed to run YGOCMG");
}
