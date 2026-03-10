use anyhow::Result;
use skills_core::config::{AgentDef, AgentsConfig};
use skills_core::AppDirs;
use skills_core::Database;
use crate::AgentAction;

pub async fn run(dirs: &AppDirs, _db: &Database, action: AgentAction) -> Result<()> {
    let mut config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        AgentAction::List => {
            if config.agents.is_empty() {
                println!("No agents configured. Run `skills-mgr agent add <name>` to add one.");
            } else {
                for (name, def) in &config.agents {
                    println!("  {} -> project: {}, global: {}", name, def.project_path, def.global_path);
                }
            }
        }
        AgentAction::Add { name, project_path, global_path } => {
            config.agents.insert(name.clone(), AgentDef { project_path, global_path });
            config.save(&dirs.agents_toml())?;
            println!("Added agent '{}'", name);
        }
        AgentAction::Remove { name } => {
            if config.agents.remove(&name).is_some() {
                config.save(&dirs.agents_toml())?;
                println!("Removed agent '{}'", name);
            } else {
                println!("Agent '{}' not found", name);
            }
        }
        AgentAction::Enable { name: _, project: _ } => {
            println!("agent enable: not yet implemented");
        }
        AgentAction::Disable { name: _, project: _ } => {
            println!("agent disable: not yet implemented");
        }
    }

    Ok(())
}
