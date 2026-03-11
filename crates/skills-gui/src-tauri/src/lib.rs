mod commands;

use commands::AppState;
use skills_core::{AppDirs, Database};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let (dirs, db) = rt.block_on(async {
        let base = AppDirs::default_base().expect("Failed to determine home directory");
        let dirs = AppDirs::new(base);
        dirs.ensure_dirs()
            .expect("Failed to create app directories");
        let db = Database::open(&dirs.database())
            .await
            .expect("Failed to open database");
        (dirs, db)
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { dirs, db })
        .invoke_handler(tauri::generate_handler![
            commands::list_skills,
            commands::create_skill,
            commands::remove_skill,
            commands::list_profiles,
            commands::create_profile,
            commands::edit_profile,
            commands::delete_profile,
            commands::list_agents,
            commands::add_agent,
            commands::edit_agent,
            commands::remove_agent,
            commands::get_status,
            commands::activate_profile,
            commands::deactivate_profile,
            commands::get_recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("Error running tauri application");
}
