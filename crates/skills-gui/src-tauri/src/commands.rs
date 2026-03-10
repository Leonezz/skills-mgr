use skills_core::{AppDirs, Database, Registry};
use skills_core::config::{AgentsConfig, ProfilesConfig};
use skills_core::placements;
use tauri::State;
use serde::Serialize;

pub struct AppState {
    pub dirs: AppDirs,
    pub db: Database,
}

#[derive(Serialize)]
pub struct SkillInfo {
    name: String,
    description: Option<String>,
    files: Vec<String>,
    source_type: Option<String>,
}

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillInfo>, String> {
    let registry = Registry::new(state.dirs.clone());
    let skills = registry.list().map_err(|e| e.to_string())?;
    Ok(skills.into_iter().map(|s| SkillInfo {
        name: s.name,
        description: s.description,
        files: s.files,
        source_type: s.source.map(|src| format!("{:?}", src.source_type).to_lowercase()),
    }).collect())
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>, project_path: String) -> Result<serde_json::Value, String> {
    let profiles_config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let s = placements::status(&state.db, &profiles_config, &project_path).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "project_path": s.project_path,
        "base_skills": s.base_skills,
        "active_profiles": s.active_profiles,
        "placement_count": s.placement_count,
    }))
}

#[tauri::command]
pub async fn activate_profile(
    state: State<'_, AppState>,
    profile_name: String,
    project_path: String,
    force: bool,
) -> Result<String, String> {
    let profiles_config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let result = placements::activate(&state.dirs, &state.db, &profiles_config, &agents_config, &profile_name, &project_path, force)
        .await.map_err(|e| e.to_string())?;
    Ok(format!("Activated '{}': {} skills, {} placements", result.profile_name, result.skills_placed, result.total_placements))
}

#[tauri::command]
pub async fn deactivate_profile(
    state: State<'_, AppState>,
    profile_name: String,
    project_path: String,
) -> Result<String, String> {
    let result = placements::deactivate(&state.db, &profile_name, &project_path)
        .await.map_err(|e| e.to_string())?;
    Ok(format!("Deactivated '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept))
}

#[tauri::command]
pub async fn get_recent_logs(state: State<'_, AppState>, limit: i64) -> Result<serde_json::Value, String> {
    let logs = state.db.get_recent_logs(limit).await.map_err(|e| e.to_string())?;
    let entries: Vec<serde_json::Value> = logs.into_iter().map(|l| serde_json::json!({
        "id": l.id,
        "timestamp": l.timestamp,
        "source": l.source,
        "agent_name": l.agent_name,
        "operation": l.operation,
        "result": l.result,
        "details": l.details,
    })).collect();
    Ok(serde_json::Value::Array(entries))
}
