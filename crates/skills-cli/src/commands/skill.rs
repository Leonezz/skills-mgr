use crate::SkillAction;
use anyhow::Result;
use skills_core::logging::{self, LogEntry, Source};
use skills_core::registry::SkillUpdateResult;
use skills_core::{AppDirs, Database, ProviderRegistry, Registry};

pub async fn run(
    dirs: &AppDirs,
    db: &Database,
    providers: &ProviderRegistry,
    action: SkillAction,
) -> Result<()> {
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
            } else if let Some(provider) = providers.detect(&source) {
                println!("Downloading via {} provider...", provider.provider_type());
                let name = registry.add_from_provider(&source, provider).await?;
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
                println!(
                    "Source '{}' is not a local path or recognized remote URL.",
                    source
                );
                println!("Supported formats:");
                println!("  Local:   /path/to/skill-dir");
                println!("  GitHub:  https://github.com/owner/repo/tree/main/path");
                println!("  Short:   owner/repo/path/to/skill");
                println!("  Hub URL: https://clawhub.ai/owner/skill-name");
            }
        }
        SkillAction::Update { name, all } => {
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
                match registry
                    .update_from_remote(skill_name, Some(providers))
                    .await
                {
                    Ok(SkillUpdateResult::Updated { name, new_hash, .. }) => {
                        let replaced =
                            skills_core::placements::replace_skill(dirs, db, &name).await?;
                        println!(
                            "  {} — updated (hash: {}…, {} placements refreshed)",
                            name,
                            &new_hash[..20.min(new_hash.len())],
                            replaced
                        );
                        updated += 1;
                    }
                    Ok(SkillUpdateResult::AlreadyUpToDate { name }) => {
                        println!("  {} — already up to date", name);
                    }
                    Ok(SkillUpdateResult::Skipped { name, reason }) => {
                        println!("  {} — skipped ({})", name, reason);
                    }
                    Ok(SkillUpdateResult::Failed { name, error }) => {
                        println!("  {} — FAILED: {}", name, error);
                    }
                    Err(e) => {
                        println!("  {} — FAILED: {}", skill_name, e);
                    }
                }
            }
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
                        details: &format!("Updated {} skills from remote", updated),
                    },
                )
                .await?;
            }
        }
        SkillAction::Sync => {
            println!("Syncing all remote-sourced skills...");
            let results = registry.sync_all(Some(providers)).await?;
            let mut updated = 0;
            for result in &results {
                match result {
                    SkillUpdateResult::Updated { name, new_hash, .. } => {
                        let replaced =
                            skills_core::placements::replace_skill(dirs, db, name).await?;
                        println!(
                            "  {} — updated (hash: {}…, {} placements refreshed)",
                            name,
                            &new_hash[..20.min(new_hash.len())],
                            replaced
                        );
                        updated += 1;
                    }
                    SkillUpdateResult::AlreadyUpToDate { name } => {
                        println!("  {} — already up to date", name);
                    }
                    SkillUpdateResult::Skipped { name, reason } => {
                        println!("  {} — skipped ({})", name, reason);
                    }
                    SkillUpdateResult::Failed { name, error } => {
                        println!("  {} — FAILED: {}", name, error);
                    }
                }
            }
            println!("\n{} skills updated", updated);
            if updated > 0 {
                logging::log(
                    db,
                    LogEntry {
                        source: Source::Cli,
                        agent_name: None,
                        operation: "skill_sync",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Synced {} skills from remote", updated),
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
        SkillAction::Discover { global_only } => {
            super::discover::run_discover(dirs, db, global_only).await?;
        }
        SkillAction::LinkRemote {
            name,
            url,
            subpath,
            git_ref,
        } => {
            super::discover::run_link_remote(dirs, &name, &url, subpath.as_deref(), &git_ref)?;
        }
        SkillAction::UnlinkRemote { name } => {
            super::discover::run_unlink_remote(dirs, &name)?;
        }
        SkillAction::Browse { source, hub } => {
            let input = if let Some(hub_name) = hub {
                format!("hub:{}", hub_name)
            } else if let Some(src) = source {
                src
            } else {
                println!("Specify a source URL or --hub <name>");
                println!("Examples:");
                println!("  skill browse owner/repo");
                println!("  skill browse --hub clawhub");
                return Ok(());
            };

            let provider = providers
                .detect(&input)
                .ok_or_else(|| anyhow::anyhow!("No provider found for '{}'", input))?;

            println!("Browsing via {} provider...", provider.provider_type());

            let staging_dir = dirs.cache().join("staging");
            let skills = provider.download_to_staging(&input, &staging_dir).await?;

            if skills.is_empty() {
                println!("No skills found.");
            } else {
                println!("\nFound {} skill(s):\n", skills.len());
                for s in &skills {
                    let desc = s.description.as_deref().unwrap_or("(no description)");
                    println!("  {} — {}", s.name, desc);
                    println!("    subpath: {}", s.subpath);
                }
                println!("\nUse `skills-mgr skill add <source>` to install a skill.");
            }
        }
        SkillAction::Hubs => {
            let hubs = skills_core::config::merge_hubs(&dirs.settings_toml());

            if hubs.is_empty() {
                println!("No hubs configured.");
            } else {
                println!("Configured hubs:\n");
                for hub in &hubs {
                    let status = if hub.enabled { "enabled" } else { "disabled" };
                    println!(
                        "  {} ({}) [{}] — {}",
                        hub.display_name, hub.name, status, hub.base_url
                    );
                }
                println!("\nUse `skills-mgr skill browse --hub <name>` to browse a hub.");
            }
        }
    }

    Ok(())
}
