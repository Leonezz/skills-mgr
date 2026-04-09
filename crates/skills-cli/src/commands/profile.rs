use crate::ProfileAction;
use anyhow::Result;
use skills_core::config::{AgentsConfig, ProfileDef, ProfilesConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::placements::{self, PlannedAction};
use skills_core::profiles;
use skills_core::{AppDirs, Database};

pub async fn run(dirs: &AppDirs, db: &Database, action: ProfileAction) -> Result<()> {
    let mut profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        ProfileAction::List => {
            println!("Base skills: {}", profiles_config.base.skills.join(", "));
            if profiles_config.profiles.is_empty() {
                println!("\nNo profiles defined.");
            } else {
                println!("\nProfiles:");
                for (name, profile) in &profiles_config.profiles {
                    let desc = profile.description.as_deref().unwrap_or("");
                    let skills = profile.skills.join(", ");
                    let includes = if profile.includes.is_empty() {
                        String::new()
                    } else {
                        format!(" (includes: {})", profile.includes.join(", "))
                    };
                    println!("  {} - {} [{}]{}", name, desc, skills, includes);
                }
            }
        }
        ProfileAction::Show { name } => {
            let resolved = profiles::resolve_profile(&profiles_config, &name, true)?;
            println!("Profile '{}' resolves to {} skills:", name, resolved.len());
            for skill in &resolved {
                println!("  {}", skill);
            }
        }
        ProfileAction::Create {
            name,
            description,
            add,
            include,
        } => {
            let profile = ProfileDef {
                description,
                skills: add,
                includes: include,
            };
            profiles_config.profiles.insert(name.clone(), profile);
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Created profile '{}'", name);
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "profile_create",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Created profile '{}'", name),
                },
            )
            .await?;
        }
        ProfileAction::Delete { name } => {
            if profiles_config.profiles.remove(&name).is_some() {
                profiles_config.save(&dirs.profiles_toml())?;
                println!("Deleted profile '{}'", name);
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "profile_delete",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Deleted profile '{}'", name),
                    },
                )
                .await?;
            } else {
                println!("Profile '{}' not found", name);
            }
        }
        ProfileAction::Edit {
            name,
            add,
            remove,
            include,
        } => {
            let profile = profiles_config
                .profiles
                .get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
            for s in &add {
                if !profile.skills.contains(s) {
                    profile.skills.push(s.clone());
                }
            }
            profile.skills.retain(|s| !remove.contains(s));
            for i in &include {
                if !profile.includes.contains(i) {
                    profile.includes.push(i.clone());
                }
            }
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;

            // Refresh placements for all projects where this profile is active
            let refreshed =
                placements::refresh_profile(dirs, db, &profiles_config, &agents_config, &name)
                    .await?;

            println!("Updated profile '{}'", name);
            if refreshed > 0 {
                println!("  Refreshed placements in {} project(s)", refreshed);
            }
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "profile_edit",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!(
                        "Updated profile '{}', {} projects refreshed",
                        name, refreshed
                    ),
                },
            )
            .await?;
        }
        ProfileAction::Activate {
            name,
            project,
            global: _,
            force,
            dry_run,
        } => {
            let project_path = resolve_project_path(project)?;
            if dry_run {
                let result = placements::dry_run_activate(
                    dirs,
                    db,
                    &profiles_config,
                    &agents_config,
                    &name,
                    &project_path,
                    force,
                )
                .await?;
                println!(
                    "Dry run: activate '{}' for {}",
                    result.profile_name, project_path
                );
                println!(
                    "  Skills: {} | Agents: {}",
                    result.skills_resolved.join(", "),
                    result.agents_used.join(", ")
                );
                print_planned_operations(&result.operations);
            } else {
                let result = placements::activate(
                    dirs,
                    db,
                    &profiles_config,
                    &agents_config,
                    &name,
                    &project_path,
                    force,
                )
                .await?;
                println!(
                    "Activated profile '{}' for {}",
                    result.profile_name, project_path
                );
                println!(
                    "  {} skills -> {} ({} placements)",
                    result.skills_placed,
                    result.agents_used.join(", "),
                    result.total_placements
                );
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "profile_activate",
                        params: None,
                        project_path: Some(&project_path),
                        result: "success",
                        details: &format!(
                            "Activated '{}': {} placements",
                            name, result.total_placements
                        ),
                    },
                )
                .await?;
            }
        }
        ProfileAction::Deactivate {
            name,
            project,
            global: _,
            dry_run,
        } => {
            let project_path = resolve_project_path(project)?;
            if dry_run {
                let result = placements::dry_run_deactivate(db, &name, &project_path).await?;
                println!(
                    "Dry run: deactivate '{}' for {}",
                    result.profile_name, project_path
                );
                if !result.would_remove.is_empty() {
                    println!("  Would remove:");
                    for op in &result.would_remove {
                        println!(
                            "    {} ({}) -> {}",
                            op.skill_name, op.agent_name, op.target_path
                        );
                    }
                }
                if !result.would_keep.is_empty() {
                    println!("  Would keep (shared with other profiles):");
                    for op in &result.would_keep {
                        println!(
                            "    {} ({}) -> {}",
                            op.skill_name, op.agent_name, op.target_path
                        );
                    }
                }
            } else {
                let result = placements::deactivate(db, &name, &project_path).await?;
                println!(
                    "Deactivated profile '{}': {} removed, {} kept",
                    result.profile_name, result.files_removed, result.files_kept
                );
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "profile_deactivate",
                        params: None,
                        project_path: Some(&project_path),
                        result: "success",
                        details: &format!(
                            "Deactivated '{}': {} removed",
                            name, result.files_removed
                        ),
                    },
                )
                .await?;
            }
        }
        ProfileAction::Switch {
            name,
            project,
            from,
            force,
            dry_run,
        } => {
            let project_path = resolve_project_path(project)?;
            if dry_run {
                let result = placements::dry_run_switch(
                    dirs,
                    db,
                    &profiles_config,
                    &agents_config,
                    &name,
                    &project_path,
                    from.as_deref(),
                )
                .await?;
                println!("Dry run: switch to '{}' for {}", name, project_path);
                if !result.old_profiles.is_empty() {
                    println!("  From: {}", result.old_profiles.join(", "));
                }
                if !result.to_add.is_empty() {
                    println!("  Add: {}", result.to_add.join(", "));
                }
                if !result.to_keep.is_empty() {
                    println!("  Keep: {}", result.to_keep.join(", "));
                }
                if !result.to_remove.is_empty() {
                    println!("  Remove: {}", result.to_remove.join(", "));
                }
                print_planned_operations(&result.operations);
            } else {
                let result = placements::switch_profile(
                    dirs,
                    db,
                    &profiles_config,
                    &agents_config,
                    &name,
                    &project_path,
                    from.as_deref(),
                    force,
                )
                .await?;
                println!(
                    "Switched to '{}': +{} added, ~{} kept, -{} removed ({} placements)",
                    result.new_profile,
                    result.skills_added,
                    result.skills_kept,
                    result.skills_removed,
                    result.total_placements
                );
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "profile_switch",
                        params: None,
                        project_path: Some(&project_path),
                        result: "success",
                        details: &format!("Switched to '{}'", name),
                    },
                )
                .await?;
            }
        }
    }

    Ok(())
}

fn resolve_project_path(project: Option<String>) -> Result<String> {
    match project {
        Some(p) => Ok(std::fs::canonicalize(&p)?.to_string_lossy().to_string()),
        None => Ok(std::env::current_dir()?.to_string_lossy().to_string()),
    }
}

fn print_planned_operations(operations: &[placements::PlannedOperation]) {
    for op in operations {
        let label = match &op.action {
            PlannedAction::Copy => "COPY",
            PlannedAction::Link => "LINK",
            PlannedAction::Overwrite => "OVERWRITE",
        };
        println!(
            "    [{}] {} ({}) -> {}",
            label, op.skill_name, op.agent_name, op.target_path
        );
    }
}
