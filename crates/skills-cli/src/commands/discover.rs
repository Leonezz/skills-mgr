use anyhow::Result;
use skills_core::config::AppDirs;
use skills_core::discovery::{self, DiscoveryScope};
use skills_core::placements::GLOBAL_PROJECT_PATH;
use skills_core::{AgentsConfig, Database, Registry};

pub async fn run_discover(dirs: &AppDirs, db: &Database, global_only: bool) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    if agents_config.agents.is_empty() {
        println!("No agents configured. Add agents first with `skills-mgr agent add`.");
        return Ok(());
    }

    let project_paths = if global_only {
        vec![]
    } else {
        db.list_all_projects()
            .await?
            .into_iter()
            .filter(|p| p.path != GLOBAL_PROJECT_PATH)
            .map(|p| p.path)
            .collect()
    };

    let discovered = discovery::scan_all_agents(dirs, &registry, &agents_config, &project_paths)?;

    if discovered.is_empty() {
        println!("No unmanaged skills found in agent paths.");
        return Ok(());
    }

    println!("Found {} unmanaged skill(s):\n", discovered.len());

    let mut current_scope = None;
    for skill in &discovered {
        let scope_label = match &skill.scope {
            DiscoveryScope::Global => "Global".to_string(),
            DiscoveryScope::Project(p) => format!("Project: {}", p),
        };
        if current_scope.as_ref() != Some(&scope_label) {
            if current_scope.is_some() {
                println!();
            }
            println!("  [{}]", scope_label);
            current_scope = Some(scope_label);
        }

        let conflict = if skill.exists_in_registry {
            " (exists in registry)"
        } else {
            ""
        };
        println!(
            "    {} — {} ({} files, ~{} tokens) via {}{}",
            skill.name,
            skill.description.as_deref().unwrap_or("No description"),
            skill.files.len(),
            skill.token_estimate,
            skill.agent_name,
            conflict,
        );
    }

    println!("\nUse the GUI's Discover tab to delegate skills to a profile.");
    Ok(())
}

pub fn run_link_remote(
    dirs: &AppDirs,
    name: &str,
    url: &str,
    subpath: Option<&str>,
    git_ref: &str,
) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    registry.link_remote(name, url, subpath, git_ref)?;
    println!("Linked '{}' to remote: {} (ref: {})", name, url, git_ref);
    Ok(())
}

pub fn run_unlink_remote(dirs: &AppDirs, name: &str) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    registry.unlink_remote(name)?;
    println!("Unlinked '{}' from remote — reverted to local", name);
    Ok(())
}
