use serde::{Deserialize, Serialize};
use skills_core::config::{AgentDef, AgentsConfig, AppSettings, ProfileDef, ProfilesConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::placements;
use skills_core::placements::GLOBAL_PROJECT_PATH;
use skills_core::profiles;
use skills_core::{AppDirs, Database, Registry};
use tauri::State;

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
    pub source_url: Option<String>,
    pub source_ref: Option<String>,
    pub is_builtin: bool,
    pub dir_path: String,
    pub total_bytes: u64,
    pub token_estimate: u64,
}

#[derive(Serialize)]
pub struct ActiveProject {
    pub path: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub includes: Vec<String>,
    pub active_projects: Vec<ActiveProject>,
}

#[derive(Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub project_path: String,
    pub global_path: String,
    pub enabled: bool,
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
    const BUILTIN_SKILLS: &[&str] = &["skills-mgr-guide"];
    Ok(skills
        .into_iter()
        .map(|s| {
            let is_builtin = BUILTIN_SKILLS.contains(&s.name.as_str());
            let dir_path = s.dir_path.to_string_lossy().to_string();
            let source_type = s
                .source
                .as_ref()
                .map(|src| format!("{:?}", src.source_type).to_lowercase());
            let source_url = s.source.as_ref().and_then(|src| src.url.clone());
            let source_ref = s.source.as_ref().and_then(|src| src.git_ref.clone());
            SkillInfo {
                name: s.name,
                description: s.description,
                files: s.files,
                source_type,
                source_url,
                source_ref,
                is_builtin,
                dir_path,
                total_bytes: s.total_bytes,
                token_estimate: s.token_estimate,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn create_skill(
    state: State<'_, AppState>,
    name: String,
    description: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry
        .create(&name, &description)
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_create",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Created skill '{}'", name),
        },
    )
    .await;
    Ok(format!("Created skill '{}'", name))
}

#[tauri::command]
pub async fn remove_skill(state: State<'_, AppState>, name: String) -> Result<String, String> {
    const BUILTIN_SKILLS: &[&str] = &["skills-mgr-guide"];
    if BUILTIN_SKILLS.contains(&name.as_str()) {
        return Err(format!("Cannot delete built-in skill '{}'", name));
    }
    let registry = Registry::new(state.dirs.clone());
    registry.remove(&name).map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_remove",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Removed skill '{}'", name),
        },
    )
    .await;
    Ok(format!("Removed skill '{}'", name))
}

#[tauri::command]
pub async fn import_skill(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    let name = registry
        .add_from_local(std::path::Path::new(&source_path))
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_import",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Imported skill '{}' from {}", name, source_path),
        },
    )
    .await;
    Ok(format!("Imported skill '{}'", name))
}

#[tauri::command]
pub async fn import_remote_skill(
    state: State<'_, AppState>,
    url: String,
) -> Result<String, String> {
    tracing::info!(url = %url, "Starting remote skill import");
    let registry = Registry::new(state.dirs.clone());
    let params_json = serde_json::json!({ "url": url });
    match registry.add_from_remote(&url).await {
        Ok(name) => {
            tracing::info!(skill = %name, "Remote skill imported successfully");
            let _ = logging::log(
                &state.db,
                LogEntry {
                    source: Source::Gui,
                    agent_name: None,
                    operation: "skill_import_remote",
                    params: Some(&params_json),
                    project_path: None,
                    result: "success",
                    details: &format!("Imported skill '{}' from {}", name, url),
                },
            )
            .await;
            Ok(format!("Imported skill '{}'", name))
        }
        Err(e) => {
            let err_msg = e.to_string();
            tracing::error!(url = %url, error = %err_msg, "Remote skill import failed");
            let _ = logging::log(
                &state.db,
                LogEntry {
                    source: Source::Gui,
                    agent_name: None,
                    operation: "skill_import_remote",
                    params: Some(&params_json),
                    project_path: None,
                    result: "error",
                    details: &format!("Failed to import from {}: {}", url, err_msg),
                },
            )
            .await;
            Err(err_msg)
        }
    }
}

