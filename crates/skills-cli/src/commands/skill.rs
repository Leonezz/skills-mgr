use anyhow::Result;
use skills_core::{AppDirs, Database, Registry};
use crate::SkillAction;

pub async fn run(dirs: &AppDirs, _db: &Database, action: SkillAction) -> Result<()> {
    let registry = Registry::new(dirs.clone());

    match action {
        SkillAction::List => {
            let skills = registry.list()?;
            if skills.is_empty() {
                println!("No skills in registry. Run `skills-mgr skill add <source>` to add one.");
                return Ok(());
            }
            for skill in &skills {
                let desc = skill.description.as_deref().unwrap_or("(no description)");
                let source_type = skill.source.as_ref()
                    .map(|s| format!("{:?}", s.source_type).to_lowercase())
                    .unwrap_or("unknown".into());
                println!("  {} [{}] - {}", skill.name, source_type, desc);
            }
            println!("\n{} skills total", skills.len());
        }
        SkillAction::Info { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    println!("Name: {}", skill.name);
                    println!("Description: {}", skill.description.as_deref().unwrap_or("(none)"));
                    println!("Path: {}", skill.dir_path.display());
                    println!("Files:");
                    for f in &skill.files {
                        println!("  {}", f);
                    }
                    if let Some(src) = &skill.source {
                        println!("Source: {:?}", src.source_type);
                        if let Some(url) = &src.url { println!("URL: {}", url); }
                        if let Some(hash) = &src.hash { println!("Hash: {}", hash); }
                    }
                }
                None => println!("Skill '{}' not found in registry", name),
            }
        }
        SkillAction::Create { name, description } => {
            let desc = description.as_deref().unwrap_or("TODO: add description");
            let path = registry.create(&name, desc)?;
            println!("Created skill '{}' at {}", name, path.display());
        }
        SkillAction::Remove { name } => {
            registry.remove(&name)?;
            println!("Removed skill '{}' from registry", name);
        }
        SkillAction::Files { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    for f in &skill.files {
                        println!("  {}", f);
                    }
                }
                None => println!("Skill '{}' not found", name),
            }
        }
        SkillAction::Add { source } => {
            let path = std::path::Path::new(&source);
            if path.exists() {
                let name = registry.add_from_local(path)?;
                println!("Added skill '{}' from local path", name);
            } else {
                println!("Git-based skill import not yet implemented. Use a local path for now.");
            }
        }
        SkillAction::Update { name: _, all: _ } => {
            println!("skill update: not yet implemented");
        }
        SkillAction::Open { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    open::that(&skill.dir_path)?;
                }
                None => println!("Skill '{}' not found", name),
            }
        }
    }

    Ok(())
}
