use crate::AgentAction;
use anyhow::{Result, bail};
use skills_core::config::{AgentDef, AgentsConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::presets;
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
            all,
        } => {
            if all {
                let mut added = Vec::new();
                let mut skipped = Vec::new();
                for preset in presets::all_presets() {
                    if config.agents.contains_key(preset.name) {
                        skipped.push(preset.name);
                        continue;
                    }
                    config.agents.insert(
                        preset.name.to_string(),
                        AgentDef {
                            project_path: preset.project_path.to_string(),
                            global_path: preset.global_path.to_string(),
                            enabled: true,
                        },
                    );
                    added.push(preset.name);
                }
                config.save(&dirs.agents_toml())?;
                if !added.is_empty() {
                    println!("Added {} agents: {}", added.len(), added.join(", "));
                }
                if !skipped.is_empty() {
                    println!(
                        "Skipped {} already configured: {}",
                        skipped.len(),
                        skipped.join(", ")
                    );
                }
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "agent_add_all",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Added {} agent presets", added.len()),
                    },
                )
                .await?;
            } else {
                let name = match name {
                    Some(n) => n,
                    None => bail!("Provide an agent name, or use --all to add all known agents"),
                };
                let (pp, gp) = resolve_agent_paths(&name, project_path, global_path)?;
                config.agents.insert(
                    name.clone(),
                    AgentDef {
                        project_path: pp,
                        global_path: gp,
                        enabled: true,
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

fn resolve_agent_paths(
    name: &str,
    project_path: Option<String>,
    global_path: Option<String>,
) -> Result<(String, String)> {
    match (project_path, global_path) {
        (Some(pp), Some(gp)) => Ok((pp, gp)),
        (pp, gp) => {
            if let Some(preset) = presets::lookup_preset(name) {
                Ok((
                    pp.unwrap_or_else(|| preset.project_path.to_string()),
                    gp.unwrap_or_else(|| preset.global_path.to_string()),
                ))
            } else {
                let known: Vec<&str> = presets::all_presets().iter().map(|p| p.name).collect();
                bail!(
                    "Unknown agent '{}'. Provide --project-path and --global-path, \
                     or use a known agent: {}",
                    name,
                    known.join(", ")
                )
            }
        }
    }
}