#[tauri::command]
pub async fn browse_remote(
    state: State<'_, AppState>,
    url: String,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!(url = %url, "Browsing remote repo");
    let source = skills_core::remote::parse_github_url(&url).map_err(|e| e.to_string())?;
    let staging_dir = state.dirs.cache().join("staging");

    let skills = skills_core::remote::download_to_staging(&source, &staging_dir)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(count = skills.len(), "Found skills in remote repo");
    Ok(skills
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "description": s.description,
                "subpath": s.subpath,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn import_from_browse(
    state: State<'_, AppState>,
    subpaths: Vec<String>,
) -> Result<String, String> {
    let staging_dir = state.dirs.cache().join("staging");
    let meta_path = staging_dir.join("meta.json");
    let repo_dir = staging_dir.join("repo");

    if !meta_path.exists() {
        return Err("No browse session active. Please browse a repository first.".to_string());
    }

    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let owner = meta["owner"].as_str().unwrap_or_default();
    let repo = meta["repo"].as_str().unwrap_or_default();
    let git_ref = meta["git_ref"].as_str().unwrap_or("main");

    let registry = Registry::new(state.dirs.clone());
    let mut imported: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for subpath in &subpaths {
        let source_dir = repo_dir.join(subpath);
        let skill_name = source_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        tracing::info!(skill = %skill_name, subpath = %subpath, "Importing from browse");
        match registry.import_from_extracted_dir(
            &source_dir,
            &skill_name,
            owner,
            repo,
            git_ref,
            subpath,
        ) {
            Ok(()) => imported.push(skill_name),
            Err(e) => {
                tracing::error!(skill = %skill_name, error = %e, "Failed to import");
                errors.push(format!("{}: {}", skill_name, e));
            }
        }
    }

    // Clean up staging
    let _ = std::fs::remove_dir_all(&staging_dir);

    // Log to DB
    let details = if errors.is_empty() {
        format!(
            "Imported {} skills: {}",
            imported.len(),
            imported.join(", ")
        )
    } else {
        format!(
            "Imported {}, failed {}: {}",
            imported.len(),
            errors.len(),
            errors.join("; ")
        )
    };
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_import_remote_batch",
            params: None,
            project_path: None,
            result: if errors.is_empty() {
                "success"
            } else {
                "partial"
            },
            details: &details,
        },
    )
    .await;

    if !errors.is_empty() {
        return Err(format!(
            "Imported {}, failed {}: {}",
            imported.len(),
            errors.len(),
            errors[0]
        ));
    }
    Ok(format!(
        "Imported {} skill{}",
        imported.len(),
        if imported.len() != 1 { "s" } else { "" }
    ))
}

#[tauri::command]
pub async fn open_skill_dir(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let skill_dir = state.dirs.registry().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill directory not found: {}", name));
    }
    tracing::info!(skill = %name, "Opening skill directory");

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&skill_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&skill_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(&skill_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn read_skill_content(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry.read_content(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_skill(
    state: State<'_, AppState>,
    name: String,
    description: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry
        .update_description(&name, &description)
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_update",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Updated skill '{}'", name),
        },
    )
    .await;
    Ok(format!("Updated skill '{}'", name))
}

// --- Discovery & Delegation ---

#[derive(Serialize)]
pub struct DiscoveredSkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub agent_name: String,
    pub found_path: String,
    pub scope: String,
    pub files: Vec<String>,
    pub total_bytes: u64,
    pub token_estimate: u64,
    pub exists_in_registry: bool,
}

#[tauri::command]
pub async fn scan_skills(state: State<'_, AppState>) -> Result<Vec<DiscoveredSkillInfo>, String> {
    let dirs = &state.dirs;
    let registry = Registry::new(dirs.clone());
    let agents_config =
        skills_core::AgentsConfig::load(&dirs.agents_toml()).map_err(|e| e.to_string())?;

    let all_projects = state
        .db
        .list_all_projects()
        .await
        .map_err(|e| e.to_string())?;

    let project_paths: Vec<String> = all_projects
        .iter()
        .filter(|p| p.path != GLOBAL_PROJECT_PATH)
        .map(|p| p.path.clone())
        .collect();

    // Collect all placed skill paths from DB to exclude from discovery
    let placed_paths = state
        .db
        .collect_placed_paths()
        .await
        .map_err(|e| e.to_string())?;

    let discovered = skills_core::discovery::scan_all_agents(
        dirs,
        &registry,
        &agents_config,
        &project_paths,
        &placed_paths,
    )
    .map_err(|e| e.to_string())?;

    Ok(discovered
        .into_iter()
        .map(|d| {
            let scope = match &d.scope {
                skills_core::discovery::DiscoveryScope::Global => "global".to_string(),
                skills_core::discovery::DiscoveryScope::Project(p) => p.clone(),
            };
            DiscoveredSkillInfo {
                name: d.name,
                description: d.description,
                agent_name: d.agent_name,
                found_path: d.found_path.to_string_lossy().to_string(),
                scope,
                files: d.files,
                total_bytes: d.total_bytes,
                token_estimate: d.token_estimate,
                exists_in_registry: d.exists_in_registry,
            }
        })
        .collect())
}

