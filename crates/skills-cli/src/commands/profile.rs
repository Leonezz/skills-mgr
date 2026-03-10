use anyhow::Result;
use skills_core::config::{ProfileDef, ProfilesConfig, AgentsConfig};
use skills_core::{AppDirs, Database};
use skills_core::placements;
use skills_core::profiles;
use crate::ProfileAction;

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
        ProfileAction::Create { name, add, include } => {
            let profile = ProfileDef {
                description: None,
                skills: add,
                includes: include,
            };
            profiles_config.profiles.insert(name.clone(), profile);
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Created profile '{}'", name);
        }
        ProfileAction::Delete { name } => {
            if profiles_config.profiles.remove(&name).is_some() {
                profiles_config.save(&dirs.profiles_toml())?;
                println!("Deleted profile '{}'", name);
            } else {
                println!("Profile '{}' not found", name);
            }
        }
        ProfileAction::Edit { name, add, remove, include } => {
            let profile = profiles_config.profiles.get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
            for s in &add { if !profile.skills.contains(s) { profile.skills.push(s.clone()); } }
            profile.skills.retain(|s| !remove.contains(s));
            for i in &include { if !profile.includes.contains(i) { profile.includes.push(i.clone()); } }
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Updated profile '{}'", name);
        }
        ProfileAction::Activate { name, project, global: _, force } => {
            let project_path = resolve_project_path(project)?;
            let result = placements::activate(dirs, db, &profiles_config, &agents_config, &name, &project_path, force).await?;
            println!("Activated profile '{}' for {}", result.profile_name, project_path);
            println!("  {} skills -> {} ({} placements)", result.skills_placed, result.agents_used.join(", "), result.total_placements);
        }
        ProfileAction::Deactivate { name, project, global: _ } => {
            let project_path = resolve_project_path(project)?;
            let result = placements::deactivate(db, &name, &project_path).await?;
            println!("Deactivated profile '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept);
        }
        ProfileAction::Switch { name, project } => {
            let project_path = resolve_project_path(project)?;
            let project_id = db.get_or_create_project(&project_path, None).await?;
            let active = db.get_active_profiles(project_id).await?;

            // Deactivate all current profiles (except base)
            for p in &active {
                placements::deactivate(db, p, &project_path).await?;
            }

            // Activate new profile
            let result = placements::activate(dirs, db, &profiles_config, &agents_config, &name, &project_path, false).await?;
            println!("Switched to profile '{}' ({} placements)", result.profile_name, result.total_placements);
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
