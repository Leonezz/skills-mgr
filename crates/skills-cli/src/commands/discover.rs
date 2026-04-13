use anyhow::Result;
use skills_core::config::{AppDirs, ProfileDef, ProfilesConfig};
use skills_core::discovery::{self, DiscoveredSkill, DiscoveryScope};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::placements::GLOBAL_PROJECT_PATH;
use skills_core::{AgentsConfig, Database, Registry};

pub async fn run_discover(
    dirs: &AppDirs,
    db: &Database,
    global_only: bool,
    delegate_to: Option<&str>,
) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    if agents_config.agents.is_empty() {
        println!("No agents configured. Add agents first with `skills-mgr agent add`.");
        return Ok(());
    }

    let all_projects = db.list_all_projects().await?;

    let project_paths = if global_only {
        vec![]
    } else {
        all_projects
            .iter()
            .filter(|p| p.path != GLOBAL_PROJECT_PATH)
            .map(|p| p.path.clone())
            .collect()
    };

    let placed_paths = db.collect_placed_paths().await?;

    let discovered = discovery::scan_all_agents(
        dirs,
        &registry,
        &agents_config,
        &project_paths,
        &placed_paths,
    )?;

    if discovered.is_empty() {
        println!("No unmanaged skills found in agent paths.");
        return Ok(());
    }

    print_discovered(&discovered);

    if let Some(profile_name) = delegate_to {
        delegate(dirs, db, &registry, &discovered, profile_name).await?;
    } else {
        println!(
            "\nUse `skills-mgr skill discover --delegate <profile>` to import and assign these skills."
        );
    }
    Ok(())
}

fn print_discovered(discovered: &[DiscoveredSkill]) {
    println!("Found {} unmanaged skill(s):\n", discovered.len());

    let mut current_scope = None;
    for skill in discovered {
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
}

async fn delegate(
    dirs: &AppDirs,
    db: &Database,
    registry: &Registry,
    discovered: &[DiscoveredSkill],
    profile_name: &str,
) -> Result<()> {
    // Only delegate skills that don't already exist in the registry
    let to_delegate: Vec<_> = discovered
        .iter()
        .filter(|s| !s.exists_in_registry)
        .collect();

    if to_delegate.is_empty() {
        println!("\nAll discovered skills already exist in the registry.");
        return Ok(());
    }

    println!(
        "\nDelegating {} skill(s) to profile '{}'...",
        to_delegate.len(),
        profile_name
    );

    let mut imported = Vec::new();
    for skill in &to_delegate {
        match registry.delegate(&skill.found_path, &skill.agent_name) {
            Ok(name) => {
                println!("  Imported '{}'", name);
                imported.push(name);
            }
            Err(e) => {
                println!("  Failed '{}': {}", skill.name, e);
            }
        }
    }

    if imported.is_empty() {
        println!("No skills were imported.");
        return Ok(());
    }

    // Add imported skills to the target profile
    let mut profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let profile = profiles_config
        .profiles
        .entry(profile_name.to_string())
        .or_insert_with(|| {
            println!("  Created new profile '{}'", profile_name);
            ProfileDef {
                description: None,
                skills: vec![],
                includes: vec![],
            }
        });

    for name in &imported {
        if !profile.skills.contains(name) {
            profile.skills.push(name.clone());
        }
    }
    profiles_config.save(&dirs.profiles_toml())?;

    println!(
        "\nDelegated {} skills to profile '{}'",
        imported.len(),
        profile_name
    );

    logging::log(
        db,
        LogEntry {
            source: Source::Cli,
            agent_name: None,
            operation: "skill_delegate",
            params: None,
            project_path: None,
            result: "success",
            details: &format!(
                "Delegated {} skills to profile '{}'",
                imported.len(),
                profile_name
            ),
        },
    )
    .await?;

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