#[derive(Deserialize)]
pub struct DelegateRequest {
    pub found_path: String,
}

#[tauri::command]
pub async fn delegate_skills(
    state: State<'_, AppState>,
    skills: Vec<DelegateRequest>,
    profile_name: String,
    create_profile: bool,
    profile_description: Option<String>,
) -> Result<String, String> {
    let dirs = &state.dirs;
    let registry = Registry::new(dirs.clone());

    // Validate or create profile BEFORE delegating any skills
    let mut profiles_config =
        ProfilesConfig::load(&dirs.profiles_toml()).map_err(|e| e.to_string())?;
    if create_profile {
        if profiles_config.profiles.contains_key(&profile_name) {
            return Err(format!("Profile '{}' already exists", profile_name));
        }
        profiles_config.profiles.insert(
            profile_name.clone(),
            ProfileDef {
                description: profile_description,
                skills: vec![],
                includes: vec![],
            },
        );
        profiles_config
            .save(&dirs.profiles_toml())
            .map_err(|e| e.to_string())?;
    } else if !profiles_config.profiles.contains_key(&profile_name) {
        return Err(format!("Profile '{}' not found", profile_name));
    }

    let mut delegated = Vec::new();
    let mut skipped = Vec::new();
    for req in &skills {
        let source_path = std::path::PathBuf::from(&req.found_path);
        let skill_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if registry.exists(&skill_name) {
            skipped.push(skill_name);
            continue;
        }
        match registry.delegate(&source_path, &req.found_path) {
            Ok(name) => {
                if let Some(profile) = profiles_config.profiles.get_mut(&profile_name) {
                    if !profile.skills.contains(&name) {
                        profile.skills.push(name.clone());
                    }
                    profiles_config
                        .save(&dirs.profiles_toml())
                        .map_err(|e| e.to_string())?;
                }
                delegated.push(name);
            }
            Err(e) => {
                return Err(format!(
                    "{}{}",
                    e,
                    if delegated.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " ({} already imported and assigned: {})",
                            delegated.len(),
                            delegated.join(", ")
                        )
                    }
                ));
            }
        }
    }

    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_delegate",
            params: None,
            project_path: None,
            result: "success",
            details: &format!(
                "Delegated {} skill(s) to profile '{}'",
                delegated.len(),
                profile_name
            ),
        },
    )
    .await;

    let mut msg = format!(
        "Delegated {} skill(s) to profile '{}'",
        delegated.len(),
        profile_name
    );
    if !skipped.is_empty() {
        msg.push_str(&format!(
            " ({} already in registry: {})",
            skipped.len(),
            skipped.join(", ")
        ));
    }
    Ok(msg)
}

#[tauri::command]
pub async fn link_remote(
    state: State<'_, AppState>,
    name: String,
    url: String,
    subpath: Option<String>,
    git_ref: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry
        .link_remote(&name, &url, subpath.as_deref(), &git_ref)
        .map_err(|e| e.to_string())?;

    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_link_remote",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Linked '{}' to {}", name, url),
        },
    )
    .await;

    Ok(format!("Linked '{}' to remote: {}", name, url))
}

#[tauri::command]
pub async fn unlink_remote(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry.unlink_remote(&name).map_err(|e| e.to_string())?;

    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "skill_unlink_remote",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Unlinked '{}' from remote", name),
        },
    )
    .await;

    Ok(format!("Unlinked '{}' from remote", name))
}

