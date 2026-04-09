use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::config::{AgentsConfig, AppDirs, ProfilesConfig};
use crate::db::Database;
use crate::profiles;
use crate::registry;

/// Result of an activation operation.
#[derive(Debug)]
pub struct ActivationResult {
    pub profile_name: String,
    pub skills_placed: usize,
    pub agents_used: Vec<String>,
    pub total_placements: usize,
}

/// Result of a deactivation operation.
#[derive(Debug)]
pub struct DeactivationResult {
    pub profile_name: String,
    pub files_removed: usize,
    pub files_kept: usize,
}

/// A single planned placement action for dry-run previews.
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub skill_name: String,
    pub agent_name: String,
    pub target_path: String,
    pub action: PlannedAction,
}

/// The action that would be taken for a placement.
#[derive(Debug, Clone, PartialEq)]
pub enum PlannedAction {
    /// New placement — skill will be copied
    Copy,
    /// Skill already placed by same profile — just link
    Link,
    /// Different skill at same path — force overwrite
    Overwrite,
}

/// Result of a dry-run activation — shows what would happen without executing.
#[derive(Debug)]
pub struct DryRunActivateResult {
    pub profile_name: String,
    pub skills_resolved: Vec<String>,
    pub agents_used: Vec<String>,
    pub operations: Vec<PlannedOperation>,
}

/// Result of a dry-run deactivation — shows what would be removed.
#[derive(Debug)]
pub struct DryRunDeactivateResult {
    pub profile_name: String,
    pub would_remove: Vec<PlannedOperation>,
    pub would_keep: Vec<PlannedOperation>,
}

/// Internal planned placement used during activation.
struct PlannedPlacement {
    skill_name: String,
    agent_name: String,
    target_path: String,
    action: PlannedAction,
}

/// Plan an activation without executing it. Used by both `activate` and `dry_run_activate`.
async fn plan_activation(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    profile_name: &str,
    project_path: &str,
    force: bool,
) -> Result<(
    i64,
    Vec<String>,
    Vec<(String, String)>,
    Vec<PlannedPlacement>,
)> {
    let skills = profiles::resolve_profile(profiles_config, profile_name, true)?;

    let project_name = Path::new(project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let project_id = db
        .get_or_create_project(project_path, project_name.as_deref())
        .await?;

    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .filter(|(_, def)| def.enabled)
        .map(|(name, def)| (name.clone(), def.project_path.clone()))
        .collect();

    if agents.is_empty() {
        bail!("No agents configured. Run `skills-mgr agent add <name>` first.");
    }

    let mut planned: Vec<PlannedPlacement> = Vec::new();
    for skill_name in &skills {
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }

        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            let action = if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                if existing.skill_name == *skill_name {
                    PlannedAction::Link
                } else if force {
                    PlannedAction::Overwrite
                } else {
                    bail!(
                        "Conflict: target path {} already occupied by skill '{}'. Use --force to overwrite.",
                        target_str,
                        existing.skill_name
                    );
                }
            } else {
                PlannedAction::Copy
            };

            planned.push(PlannedPlacement {
                skill_name: skill_name.clone(),
                agent_name: agent_name.clone(),
                target_path: target_str,
                action,
            });
        }
    }

    Ok((project_id, skills, agents, planned))
}

/// Preview what an activation would do without making any changes.
pub async fn dry_run_activate(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    profile_name: &str,
    project_path: &str,
    force: bool,
) -> Result<DryRunActivateResult> {
    let (_project_id, skills, agents, planned) = plan_activation(
        dirs,
        db,
        profiles_config,
        agents_config,
        profile_name,
        project_path,
        force,
    )
    .await?;

    let operations = planned
        .into_iter()
        .map(|p| PlannedOperation {
            skill_name: p.skill_name,
            agent_name: p.agent_name,
            target_path: p.target_path,
            action: p.action,
        })
        .collect();

    Ok(DryRunActivateResult {
        profile_name: profile_name.to_string(),
        skills_resolved: skills,
        agents_used: agents.into_iter().map(|(n, _)| n).collect(),
        operations,
    })
}

