mod commands;

use commands::AppState;
use skills_core::config::merge_hubs;
use skills_core::{AppDirs, Database, ProviderRegistry};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn init_tracing(log_dir: &std::path::Path) {
    let file_appender = tracing_appender::rolling::daily(log_dir, "skills-gui.log");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,reqwest=warn,hyper=warn"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_ansi(false).with_writer(file_appender))
        .init();

    // Bridge log crate (used by Tauri, sqlx, reqwest) into tracing
    tracing_log::LogTracer::init().ok();
}

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

    // Initialize tracing to stderr + rolling log file
    let log_dir = dirs.base().join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    init_tracing(&log_dir);

    tracing::info!("skills-gui starting up, base_dir={}", dirs.base().display());

    // Build provider registry: merge built-in hubs with user-configured hubs
    let all_hubs = merge_hubs(&dirs.settings_toml());
    let providers = ProviderRegistry::with_hubs(&all_hubs);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            dirs,
            db,
            providers,
        })
        // TODO: wire up settings.scan.auto_scan_on_startup to trigger
        // scan_skills on app launch when enabled
        .invoke_handler(tauri::generate_handler![
            commands::list_skills,
            commands::create_skill,
            commands::import_skill,
            commands::import_remote_skill,
            commands::browse_remote,
            commands::import_from_browse,
            commands::open_skill_dir,
            commands::remove_skill,
            commands::read_skill_content,
            commands::update_skill,
            commands::sync_skill,
            commands::sync_all_skills,
            commands::list_profiles,
            commands::create_profile,
            commands::duplicate_profile,
            commands::edit_profile,
            commands::delete_profile,
            commands::list_agents,
            commands::add_agent,
            commands::list_agent_presets,
            commands::edit_agent,
            commands::remove_agent,
            commands::toggle_agent,
            commands::get_global_status,
            commands::activate_global,
            commands::deactivate_global,
            commands::edit_global_skills,
            commands::get_status,
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::link_profile_to_project,
            commands::unlink_profile_from_project,
            commands::activate_project,
            commands::deactivate_project,
            commands::get_project_detail,
            commands::reveal_path,
            commands::activate_profile,
            commands::deactivate_profile,
            commands::switch_profile,
            commands::scan_skills,
            commands::delegate_skills,
            commands::link_remote,
            commands::unlink_remote,
            commands::get_settings,
            commands::save_settings,
            commands::get_recent_logs,
            commands::list_hubs,
            commands::browse_hub,
        ])
        .run(tauri::generate_context!())
        .expect("Error running tauri application");
}