// --- Profiles ---

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let mut result_profiles: Vec<ProfileInfo> = Vec::new();
    for (name, profile) in &config.profiles {
        let projects = state
            .db
            .get_projects_for_profile(name)
            .await
            .unwrap_or_default();
        let active_projects: Vec<ActiveProject> = projects
            .into_iter()
            .map(|(path, name)| {
                let display = name.unwrap_or_else(|| path.clone());
                ActiveProject {
                    path,
                    name: display,
                }
            })
            .collect();
        result_profiles.push(ProfileInfo {
            name: name.clone(),
            description: profile.description.clone(),
            skills: profile.skills.clone(),
            includes: profile.includes.clone(),
            active_projects,
        });
    }
    let global_status = placements::global_status(&state.db, &config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "base": { "skills": config.base.skills },
        "global": {
            "skills": global_status.configured_skills,
            "placed_skills": global_status.placed_skills,
            "is_active": global_status.is_active,
        },
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
    let mut config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let profile = ProfileDef {
        description,
        skills,
        includes,
    };
    config.profiles.insert(name.clone(), profile);
    profiles::validate_no_cycles(&config).map_err(|e| e.to_string())?;
    config
        .save(&state.dirs.profiles_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_create",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Created profile '{}'", name),
        },
    )
    .await;
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
    let mut config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let profile = config
        .profiles
        .get_mut(&name)
        .ok_or_else(|| format!("Profile '{}' not found", name))?;
    if let Some(desc) = description {
        profile.description = Some(desc);
    }
    for s in &add_skills {
        if !profile.skills.contains(s) {
            profile.skills.push(s.clone());
        }
    }
    profile.skills.retain(|s| !remove_skills.contains(s));
    for i in &add_includes {
        if !profile.includes.contains(i) {
            profile.includes.push(i.clone());
        }
    }
    profiles::validate_no_cycles(&config).map_err(|e| e.to_string())?;
    config
        .save(&state.dirs.profiles_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_edit",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Updated profile '{}'", name),
        },
    )
    .await;
    Ok(format!("Updated profile '{}'", name))
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let mut config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    if config.profiles.remove(&name).is_none() {
        return Err(format!("Profile '{}' not found", name));
    }
    config
        .save(&state.dirs.profiles_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_delete",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Deleted profile '{}'", name),
        },
    )
    .await;
    Ok(format!("Deleted profile '{}'", name))
}

// --- Agents ---

#[tauri::command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<Vec<AgentInfo>, String> {
    let config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    Ok(config
        .agents
        .into_iter()
        .map(|(name, def)| AgentInfo {
            name,
            project_path: def.project_path,
            global_path: def.global_path,
            enabled: def.enabled,
        })
        .collect())
}