/// Preview what a deactivation would do without making any changes.
pub async fn dry_run_deactivate(
    db: &Database,
    profile_name: &str,
    project_path: &str,
) -> Result<DryRunDeactivateResult> {
    let project_id = db.get_or_create_project(project_path, None).await?;
    let placements = db
        .get_placements_for_project_profile(project_id, profile_name)
        .await?;

    let mut would_remove = Vec::new();
    let mut would_keep = Vec::new();

    for placement in &placements {
        let remaining = db.get_placement_profile_count(placement.id).await?;
        let op = PlannedOperation {
            skill_name: placement.skill_name.clone(),
            agent_name: placement.agent_name.clone(),
            target_path: placement.target_path.clone(),
            action: PlannedAction::Copy, // reused as a label; the action field is informational
        };
        // remaining includes current profile, so if only 1 remains it would be removed
        if remaining <= 1 {
            would_remove.push(op);
        } else {
            would_keep.push(op);
        }
    }

    Ok(DryRunDeactivateResult {
        profile_name: profile_name.to_string(),
        would_remove,
        would_keep,
    })
}

/// Activate a profile for a project.
///
/// 1. Resolve skills (profile + base)
/// 2. For each skill x agent, compute target path
/// 3. Check conflicts
/// 4. Copy skill directories atomically
/// 5. Record placements in DB
pub async fn activate(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    profile_name: &str,
    project_path: &str,
    force: bool,
) -> Result<ActivationResult> {
    let (project_id, skills, agents, planned) = plan_activation(
        dirs,
        db,
        profiles_config,
        agents_config,
        profile_name,
        project_path,
        force,
    )
    .await?;

    // Handle force-overwrite DB cleanup before copying
    for p in &planned {
        if p.action == PlannedAction::Overwrite
            && let Some(existing) = db.find_conflict(project_id, &p.target_path).await?
        {
            db.delete_placement(existing.id).await?;
            let target = PathBuf::from(&p.target_path);
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
        }
    }

    // Execute placements atomically
    let mut placed_paths: Vec<PathBuf> = Vec::new();
    for p in &planned {
        let src = dirs.registry().join(&p.skill_name);
        let dst = PathBuf::from(&p.target_path);

        // Skip if already placed (same skill deduplicated)
        if p.action == PlannedAction::Link
            && let Some(existing) = db.find_conflict(project_id, &p.target_path).await?
            && existing.skill_name == p.skill_name
        {
            let pid = existing.id;
            db.link_placement_profile(pid, profile_name).await?;
            continue;
        }

        if let Err(e) = registry::copy_dir_recursive(&src, &dst) {
            // Rollback all placed paths
            for rollback_path in &placed_paths {
                let _ = std::fs::remove_dir_all(rollback_path);
            }
            bail!(
                "Failed to copy skill '{}' to '{}': {}. All placements rolled back.",
                p.skill_name,
                p.target_path,
                e
            );
        }
        placed_paths.push(dst);
    }

    // Record in DB
    for p in &planned {
        if p.action == PlannedAction::Link {
            continue; // already linked above
        }
        let placement_id = db
            .insert_placement(project_id, &p.skill_name, &p.agent_name, &p.target_path)
            .await?;
        db.link_placement_profile(placement_id, profile_name)
            .await?;
    }

    // Record active profile and ensure it's linked
    db.link_profile_to_project(project_id, profile_name).await?;
    db.activate_project_profile(project_id, profile_name)
        .await?;

    let agents_used: Vec<String> = agents.iter().map(|(n, _)| n.clone()).collect();
    Ok(ActivationResult {
        profile_name: profile_name.to_string(),
        skills_placed: skills.len(),
        agents_used,
        total_placements: planned.len(),
    })
}

