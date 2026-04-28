#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;
use ygocmg_core::bootstrap::wiring::build_app_state_with_event_bus;
use ygocmg_core::infrastructure::tauri_event_bus::TauriEventBus;
use ygocmg_core::tauri_commands;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|source| source.to_string())?;
            let event_bus = Arc::new(TauriEventBus::new(app.handle().clone()));
            let state = build_app_state_with_event_bus(PathBuf::from(app_data_dir), event_bus)
                .map_err(|source| source.to_string())?;
            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
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
            tauri_commands::close_pack,
            tauri_commands::set_active_pack,
            tauri_commands::update_pack_metadata,
            tauri_commands::delete_pack,
            tauri_commands::list_pack_overviews,
            tauri_commands::list_cards,
            tauri_commands::get_card,
            tauri_commands::create_card,
            tauri_commands::update_card,
            tauri_commands::delete_card,
            tauri_commands::confirm_card_write,
            tauri_commands::suggest_card_code,
            tauri_commands::list_pack_strings,
            tauri_commands::get_pack_string,
            tauri_commands::upsert_pack_string,
            tauri_commands::upsert_pack_string_record,
            tauri_commands::delete_pack_strings,
            tauri_commands::remove_pack_string_translation,
            tauri_commands::confirm_pack_strings_write,
            tauri_commands::import_main_image,
            tauri_commands::delete_main_image,
            tauri_commands::import_field_image,
            tauri_commands::delete_field_image,
            tauri_commands::create_empty_script,
            tauri_commands::import_script,
            tauri_commands::delete_script,
            tauri_commands::open_script_external,
            tauri_commands::preview_export_bundle,
            tauri_commands::get_standard_pack_status,
            tauri_commands::rebuild_standard_pack_index,
            tauri_commands::search_standard_cards,
            tauri_commands::search_standard_strings,
            tauri_commands::get_standard_card,
            tauri_commands::get_job_status,
            tauri_commands::list_active_jobs
        ])
        .run(tauri::generate_context!())
        .expect("failed to run YGOCMG");
}