#[tauri::command]
pub async fn add_agent(
    state: State<'_, AppState>,
    name: String,
    project_path: Option<String>,
    global_path: Option<String>,
) -> Result<String, String> {
    let (pp, gp) = match (project_path, global_path) {
        (Some(pp), Some(gp)) => (pp, gp),
        (pp, gp) => {
            if let Some(preset) = skills_core::lookup_preset(&name) {
                (
                    pp.unwrap_or_else(|| preset.project_path.to_string()),
                    gp.unwrap_or_else(|| preset.global_path.to_string()),
                )
            } else {
                return Err(format!(
                    "Unknown agent '{}'. Provide project_path and global_path, \
                     or use a known agent: {}",
                    name,
                    skills_core::KNOWN_AGENTS
                        .iter()
                        .map(|p| p.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    };

    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    config.agents.insert(
        name.clone(),
        AgentDef {
            project_path: pp,
            global_path: gp,
            enabled: true,
        },
    );
    config
        .save(&state.dirs.agents_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "agent_add",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Added agent '{}'", name),
        },
    )
    .await;
    Ok(format!("Added agent '{}'", name))
}

#[tauri::command]
pub async fn list_agent_presets() -> Result<Vec<serde_json::Value>, String> {
    Ok(skills_core::KNOWN_AGENTS
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "project_path": p.project_path,
                "global_path": p.global_path,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn edit_agent(
    state: State<'_, AppState>,
    name: String,
    project_path: String,
    global_path: String,
) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let existing = config
        .agents
        .get(&name)
        .ok_or_else(|| format!("Agent '{}' not found", name))?;
    let enabled = existing.enabled;
    config.agents.insert(
        name.clone(),
        AgentDef {
            project_path,
            global_path,
            enabled,
        },
    );
    config
        .save(&state.dirs.agents_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "agent_edit",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Updated agent '{}'", name),
        },
    )
    .await;
    Ok(format!("Updated agent '{}'", name))
}

#[tauri::command]
pub async fn remove_agent(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    if config.agents.remove(&name).is_none() {
        return Err(format!("Agent '{}' not found", name));
    }
    config
        .save(&state.dirs.agents_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "agent_remove",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Removed agent '{}'", name),
        },
    )
    .await;
    Ok(format!("Removed agent '{}'", name))
}

#[tauri::command]
pub async fn toggle_agent(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<String, String> {
    let mut config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let agent = config
        .agents
        .get_mut(&name)
        .ok_or_else(|| format!("Agent '{}' not found", name))?;
    agent.enabled = enabled;
    config
        .save(&state.dirs.agents_toml())
        .map_err(|e| e.to_string())?;
    let label = if enabled { "enabled" } else { "disabled" };
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "agent_toggle",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Agent '{}' {}", name, label),
        },
    )
    .await;
    Ok(format!("Agent '{}' {}", name, label))
}

// --- Status & Placements ---

#[tauri::command]
pub async fn get_status(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<StatusInfo, String> {
    let profiles_config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let s = placements::status(&state.db, &profiles_config, &project_path)
        .await
        .map_err(|e| e.to_string())?;
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
    let profiles_config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let result = placements::activate(
        &state.dirs,
        &state.db,
        &profiles_config,
        &agents_config,
        &profile_name,
        &project_path,
        force,
    )
    .await
    .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_activate",
            params: None,
            project_path: Some(&project_path),
            result: "success",
            details: &format!(
                "Activated '{}': {} placements",
                profile_name, result.total_placements
            ),
        },
    )
    .await;
    Ok(format!(
        "Activated '{}': {} skills, {} placements",
        result.profile_name, result.skills_placed, result.total_placements
    ))
}

#[tauri::command]
pub async fn deactivate_profile(
    state: State<'_, AppState>,
    profile_name: String,
    project_path: String,
) -> Result<String, String> {
    let result = placements::deactivate(&state.db, &profile_name, &project_path)
        .await
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_deactivate",
            params: None,
            project_path: Some(&project_path),
            result: "success",
            details: &format!(
                "Deactivated '{}': {} removed",
                profile_name, result.files_removed
            ),
        },
    )
    .await;
    Ok(format!(
        "Deactivated '{}': {} removed, {} kept",
        result.profile_name, result.files_removed, result.files_kept
    ))
}

#[tauri::command]
pub async fn switch_profile(
    state: State<'_, AppState>,
    new_profile: String,
    project_path: String,
    from_profile: Option<String>,
    force: bool,
) -> Result<String, String> {
    let profiles_config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let result = placements::switch_profile(
        &state.dirs,
        &state.db,
        &profiles_config,
        &agents_config,
        &new_profile,
        &project_path,
        from_profile.as_deref(),
        force,
    )
    .await
    .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "profile_switch",
            params: None,
            project_path: Some(&project_path),
            result: "success",
            details: &format!(
                "Switched to '{}': +{} -{} ~{}",
                new_profile, result.skills_added, result.skills_removed, result.skills_kept
            ),
        },
    )
    .await;
    Ok(format!(
        "Switched to '{}': +{} added, ~{} kept, -{} removed",
        result.new_profile, result.skills_added, result.skills_kept, result.skills_removed
    ))
}

// --- Global Skills ---

#[tauri::command]
pub async fn get_global_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let status = placements::global_status(&state.db, &config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "configured_skills": status.configured_skills,
        "placed_skills": status.placed_skills,
        "is_active": status.is_active,
    }))
}