/// Deactivate a profile for a project.
///
/// 1. Find placements linked to this profile
/// 2. Unlink profile from each placement
/// 3. Remove placements (and files) that have no remaining profile links
pub async fn deactivate(
    db: &Database,
    profile_name: &str,
    project_path: &str,
) -> Result<DeactivationResult> {
    let project_id = db.get_or_create_project(project_path, None).await?;

    let placements = db
        .get_placements_for_project_profile(project_id, profile_name)
        .await?;

    let mut files_removed = 0;
    let mut files_kept = 0;

    for placement in &placements {
        db.unlink_placement_profile(placement.id, profile_name)
            .await?;
        let remaining = db.get_placement_profile_count(placement.id).await?;

        if remaining == 0 {
            // No more profiles reference this placement — remove file and DB record
            let target = Path::new(&placement.target_path);
            if target.exists() {
                std::fs::remove_dir_all(target)?;
            }
            db.delete_placement(placement.id).await?;
            files_removed += 1;
        } else {
            files_kept += 1;
        }
    }

    db.deactivate_project_profile(project_id, profile_name)
        .await?;

    Ok(DeactivationResult {
        profile_name: profile_name.to_string(),
        files_removed,
        files_kept,
    })
}

/// Refresh a profile's placements for all projects where it is active.
///
/// After a profile is edited (skills added/removed), call this to reconcile
/// the on-disk placements with the new profile definition.
/// Returns the number of projects refreshed.
pub async fn refresh_profile(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    profile_name: &str,
) -> Result<usize> {
    let projects = db.get_projects_for_profile(profile_name).await?;
    let mut refreshed = 0;

    for (project_path, _project_name) in &projects {
        // Deactivate then re-activate to reconcile placements
        deactivate(db, profile_name, project_path).await?;
        activate(
            dirs,
            db,
            profiles_config,
            agents_config,
            profile_name,
            project_path,
            true, // force: overwrite any conflicts
        )
        .await?;
        refreshed += 1;
    }

    Ok(refreshed)
}

/// Result of an atomic profile switch.
#[derive(Debug)]
pub struct SwitchResult {
    pub old_profiles: Vec<String>,
    pub new_profile: String,
    pub skills_added: usize,
    pub skills_removed: usize,
    pub skills_kept: usize,
    pub total_placements: usize,
}

/// Result of a dry-run switch.
#[derive(Debug)]
pub struct DryRunSwitchResult {
    pub old_profiles: Vec<String>,
    pub new_profile: String,
    pub to_add: Vec<String>,
    pub to_remove: Vec<String>,
    pub to_keep: Vec<String>,
    pub operations: Vec<PlannedOperation>,
}

