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

    // Record active profile
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
