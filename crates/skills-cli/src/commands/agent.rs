use crate::AgentAction;
use anyhow::Result;
use skills_core::config::{AgentDef, AgentsConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::{AppDirs, Database};

pub async fn run(dirs: &AppDirs, db: &Database, action: AgentAction) -> Result<()> {
    let mut config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        AgentAction::List => {
            if config.agents.is_empty() {
                println!("No agents configured. Run `skills-mgr agent add <name>` to add one.");
            } else {
                for (name, def) in &config.agents {
                    println!(
                        "  {} -> project: {}, global: {}",
                        name, def.project_path, def.global_path
                    );
                }
            }
        }
        AgentAction::Add {
            name,
            project_path,
            global_path,
        } => {
            config.agents.insert(
                name.clone(),
                AgentDef {
                    project_path,
                    global_path,
                },
            );
            config.save(&dirs.agents_toml())?;
            println!("Added agent '{}'", name);
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "agent_add",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Added agent '{}'", name),
                },
            )
            .await?;
        }
        AgentAction::Remove { name } => {
            if config.agents.remove(&name).is_some() {
                config.save(&dirs.agents_toml())?;
                println!("Removed agent '{}'", name);
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "agent_remove",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Removed agent '{}'", name),
                    },
                )
                .await?;
            } else {
                println!("Agent '{}' not found", name);
            }
        }
        AgentAction::Enable { name, project } => {
            let project_path = resolve_project_path(project)?;
            let project_id = db.get_or_create_project(&project_path, None).await?;
            db.set_agent_enabled(project_id, &name, true).await?;
            println!("Enabled agent '{}' for {}", name, project_path);
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: Some(&name),
                    operation: "agent_enable",
                    params: None,
                    project_path: Some(&project_path),
                    result: "success",
                    details: &format!("Enabled agent '{}'", name),
                },
            )
            .await?;
        }
        AgentAction::Disable { name, project } => {
            let project_path = resolve_project_path(project)?;
            let project_id = db.get_or_create_project(&project_path, None).await?;
            db.set_agent_enabled(project_id, &name, false).await?;
            println!("Disabled agent '{}' for {}", name, project_path);
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: Some(&name),
                    operation: "agent_disable",
                    params: None,
                    project_path: Some(&project_path),
                    result: "success",
                    details: &format!("Disabled agent '{}'", name),
                },
            )
            .await?;
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