/// Atomically switch profiles for a project using diff-based semantics.
///
/// 1. Resolve new profile fully (fail early)
/// 2. Compute diff: skills to add, remove, keep
/// 3. Copy new skills first
/// 4. Update DB links for kept skills
/// 5. Remove old-only skills
/// 6. Update profile activation state
#[allow(clippy::too_many_arguments)]
pub async fn switch_profile(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    new_profile: &str,
    project_path: &str,
    from_profile: Option<&str>,
    force: bool,
) -> Result<SwitchResult> {
    // 1. Resolve new profile fully (fail early)
    let new_skills = profiles::resolve_profile(profiles_config, new_profile, true)?;
    for skill_name in &new_skills {
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }
    }

    let project_name = Path::new(project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let project_id = db
        .get_or_create_project(project_path, project_name.as_deref())
        .await?;

    // 2. Get old profiles and their skills
    let active = db.get_active_profiles(project_id).await?;
    let old_profiles: Vec<String> = if let Some(from) = from_profile {
        if !active.contains(&from.to_string()) {
            bail!("Profile '{}' is not active for this project", from);
        }
        vec![from.to_string()]
    } else {
        active.clone()
    };

    if old_profiles.is_empty() && active.contains(&new_profile.to_string()) {
        bail!("Profile '{}' is already active", new_profile);
    }

    let mut old_skills: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut unresolvable_profiles: Vec<String> = Vec::new();
    for p in &old_profiles {
        match profiles::resolve_profile(profiles_config, p, true) {
            Ok(skills) => old_skills.extend(skills),
            Err(_) => unresolvable_profiles.push(p.clone()),
        }
    }
    if !unresolvable_profiles.is_empty() {
        tracing::warn!(
            profiles = ?unresolvable_profiles,
            "Old profile(s) no longer in config — their skills cannot be resolved for cleanup. \
             Orphaned skill placements may remain on disk."
        );
    }

    let new_skills_set: std::collections::BTreeSet<String> = new_skills.iter().cloned().collect();
    let to_keep: Vec<String> = old_skills.intersection(&new_skills_set).cloned().collect();
    let to_remove: Vec<String> = old_skills.difference(&new_skills_set).cloned().collect();
    let to_add: Vec<String> = new_skills_set.difference(&old_skills).cloned().collect();

    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .filter(|(_, def)| def.enabled)
        .map(|(name, def)| (name.clone(), def.project_path.clone()))
        .collect();

    if agents.is_empty() {
        bail!("No agents configured. Run `skills-mgr agent add <name>` first.");
    }

    // 3. Copy new skills first (project never has fewer skills than needed)
    let mut placed_paths: Vec<PathBuf> = Vec::new();
    for skill_name in &to_add {
        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();
            let src = dirs.registry().join(skill_name);

            // Handle conflicts
            if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                if existing.skill_name == *skill_name {
                    // Same skill already there, just link
                    db.link_placement_profile(existing.id, new_profile).await?;
                    continue;
                } else if force {
                    db.delete_placement(existing.id).await?;
                    if target.exists() {
                        std::fs::remove_dir_all(&target)?;
                    }
                } else {
                    bail!(
                        "Conflict: {} already occupied by '{}'. Use --force to overwrite.",
                        target_str,
                        existing.skill_name
                    );
                }
            }

            if let Err(e) = registry::copy_dir_recursive(&src, &target) {
                for rollback_path in &placed_paths {
                    let _ = std::fs::remove_dir_all(rollback_path);
                }
                bail!(
                    "Failed to copy skill '{}' to '{}': {}. Switch rolled back.",
                    skill_name,
                    target_str,
                    e
                );
            }
            placed_paths.push(target.clone());

            let placement_id = db
                .insert_placement(project_id, skill_name, agent_name, &target_str)
                .await?;
            db.link_placement_profile(placement_id, new_profile).await?;
        }
    }

    // 4. Update kept skills — relink from old profiles to new
    for skill_name in &to_keep {
        for (_agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                // Link new profile
                db.link_placement_profile(existing.id, new_profile).await?;
                // Unlink old profiles
                for old_p in &old_profiles {
                    db.unlink_placement_profile(existing.id, old_p).await?;
                }
            }
        }
    }

    // 5. Remove old-only skills
    let mut skills_removed_count = 0;
    for skill_name in &to_remove {
        for (_agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                // Unlink old profiles
                for old_p in &old_profiles {
                    db.unlink_placement_profile(existing.id, old_p).await?;
                }
                let remaining = db.get_placement_profile_count(existing.id).await?;
                if remaining == 0 {
                    if target.exists() {
                        std::fs::remove_dir_all(&target)?;
                    }
                    db.delete_placement(existing.id).await?;
                    skills_removed_count += 1;
                }
            }
        }
    }

    // 6. Update profile activation state and ensure new profile is linked
    for old_p in &old_profiles {
        db.deactivate_project_profile(project_id, old_p).await?;
    }
    db.link_profile_to_project(project_id, new_profile).await?;
    db.activate_project_profile(project_id, new_profile).await?;

    let total_placements = (to_add.len() + to_keep.len()) * agents.len();
    Ok(SwitchResult {
        old_profiles,
        new_profile: new_profile.to_string(),
        skills_added: to_add.len(),
        skills_removed: skills_removed_count,
        skills_kept: to_keep.len(),
        total_placements,
    })
}

