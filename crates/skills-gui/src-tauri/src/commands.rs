use skills_core::{AppDirs, Database, Registry};
use skills_core::config::{AgentDef, AgentsConfig, ProfileDef, ProfilesConfig};
use skills_core::logging::{self, Source};
use skills_core::placements;
use skills_core::profiles;
use tauri::State;
use serde::Serialize;

pub struct AppState {
    pub dirs: AppDirs,
    pub db: Database,
}

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub files: Vec<String>,
    pub source_type: Option<String>,
}

#[derive(Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub includes: Vec<String>,
}

#[derive(Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub project_path: String,
    pub global_path: String,
}

#[derive(Serialize)]
pub struct StatusInfo {
    pub project_path: String,
    pub base_skills: Vec<String>,
    pub active_profiles: Vec<String>,
    pub placement_count: usize,
}

// --- Skills ---

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
pub async fn create_skill(state: State<'_, AppState>, name: String, description: String) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry.create(&name, &description).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "skill_create", None, None, "success", &format!("Created skill '{}'", name)).await;
    Ok(format!("Created skill '{}'", name))
}

#[tauri::command]
pub async fn remove_skill(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry.remove(&name).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "skill_remove", None, None, "success", &format!("Removed skill '{}'", name)).await;
    Ok(format!("Removed skill '{}'", name))
}

// --- Profiles ---

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let mut result_profiles: Vec<ProfileInfo> = Vec::new();
    for (name, profile) in &config.profiles {
        result_profiles.push(ProfileInfo {
            name: name.clone(),
            description: profile.description.clone(),
            skills: profile.skills.clone(),
            includes: profile.includes.clone(),
        });
    }
    Ok(serde_json::json!({
        "base": { "skills": config.base.skills },
        "profiles": result_profiles,
    }))
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    name: String,
    skills: Vec<String>,
    includes: Vec<String>,
    description: Option<String>,
) -> Result<String, String> {
    let mut config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let profile = ProfileDef {
        description,
        skills,
        includes,
    };
    config.profiles.insert(name.clone(), profile);
    profiles::validate_no_cycles(&config).map_err(|e| e.to_string())?;
    config.save(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "profile_create", None, None, "success", &format!("Created profile '{}'", name)).await;
    Ok(format!("Created profile '{}'", name))
}

#[tauri::command]
pub async fn edit_profile(
    state: State<'_, AppState>,
    name: String,
    add_skills: Vec<String>,
    remove_skills: Vec<String>,
    add_includes: Vec<String>,
    description: Option<String>,
) -> Result<String, String> {
    let mut config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let profile = config.profiles.get_mut(&name)
        .ok_or_else(|| format!("Profile '{}' not found", name))?;
    if let Some(desc) = description {
        profile.description = Some(desc);
    }
    for s in &add_skills {
        if !profile.skills.contains(s) { profile.skills.push(s.clone()); }
    }
    profile.skills.retain(|s| !remove_skills.contains(s));
    for i in &add_includes {
        if !profile.includes.contains(i) { profile.includes.push(i.clone()); }
    }
    profiles::validate_no_cycles(&config).map_err(|e| e.to_string())?;
    config.save(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "profile_edit", None, None, "success", &format!("Updated profile '{}'", name)).await;
    Ok(format!("Updated profile '{}'", name))
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let mut config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    if config.profiles.remove(&name).is_none() {
        return Err(format!("Profile '{}' not found", name));
    }
    config.save(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "profile_delete", None, None, "success", &format!("Deleted profile '{}'", name)).await;
    Ok(format!("Deleted profile '{}'", name))
}

// --- Agents ---

#[tauri::command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<Vec<AgentInfo>, String> {
    let config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    Ok(config.agents.into_iter().map(|(name, def)| AgentInfo {
        name,
        project_path: def.project_path,
        global_path: def.global_path,
    }).collect())
}

#[tauri::command]
pub async fn add_agent(
    state: State<'_, AppState>,
    name: String,
    project_path: String,
    global_path: String,
) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    config.agents.insert(name.clone(), AgentDef { project_path, global_path });
    config.save(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "agent_add", None, None, "success", &format!("Added agent '{}'", name)).await;
    Ok(format!("Added agent '{}'", name))
}

#[tauri::command]
pub async fn edit_agent(
    state: State<'_, AppState>,
    name: String,
    project_path: String,
    global_path: String,
) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    if !config.agents.contains_key(&name) {
        return Err(format!("Agent '{}' not found", name));
    }
    config.agents.insert(name.clone(), AgentDef { project_path, global_path });
    config.save(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "agent_edit", None, None, "success", &format!("Updated agent '{}'", name)).await;
    Ok(format!("Updated agent '{}'", name))
}

#[tauri::command]
pub async fn remove_agent(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    if config.agents.remove(&name).is_none() {
        return Err(format!("Agent '{}' not found", name));
    }
    config.save(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let _ = logging::log(&state.db, Source::Gui, None, "agent_remove", None, None, "success", &format!("Removed agent '{}'", name)).await;
    Ok(format!("Removed agent '{}'", name))
}

// --- Status & Placements ---

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>, project_path: String) -> Result<StatusInfo, String> {
    let profiles_config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let s = placements::status(&state.db, &profiles_config, &project_path).await.map_err(|e| e.to_string())?;
    Ok(StatusInfo {
        project_path: s.project_path,
        base_skills: s.base_skills,
        active_profiles: s.active_profiles,
        placement_count: s.placement_count,
    })
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
    let _ = logging::log(&state.db, Source::Gui, None, "profile_activate", None, Some(&project_path), "success", &format!("Activated '{}': {} placements", profile_name, result.total_placements)).await;
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
    let _ = logging::log(&state.db, Source::Gui, None, "profile_deactivate", None, Some(&project_path), "success", &format!("Deactivated '{}': {} removed", profile_name, result.files_removed)).await;
    Ok(format!("Deactivated '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept))
}

// --- Logs ---

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
