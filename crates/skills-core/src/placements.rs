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
    // Resolve skills
    let skills = profiles::resolve_profile(profiles_config, profile_name, true)?;

    // Get or create project
    let project_name = Path::new(project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let project_id = db
        .get_or_create_project(project_path, project_name.as_deref())
        .await?;

    // Determine which agents to place into (skip disabled)
    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .filter(|(_, def)| def.enabled)
        .map(|(name, def)| (name.clone(), def.project_path.clone()))
        .collect();

    if agents.is_empty() {
        bail!("No agents configured. Run `skills-mgr agent add <name>` first.");
    }

    // Compute all placements and check conflicts
    struct PlannedPlacement {
        skill_name: String,
        agent_name: String,
        target_path: String,
    }

    let mut planned: Vec<PlannedPlacement> = Vec::new();
    for skill_name in &skills {
        // Check skill exists in registry
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }

        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            // Check for conflicts
            if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                if existing.skill_name == *skill_name {
                    // Same skill — just add profile link, no conflict
                } else if force {
                    // Different skill at same path — force overwrite
                    db.delete_placement(existing.id).await?;
                    if target.exists() {
                        std::fs::remove_dir_all(&target)?;
                    }
                } else {
                    bail!(
                        "Conflict: target path {} already occupied by skill '{}'. Use --force to overwrite.",
                        target_str,
                        existing.skill_name
                    );
                }
            }

            planned.push(PlannedPlacement {
                skill_name: skill_name.clone(),
                agent_name: agent_name.clone(),
                target_path: target_str,
            });
        }
    }

    // Execute placements atomically
    let mut placed_paths: Vec<PathBuf> = Vec::new();
    for p in &planned {
        let src = dirs.registry().join(&p.skill_name);
        let dst = PathBuf::from(&p.target_path);

        // Skip if already placed (same skill deduplicated)
        if dst.exists() {
            // Check if it's the same skill by seeing if we already placed it
            if let Some(existing) = db.find_conflict(project_id, &p.target_path).await?
                && existing.skill_name == p.skill_name
            {
                // Just link the profile, don't re-copy
                let pid = existing.id;
                db.link_placement_profile(pid, profile_name).await?;
                continue;
            }
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