/// Preview what a profile switch would do without making any changes.
pub async fn dry_run_switch(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    new_profile: &str,
    project_path: &str,
    from_profile: Option<&str>,
) -> Result<DryRunSwitchResult> {
    let new_skills = profiles::resolve_profile(profiles_config, new_profile, true)?;
    for skill_name in &new_skills {
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }
    }

    let project_id = db.get_or_create_project(project_path, None).await?;
    let active = db.get_active_profiles(project_id).await?;
    let old_profiles: Vec<String> = if let Some(from) = from_profile {
        vec![from.to_string()]
    } else {
        active.clone()
    };

    let mut old_skills: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut unresolvable_profiles: Vec<String> = Vec::new();
    for p in &old_profiles {
        match profiles::resolve_profile(profiles_config, p, true) {
            Ok(skills) => old_skills.extend(skills),
            Err(_) => unresolvable_profiles.push(p.clone()),
        }
    }
    if !unresolvable_profiles.is_empty() {
        tracing::warn!(
            profiles = ?unresolvable_profiles,
            "Old profile(s) no longer in config — their skills cannot be resolved for cleanup. \
             Orphaned skill placements may remain on disk."
        );
    }

    let new_skills_set: std::collections::BTreeSet<String> = new_skills.iter().cloned().collect();
    let to_keep: Vec<String> = old_skills.intersection(&new_skills_set).cloned().collect();
    let to_remove: Vec<String> = old_skills.difference(&new_skills_set).cloned().collect();
    let to_add: Vec<String> = new_skills_set.difference(&old_skills).cloned().collect();

    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .filter(|(_, def)| def.enabled)
        .map(|(name, def)| (name.clone(), def.project_path.clone()))
        .collect();

    let mut operations = Vec::new();
    for skill_name in &to_add {
        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            operations.push(PlannedOperation {
                skill_name: skill_name.clone(),
                agent_name: agent_name.clone(),
                target_path: target.to_string_lossy().to_string(),
                action: PlannedAction::Copy,
            });
        }
    }
    for skill_name in &to_keep {
        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            operations.push(PlannedOperation {
                skill_name: skill_name.clone(),
                agent_name: agent_name.clone(),
                target_path: target.to_string_lossy().to_string(),
                action: PlannedAction::Link,
            });
        }
    }

    Ok(DryRunSwitchResult {
        old_profiles,
        new_profile: new_profile.to_string(),
        to_add,
        to_remove,
        to_keep,
        operations,
    })
}

/// Re-copy a skill to all active placements after a registry update.
///
/// Queries all projects for placements of this skill and re-copies from
/// the registry to each target path.
pub async fn replace_skill(dirs: &AppDirs, db: &Database, skill_name: &str) -> Result<usize> {
    let projects = db.list_all_projects().await?;
    let mut replaced = 0;

    for project in &projects {
        let placements = db.get_all_placements_for_project(project.id).await?;
        for placement in &placements {
            if placement.skill_name == skill_name {
                let src = dirs.registry().join(skill_name);
                let dst = std::path::PathBuf::from(&placement.target_path);
                if dst.exists() {
                    std::fs::remove_dir_all(&dst)?;
                }
                registry::copy_dir_recursive(&src, &dst)?;
                replaced += 1;
            }
        }
    }

    Ok(replaced)
}

/// Get status for a project — active profiles and placement count.
pub async fn status(
    db: &Database,
    profiles_config: &ProfilesConfig,
    project_path: &str,
) -> Result<ProjectStatus> {
    let project_id = db.get_or_create_project(project_path, None).await?;
    let active_profiles = db.get_active_profiles(project_id).await?;
    let placements = db.get_all_placements_for_project(project_id).await?;

    Ok(ProjectStatus {
        project_path: project_path.to_string(),
        base_skills: profiles_config.base.skills.clone(),
        active_profiles,
        placement_count: placements.len(),
    })
}

#[derive(Debug)]
pub struct ProjectStatus {
    pub project_path: String,
    pub base_skills: Vec<String>,
    pub active_profiles: Vec<String>,
    pub placement_count: usize,
}

