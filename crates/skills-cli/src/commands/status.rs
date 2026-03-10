use anyhow::Result;
use skills_core::config::ProfilesConfig;
use skills_core::{AppDirs, Database};
use skills_core::placements;

pub async fn run(dirs: &AppDirs, db: &Database, project: Option<String>) -> Result<()> {
    let project_path = match project {
        Some(p) => std::fs::canonicalize(&p)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };

    let profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let s = placements::status(db, &profiles_config, &project_path).await?;

    println!("Project: {}", s.project_path);
    println!("Base: [{}]", s.base_skills.join(", "));
    if s.active_profiles.is_empty() {
        println!("Active profiles: (none)");
    } else {
        println!("Active profiles: [{}]", s.active_profiles.join(", "));
    }
    println!("Placements: {}", s.placement_count);

    Ok(())
}