#[tauri::command]
pub async fn activate_global(state: State<'_, AppState>) -> Result<String, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let result = placements::activate_global(&state.dirs, &state.db, &config, &agents_config)
        .await
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "global_activate",
            params: None,
            project_path: None,
            result: "success",
            details: &format!(
                "Activated global skills: {} placements",
                result.total_placements
            ),
        },
    )
    .await;
    Ok(format!(
        "Activated {} global skills ({} placements)",
        result.skills_placed, result.total_placements
    ))
}

#[tauri::command]
pub async fn deactivate_global(state: State<'_, AppState>) -> Result<String, String> {
    let result = placements::deactivate_global(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "global_deactivate",
            params: None,
            project_path: None,
            result: "success",
            details: &format!(
                "Deactivated global skills: {} removed",
                result.files_removed
            ),
        },
    )
    .await;
    Ok(format!(
        "Deactivated global skills: {} removed",
        result.files_removed
    ))
}

#[tauri::command]
pub async fn edit_global_skills(
    state: State<'_, AppState>,
    skills: Vec<String>,
) -> Result<String, String> {
    let mut config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    config.global.skills = skills.clone();
    config
        .save(&state.dirs.profiles_toml())
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "global_edit",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Updated global skills: {}", skills.join(", ")),
        },
    )
    .await;
    Ok(format!("Updated global skills: {}", skills.join(", ")))
}

// --- Projects ---

#[derive(Serialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub linked_profiles: Vec<String>,
    pub active_profiles: Vec<String>,
    pub placement_count: usize,
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    let projects = state
        .db
        .list_all_projects()
        .await
        .map_err(|e| e.to_string())?;
    let mut result: Vec<ProjectInfo> = Vec::new();
    for project in projects
        .into_iter()
        .filter(|p| p.path != GLOBAL_PROJECT_PATH)
    {
        let linked_profiles = state
            .db
            .get_linked_profiles(project.id)
            .await
            .unwrap_or_default();
        let active_profiles = state
            .db
            .get_active_profiles(project.id)
            .await
            .unwrap_or_default();
        let placements = state
            .db
            .get_all_placements_for_project(project.id)
            .await
            .unwrap_or_default();
        let display_name = project.name.unwrap_or_else(|| {
            std::path::Path::new(&project.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| project.path.clone())
        });
        result.push(ProjectInfo {
            path: project.path,
            name: display_name,
            linked_profiles,
            active_profiles,
            placement_count: placements.len(),
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn add_project(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> Result<String, String> {
    let display = name.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&path)
    });
    state
        .db
        .get_or_create_project(&path, Some(display))
        .await
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "project_add",
            params: None,
            project_path: Some(&path),
            result: "success",
            details: &format!("Registered project '{}'", display),
        },
    )
    .await;
    Ok(format!("Registered project '{}'", display))
}

#[tauri::command]
pub async fn remove_project(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let project_id = state
        .db
        .get_or_create_project(&path, None)
        .await
        .map_err(|e| e.to_string())?;
    state
        .db
        .delete_project(project_id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "project_remove",
            params: None,
            project_path: Some(&path),
            result: "success",
            details: &format!("Removed project tracking for '{}'", path),
        },
    )
    .await;
    Ok(format!("Removed project '{}'", path))
}