// --- Global Skills ---

pub const GLOBAL_PROJECT_PATH: &str = "__global__";
const GLOBAL_PROFILE_NAME: &str = "__global__";

#[derive(Debug)]
pub struct GlobalActivationResult {
    pub skills_placed: usize,
    pub agents_used: Vec<String>,
    pub total_placements: usize,
}

#[derive(Debug)]
pub struct GlobalDeactivationResult {
    pub files_removed: usize,
}

#[derive(Debug)]
pub struct GlobalStatus {
    pub configured_skills: Vec<String>,
    pub placed_skills: Vec<String>,
    pub is_active: bool,
}

pub fn expand_tilde(path: &str) -> String {
    if let (Some(rest), Some(home)) = (path.strip_prefix("~/"), dirs::home_dir()) {
        return format!("{}/{}", home.display(), rest);
    }
    path.to_string()
}

/// Activate global skills — place them into each agent's global_path.
pub async fn activate_global(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
) -> Result<GlobalActivationResult> {
    let global_skills = &profiles_config.global.skills;
    if global_skills.is_empty() {
        bail!("No global skills configured. Add skills to [global] in profiles.toml.");
    }

    let project_id = db
        .get_or_create_project(GLOBAL_PROJECT_PATH, Some("Global Skills"))
        .await?;

    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .filter(|(_, def)| def.enabled)
        .map(|(name, def)| (name.clone(), expand_tilde(&def.global_path)))
        .collect();

    if agents.is_empty() {
        bail!("No agents configured.");
    }

    let mut total_placements = 0;
    for skill_name in global_skills {
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }

        for (agent_name, global_path) in &agents {
            let target = PathBuf::from(global_path).join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            let src = dirs.registry().join(skill_name);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            registry::copy_dir_recursive(&src, &target)?;

            let placement_id = db
                .insert_placement(project_id, skill_name, agent_name, &target_str)
                .await?;
            db.link_placement_profile(placement_id, GLOBAL_PROFILE_NAME)
                .await?;
            total_placements += 1;
        }
    }

    db.activate_project_profile(project_id, GLOBAL_PROFILE_NAME)
        .await?;

    Ok(GlobalActivationResult {
        skills_placed: global_skills.len(),
        agents_used: agents.iter().map(|(n, _)| n.clone()).collect(),
        total_placements,
    })
}

/// Deactivate global skills — remove placements from agent global_paths.
pub async fn deactivate_global(db: &Database) -> Result<GlobalDeactivationResult> {
    let project_id = db
        .get_or_create_project(GLOBAL_PROJECT_PATH, Some("Global Skills"))
        .await?;

    let placements = db
        .get_placements_for_project_profile(project_id, GLOBAL_PROFILE_NAME)
        .await?;

    let mut files_removed = 0;
    for placement in &placements {
        db.unlink_placement_profile(placement.id, GLOBAL_PROFILE_NAME)
            .await?;
        let remaining = db.get_placement_profile_count(placement.id).await?;

        if remaining == 0 {
            let target = Path::new(&placement.target_path);
            if target.exists() {
                std::fs::remove_dir_all(target)?;
            }
            db.delete_placement(placement.id).await?;
            files_removed += 1;
        }
    }

    db.deactivate_project_profile(project_id, GLOBAL_PROFILE_NAME)
        .await?;

    Ok(GlobalDeactivationResult { files_removed })
}

