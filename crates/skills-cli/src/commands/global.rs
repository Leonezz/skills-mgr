use crate::GlobalAction;
use anyhow::Result;
use skills_core::config::{AgentsConfig, ProfilesConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::placements;
use skills_core::{AppDirs, Database};

pub async fn run(dirs: &AppDirs, db: &Database, action: GlobalAction) -> Result<()> {
    let mut profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        GlobalAction::Status => {
            let status = placements::global_status(db, &profiles_config).await?;
            println!("Global Skills Configuration:");
            if status.configured_skills.is_empty() {
                println!("  No skills configured in [global]");
            } else {
                println!("  Configured: {}", status.configured_skills.join(", "));
            }
            println!(
                "  Status: {}",
                if status.is_active {
                    "Active"
                } else {
                    "Inactive"
                }
            );
            if !status.placed_skills.is_empty() {
                println!("  Placed: {}", status.placed_skills.join(", "));
            }
        }
        GlobalAction::Activate => {
            let result =
                placements::activate_global(dirs, db, &profiles_config, &agents_config).await?;
            println!(
                "Global skills activated: {} skills -> {} ({} placements)",
                result.skills_placed,
                result.agents_used.join(", "),
                result.total_placements
            );
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
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
            .await?;
        }
        GlobalAction::Deactivate => {
            let result = placements::deactivate_global(db).await?;
            println!(
                "Global skills deactivated: {} removed",
                result.files_removed
            );
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
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
            .await?;
        }
        GlobalAction::Add { skills } => {
            for skill in &skills {
                if !profiles_config.global.skills.contains(skill) {
                    profiles_config.global.skills.push(skill.clone());
                }
            }
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Added to global skills: {}", skills.join(", "));
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "global_add",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Added global skills: {}", skills.join(", ")),
                },
            )
            .await?;
        }
        GlobalAction::Remove { skills } => {
            profiles_config
                .global
                .skills
                .retain(|s| !skills.contains(s));
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Removed from global skills: {}", skills.join(", "));
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "global_remove",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Removed global skills: {}", skills.join(", ")),
                },
            )
            .await?;
        }
    }

    Ok(())
}