#[tauri::command]
pub async fn link_profile_to_project(
    state: State<'_, AppState>,
    project_path: String,
    profile_name: String,
) -> Result<String, String> {
    let project_id = state
        .db
        .get_or_create_project(&project_path, None)
        .await
        .map_err(|e| e.to_string())?;
    state
        .db
        .link_profile_to_project(project_id, &profile_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Linked '{}' to project", profile_name))
}

#[tauri::command]
pub async fn unlink_profile_from_project(
    state: State<'_, AppState>,
    project_path: String,
    profile_name: String,
) -> Result<String, String> {
    let project_id = state
        .db
        .get_or_create_project(&project_path, None)
        .await
        .map_err(|e| e.to_string())?;
    state
        .db
        .unlink_profile_from_project(project_id, &profile_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Unlinked '{}' from project", profile_name))
}

#[tauri::command]
pub async fn activate_project(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<String, String> {
    let project_id = state
        .db
        .get_or_create_project(&project_path, None)
        .await
        .map_err(|e| e.to_string())?;
    let linked = state
        .db
        .get_linked_profiles(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if linked.is_empty() {
        return Err("No profiles linked to this project".to_string());
    }
    let profiles_config =
        ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let mut errors: Vec<String> = Vec::new();
    let mut activated: Vec<String> = Vec::new();
    for profile_name in &linked {
        match placements::activate(
            &state.dirs,
            &state.db,
            &profiles_config,
            &agents_config,
            profile_name,
            &project_path,
            false,
        )
        .await
        {
            Ok(_) => activated.push(profile_name.clone()),
            Err(e) => errors.push(format!("{}: {}", profile_name, e)),
        }
    }
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "project_activate",
            params: None,
            project_path: Some(&project_path),
            result: if errors.is_empty() {
                "success"
            } else {
                "partial"
            },
            details: &format!("Activated {} profiles", activated.len()),
        },
    )
    .await;
    if !errors.is_empty() {
        return Err(format!("Some profiles failed: {}", errors.join("; ")));
    }
    Ok(format!("Activated {} profiles", activated.len()))
}

#[tauri::command]
pub async fn deactivate_project(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<String, String> {
    let project_id = state
        .db
        .get_or_create_project(&project_path, None)
        .await
        .map_err(|e| e.to_string())?;
    let active = state
        .db
        .get_active_profiles(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if active.is_empty() {
        return Err("No active profiles to deactivate".to_string());
    }
    let mut errors: Vec<String> = Vec::new();
    let mut deactivated: Vec<String> = Vec::new();
    for profile_name in &active {
        match placements::deactivate(&state.db, profile_name, &project_path).await {
            Ok(_) => deactivated.push(profile_name.clone()),
            Err(e) => errors.push(format!("{}: {}", profile_name, e)),
        }
    }
    let _ = logging::log(
        &state.db,
        LogEntry {
            source: Source::Gui,
            agent_name: None,
            operation: "project_deactivate",
            params: None,
            project_path: Some(&project_path),
            result: if errors.is_empty() {
                "success"
            } else {
                "partial"
            },
            details: &format!("Deactivated {} profiles", deactivated.len()),
        },
    )
    .await;
    if !errors.is_empty() {
        return Err(format!("Some profiles failed: {}", errors.join("; ")));
    }
    Ok(format!("Deactivated {} profiles", deactivated.len()))
}

// --- Settings ---

#[derive(Serialize, Deserialize)]
pub struct SettingsPayload {
    pub mcp_enabled: bool,
    pub mcp_port: u16,
    pub mcp_transport: String,
    pub git_sync_enabled: bool,
    pub git_sync_repo_url: String,
    #[serde(default)]
    pub scan_auto_on_startup: bool,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    let settings = AppSettings::load(&state.dirs.settings_toml()).map_err(|e| e.to_string())?;
    Ok(SettingsPayload {
        mcp_enabled: settings.mcp.enabled,
        mcp_port: settings.mcp.port,
        mcp_transport: settings.mcp.transport,
        git_sync_enabled: settings.git_sync.enabled,
        git_sync_repo_url: settings.git_sync.repo_url,
        scan_auto_on_startup: settings.scan.auto_scan_on_startup,
    })
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    payload: SettingsPayload,
) -> Result<String, String> {
    let mut settings = AppSettings::load(&state.dirs.settings_toml()).map_err(|e| e.to_string())?;
    settings.mcp.enabled = payload.mcp_enabled;
    settings.mcp.port = payload.mcp_port;
    settings.mcp.transport = payload.mcp_transport;
    settings.git_sync.enabled = payload.git_sync_enabled;
    settings.git_sync.repo_url = payload.git_sync_repo_url;
    settings.scan.auto_scan_on_startup = payload.scan_auto_on_startup;
    settings
        .save(&state.dirs.settings_toml())
        .map_err(|e| e.to_string())?;
    Ok("Settings saved".to_string())
}

// --- Logs ---

#[tauri::command]
pub async fn get_recent_logs(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<serde_json::Value, String> {
    let logs = state
        .db
        .get_recent_logs(limit)
        .await
        .map_err(|e| e.to_string())?;
    let entries: Vec<serde_json::Value> = logs
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "timestamp": l.timestamp,
                "source": l.source,
                "agent_name": l.agent_name,
                "operation": l.operation,
                "result": l.result,
                "details": l.details,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(entries))
}