/// Get current global skills status.
pub async fn global_status(
    db: &Database,
    profiles_config: &ProfilesConfig,
) -> Result<GlobalStatus> {
    let project_id = db
        .get_or_create_project(GLOBAL_PROJECT_PATH, Some("Global Skills"))
        .await?;
    let active = db.get_active_profiles(project_id).await?;
    let placements = db.get_all_placements_for_project(project_id).await?;

    let placed_skills: Vec<String> = placements
        .iter()
        .map(|p| p.skill_name.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(GlobalStatus {
        configured_skills: profiles_config.global.skills.clone(),
        placed_skills,
        is_active: active.contains(&GLOBAL_PROFILE_NAME.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, AppDirs, Database, ProfilesConfig, AgentsConfig) {
        let tmp = TempDir::new().unwrap();
        let dirs = AppDirs::new(tmp.path().join("skills-mgr"));
        dirs.ensure_dirs().unwrap();

        let db = Database::open_memory().await.unwrap();

        // Create test skills in registry
        for name in &[
            "code-review",
            "rust-engineer",
            "react-specialist",
            "api-design",
        ] {
            let skill_dir = dirs.registry().join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {}\ndescription: Test skill\n---\nContent", name),
            )
            .unwrap();
        }

        let profiles_config = ProfilesConfig {
            global: GlobalConfig::default(),
            base: BaseConfig {
                skills: vec!["code-review".into()],
            },
            profiles: {
                let mut m = BTreeMap::new();
                m.insert(
                    "rust".into(),
                    ProfileDef {
                        description: Some("Rust".into()),
                        skills: vec!["rust-engineer".into()],
                        includes: vec![],
                    },
                );
                m.insert(
                    "react".into(),
                    ProfileDef {
                        description: Some("React".into()),
                        skills: vec!["react-specialist".into()],
                        includes: vec![],
                    },
                );
                m
            },
        };

        let agents_config = AgentsConfig {
            agents: {
                let mut m = BTreeMap::new();
                m.insert(
                    "test-agent".into(),
                    AgentDef {
                        project_path: ".test-agent/skills".into(),
                        global_path: "~/.test-agent/skills".into(),
                        enabled: true,
                    },
                );
                m
            },
        };

        (tmp, dirs, db, profiles_config, agents_config)
    }

    #[tokio::test]
    async fn test_activate_profile() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        let result = activate(
            &dirs,
            &db,
            &profiles,
            &agents,
            "rust",
            &project_path.to_string_lossy(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(result.profile_name, "rust");
        assert_eq!(result.skills_placed, 2); // code-review (base) + rust-engineer

        // Verify files were placed
        assert!(
            project_path
                .join(".test-agent/skills/rust-engineer/SKILL.md")
                .exists()
        );
        assert!(
            project_path
                .join(".test-agent/skills/code-review/SKILL.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn test_deactivate_profile() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        activate(
            &dirs,
            &db,
            &profiles,
            &agents,
            "rust",
            &project_path.to_string_lossy(),
            false,
        )
        .await
        .unwrap();
        let result = deactivate(&db, "rust", &project_path.to_string_lossy())
            .await
            .unwrap();

        assert!(result.files_removed > 0);
        // Verify files were removed
        assert!(
            !project_path
                .join(".test-agent/skills/rust-engineer/SKILL.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn test_composable_deactivation_keeps_shared() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        // Both rust and react profiles share base "code-review"
        activate(
            &dirs,
            &db,
            &profiles,
            &agents,
            "rust",
            &project_path.to_string_lossy(),
            false,
        )
        .await
        .unwrap();
        activate(
            &dirs,
            &db,
            &profiles,
            &agents,
            "react",
            &project_path.to_string_lossy(),
            false,
        )
        .await
        .unwrap();

        // Deactivate rust — code-review should remain (still used by react)
        let result = deactivate(&db, "rust", &project_path.to_string_lossy())
            .await
            .unwrap();
        assert!(result.files_kept > 0); // code-review kept
        assert!(
            project_path
                .join(".test-agent/skills/code-review/SKILL.md")
                .exists()
        );
        assert!(
            !project_path
                .join(".test-agent/skills/rust-engineer/SKILL.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn test_status() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        activate(
            &dirs,
            &db,
            &profiles,
            &agents,
            "rust",
            &project_path.to_string_lossy(),
            false,
        )
        .await
        .unwrap();

        let s = status(&db, &profiles, &project_path.to_string_lossy())
            .await
            .unwrap();
        assert_eq!(s.active_profiles, vec!["rust"]);
        assert!(s.placement_count > 0);
    }
}
