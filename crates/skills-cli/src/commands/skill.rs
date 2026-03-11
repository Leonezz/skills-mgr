use crate::SkillAction;
use anyhow::Result;
use skills_core::config::SourcesConfig;
use skills_core::logging::{self, LogEntry, Source};
use skills_core::registry::compute_tree_hash;
use skills_core::{AppDirs, Database, Registry};

pub async fn run(dirs: &AppDirs, db: &Database, action: SkillAction) -> Result<()> {
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
                let source_type = skill
                    .source
                    .as_ref()
                    .map(|s| format!("{:?}", s.source_type).to_lowercase())
                    .unwrap_or("unknown".into());
                println!("  {} [{}] - {}", skill.name, source_type, desc);
            }
            println!("\n{} skills total", skills.len());
        }
        SkillAction::Info { name } => match registry.get(&name)? {
            Some(skill) => {
                println!("Name: {}", skill.name);
                println!(
                    "Description: {}",
                    skill.description.as_deref().unwrap_or("(none)")
                );
                println!("Path: {}", skill.dir_path.display());
                println!("Files:");
                for f in &skill.files {
                    println!("  {}", f);
                }
                if let Some(src) = &skill.source {
                    println!("Source: {:?}", src.source_type);
                    if let Some(url) = &src.url {
                        println!("URL: {}", url);
                    }
                    if let Some(hash) = &src.hash {
                        println!("Hash: {}", hash);
                    }
                }
            }
            None => println!("Skill '{}' not found in registry", name),
        },
        SkillAction::Create { name, description } => {
            let desc = description.as_deref().unwrap_or("TODO: add description");
            let path = registry.create(&name, desc)?;
            println!("Created skill '{}' at {}", name, path.display());
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "skill_create",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Created skill '{}'", name),
                },
            )
            .await?;
        }
        SkillAction::Remove { name } => {
            registry.remove(&name)?;
            println!("Removed skill '{}' from registry", name);
            logging::log(
                db,
                LogEntry {
                    source: Source::Cli,
                    agent_name: None,
                    operation: "skill_remove",
                    params: None,
                    project_path: None,
                    result: "success",
                    details: &format!("Removed skill '{}'", name),
                },
            )
            .await?;
        }
        SkillAction::Files { name } => match registry.get(&name)? {
            Some(skill) => {
                for f in &skill.files {
                    println!("  {}", f);
                }
            }
            None => println!("Skill '{}' not found", name),
        },
        SkillAction::Add { source } => {
            let path = std::path::Path::new(&source);
            if path.exists() {
                let name = registry.add_from_local(path)?;
                println!("Added skill '{}' from local path", name);
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "skill_add",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Added skill '{}' from local", name),
                    },
                )
                .await?;
            } else if skills_core::remote::is_remote_source(&source) {
                println!("Downloading from {}...", source);
                let name = registry.add_from_remote(&source).await?;
                println!("Added skill '{}' from remote", name);
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "skill_import",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Imported skill '{}' from {}", name, source),
                    },
                )
                .await?;
            } else {
                println!("Source '{}' is not a local path or recognized remote URL.", source);
                println!("Supported formats:");
                println!("  Local:  /path/to/skill-dir");
                println!("  GitHub: https://github.com/owner/repo/tree/main/path");
                println!("  Short:  owner/repo/path/to/skill");
            }
        }
        SkillAction::Update { name, all } => {
            let mut sources = SourcesConfig::load(&dirs.sources_toml())?;
            let skills_to_update: Vec<String> = if all {
                registry.list()?.into_iter().map(|s| s.name).collect()
            } else if let Some(n) = name {
                vec![n]
            } else {
                println!("Specify a skill name or --all");
                return Ok(());
            };

            let mut updated = 0;
            for skill_name in &skills_to_update {
                let skill_dir = dirs.registry().join(skill_name);
                if !skill_dir.exists() {
                    println!("  {} — not found, skipping", skill_name);
                    continue;
                }
                let new_hash = compute_tree_hash(&skill_dir)?;
                let old_hash = sources.skills.get(skill_name).and_then(|s| s.hash.as_ref());
                if old_hash == Some(&new_hash) {
                    println!("  {} — up to date", skill_name);
                } else {
                    if let Some(entry) = sources.skills.get_mut(skill_name) {
                        entry.hash = Some(new_hash);
                        entry.updated_at = Some(
                            chrono::Utc::now()
                                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                                .to_string(),
                        );
                    }
                    println!("  {} — hash updated", skill_name);
                    updated += 1;
                }
            }
            sources.save(&dirs.sources_toml())?;
            println!("\n{} skills updated", updated);
            if updated > 0 {
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "skill_update",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Updated {} skills", updated),
                    },
                )
                .await?;
            }
        }
        SkillAction::Open { name } => match registry.get(&name)? {
            Some(skill) => {
                open::that(&skill.dir_path)?;
            }
            None => println!("Skill '{}' not found", name),
        },
    }

    Ok(())
}
