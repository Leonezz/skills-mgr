use crate::ProjectAction;
use anyhow::Result;
use skills_core::logging::{self, LogEntry, Source};
use skills_core::placements::GLOBAL_PROJECT_PATH;
use skills_core::{AppDirs, Database};

pub async fn run(dirs: &AppDirs, db: &Database, action: ProjectAction) -> Result<()> {
    match action {
        ProjectAction::List => {
            let projects = db.list_all_projects().await?;
            let projects: Vec<_> = projects
                .into_iter()
                .filter(|p| p.path != GLOBAL_PROJECT_PATH)
                .collect();

            if projects.is_empty() {
                println!("No projects registered. Run `skills-mgr project add <path>` to add one.");
                return Ok(());
            }

            for project in &projects {
                let display_name = project.name.as_deref().unwrap_or_else(|| {
                    std::path::Path::new(&project.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&project.path)
                });

                let linked = db.get_linked_profiles(project.id).await.unwrap_or_default();
                let active = db.get_active_profiles(project.id).await.unwrap_or_default();

                let status = if !active.is_empty() {
                    format!(
                        "{} linked, {} active ({})",
                        linked.len(),
                        active.len(),
                        active.join(", ")
                    )
                } else if !linked.is_empty() {
                    format!("{} linked, inactive", linked.len())
                } else {
                    "no profiles".into()
                };

                println!("  {} — {} [{}]", display_name, project.path, status);
            }
            println!("\n{} project(s)", projects.len());
        }
        ProjectAction::Add { path, name } => {
            let canonical = std::fs::canonicalize(&path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());

            let display = name.as_deref().unwrap_or_else(|| {
                std::path::Path::new(&canonical)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&canonical)
            });

            db.get_or_create_project(&canonical, Some(display)).await?;
            println!("Added project '{}' at {}", display, canonical);

            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "project_add",
                    params: None,
                    project_path: Some(&canonical),
                    result: "success",
                    details: &format!("Added project '{}'", display),
                },
            )
            .await?;
        }
        ProjectAction::Remove { path } => {
            let canonical = std::fs::canonicalize(&path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());

            let projects = db.list_all_projects().await?;
            let project = projects
                .iter()
                .find(|p| p.path == canonical)
                .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", canonical))?;

            db.delete_project(project.id).await?;
            println!("Removed project '{}'", canonical);

            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "project_remove",
                    params: None,
                    project_path: Some(&canonical),
                    result: "success",
                    details: &format!("Removed project '{}'", canonical),
                },
            )
            .await?;
        }
        ProjectAction::Link {
            profile,
            project: project_arg,
        } => {
            let project_path = resolve_project_path(project_arg)?;
            let projects = db.list_all_projects().await?;
            let project = projects
                .iter()
                .find(|p| p.path == project_path)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Project '{}' not registered. Run `project add` first.",
                        project_path
                    )
                })?;

            // Validate profile exists
            let profiles_config = skills_core::config::ProfilesConfig::load(&dirs.profiles_toml())?;
            if !profiles_config.profiles.contains_key(&profile) {
                anyhow::bail!("Profile '{}' not found", profile);
            }

            db.link_profile_to_project(project.id, &profile).await?;
            println!("Linked profile '{}' to project '{}'", profile, project_path);

            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "project_link_profile",
                    params: None,
                    project_path: Some(&project_path),
                    result: "success",
                    details: &format!("Linked profile '{}' to project", profile),
                },
            )
            .await?;
        }
        ProjectAction::Unlink {
            profile,
            project: project_arg,
        } => {
            let project_path = resolve_project_path(project_arg)?;
            let projects = db.list_all_projects().await?;
            let project = projects
                .iter()
                .find(|p| p.path == project_path)
                .ok_or_else(|| anyhow::anyhow!("Project '{}' not registered", project_path))?;

            db.unlink_profile_from_project(project.id, &profile).await?;
            println!(
                "Unlinked profile '{}' from project '{}'",
                profile, project_path
            );

            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "project_unlink_profile",
                    params: None,
                    project_path: Some(&project_path),
                    result: "success",
                    details: &format!("Unlinked profile '{}' from project", profile),
                },
            )
            .await?;
        }
    }

    Ok(())
}

fn resolve_project_path(project: Option<String>) -> Result<String> {
    match project {
        Some(p) => Ok(std::fs::canonicalize(&p)
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or(p)),
        None => Ok(std::env::current_dir()?.to_string_lossy().to_string()),
    }
}
