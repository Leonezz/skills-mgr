mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use skills_core::config::ProfilesConfig;
use skills_core::profiles;
use skills_core::registry::compute_tree_hash;
use skills_core::{AppDirs, Database, Registry};

#[derive(Parser)]
#[command(name = "skills-mgr", about = "Cross-agent skill management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage skills in the registry
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Manage agent configurations
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Manage global skills (placed in agent global paths)
    Global {
        #[command(subcommand)]
        action: GlobalAction,
    },
    /// Show active profiles and placements for a project
    Status {
        /// Target project path (default: current directory)
        #[arg(long)]
        project: Option<String>,
    },
    /// Scan for overlapping skills across active profiles
    CheckConflicts {
        #[arg(long)]
        project: Option<String>,
    },
    /// Verify placements match DB, check for orphans
    Doctor,
    /// Estimate token cost of active or specified profile
    Budget {
        /// Profile name (default: current active set)
        profile: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Show recent operations
    Log {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value = "20")]
        limit: i64,
    },
}

#[derive(Subcommand)]
pub enum SkillAction {
    List,
    Add {
        source: String,
    },
    Remove {
        name: String,
    },
    Update {
        name: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Info {
        name: String,
    },
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    Open {
        name: String,
    },
    Files {
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ProfileAction {
    List,
    Create {
        name: String,
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,
    },
    Delete {
        name: String,
    },
    Show {
        name: String,
    },
    Edit {
        name: String,
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        remove: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,
    },
    Activate {
        name: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        force: bool,
    },
    Deactivate {
        name: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        global: bool,
    },
    Switch {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum GlobalAction {
    /// Show current global skills status
    Status,
    /// Activate global skills (place into agent global paths)
    Activate,
    /// Deactivate global skills (remove from agent global paths)
    Deactivate,
    /// Add skills to the global configuration
    Add {
        #[arg(required = true, num_args = 1..)]
        skills: Vec<String>,
    },
    /// Remove skills from the global configuration
    Remove {
        #[arg(required = true, num_args = 1..)]
        skills: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum AgentAction {
    List,
    Add {
        name: String,
        #[arg(long)]
        project_path: String,
        #[arg(long)]
        global_path: String,
    },
    Remove {
        name: String,
    },
    Enable {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
    Disable {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let base = AppDirs::default_base()?;
    let dirs = AppDirs::new(base);
    dirs.ensure_dirs()?;

    let db = Database::open(&dirs.database()).await?;

    match cli.command {
        Commands::Skill { action } => commands::skill::run(&dirs, &db, action).await?,
        Commands::Profile { action } => commands::profile::run(&dirs, &db, action).await?,
        Commands::Agent { action } => commands::agent::run(&dirs, &db, action).await?,
        Commands::Global { action } => commands::global::run(&dirs, &db, action).await?,
        Commands::Status { project } => commands::status::run(&dirs, &db, project).await?,
        Commands::Log {
            project: _,
            source: _,
            limit,
        } => {
            commands::util::show_log(&db, limit).await?;
        }
        Commands::CheckConflicts { project } => {
            run_check_conflicts(&dirs, &db, project).await?;
        }
        Commands::Doctor => {
            run_doctor(&dirs, &db).await?;
        }
        Commands::Budget { profile, project } => {
            run_budget(&dirs, profile, project)?;
        }
    }

    Ok(())
}

async fn run_check_conflicts(dirs: &AppDirs, db: &Database, project: Option<String>) -> Result<()> {
    let project_path = match project {
        Some(p) => std::fs::canonicalize(&p)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };
    let profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let project_id = db.get_or_create_project(&project_path, None).await?;
    let active = db.get_active_profiles(project_id).await?;

    if active.is_empty() {
        println!("No active profiles for {}", project_path);
        return Ok(());
    }

    // Check for missing skills referenced in profiles
    let registry = Registry::new(dirs.clone());
    let registry_skills: Vec<String> = registry.list()?.into_iter().map(|s| s.name).collect();
    let missing = profiles::validate_skills_exist(&profiles_config, &registry_skills);
    if !missing.is_empty() {
        println!("Missing skills (referenced in profiles but not in registry):");
        for s in &missing {
            println!("  - {}", s);
        }
    }

    // Check for overlapping skills across active profiles
    let mut skill_sources: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for profile_name in &active {
        if let Ok(skills) = profiles::resolve_profile(&profiles_config, profile_name, false) {
            for skill in skills {
                skill_sources
                    .entry(skill)
                    .or_default()
                    .push(profile_name.clone());
            }
        }
    }

    let overlaps: Vec<_> = skill_sources
        .iter()
        .filter(|(_, sources)| sources.len() > 1)
        .collect();

    if overlaps.is_empty() && missing.is_empty() {
        println!(
            "No conflicts found for {} ({} active profiles)",
            project_path,
            active.len()
        );
    } else if !overlaps.is_empty() {
        println!("\nShared skills across profiles (not necessarily conflicts):");
        for (skill, sources) in &overlaps {
            println!("  {} — used by: {}", skill, sources.join(", "));
        }
    }

    Ok(())
}

async fn run_doctor(dirs: &AppDirs, _db: &Database) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    let mut issues = 0;

    // 1. Check registry skill hashes match stored hashes
    println!("Checking registry integrity...");
    let sources = skills_core::SourcesConfig::load(&dirs.sources_toml())?;
    let skills = registry.list()?;
    for skill in &skills {
        if let Some(source) = &skill.source {
            if let Some(stored_hash) = &source.hash {
                let actual_hash = compute_tree_hash(&skill.dir_path)?;
                if *stored_hash != actual_hash {
                    println!(
                        "  MISMATCH: {} — stored hash differs from content",
                        skill.name
                    );
                    println!("    stored:  {}", stored_hash);
                    println!("    actual:  {}", actual_hash);
                    issues += 1;
                }
            }
        } else if !sources.skills.contains_key(&skill.name) {
            println!("  UNTRACKED: {} — not in sources.toml", skill.name);
            issues += 1;
        }
    }

    // 2. Check sources.toml references exist
    for name in sources.skills.keys() {
        if !registry.exists(name) {
            println!(
                "  ORPHAN SOURCE: {} — in sources.toml but not in registry",
                name
            );
            issues += 1;
        }
    }

    // 3. Check profile references
    println!("Checking profile references...");
    let profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let registry_names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
    let missing = profiles::validate_skills_exist(&profiles_config, &registry_names);
    for m in &missing {
        println!(
            "  MISSING SKILL: {} — referenced in profile but not in registry",
            m
        );
        issues += 1;
    }

    // 4. Validate no cycles
    if let Err(e) = profiles::validate_no_cycles(&profiles_config) {
        println!("  CYCLE: {}", e);
        issues += 1;
    }

    if issues == 0 {
        println!("\nAll checks passed. No issues found.");
    } else {
        println!("\nFound {} issue(s).", issues);
    }

    Ok(())
}

fn run_budget(dirs: &AppDirs, profile: Option<String>, project: Option<String>) -> Result<()> {
    if project.is_some() {
        eprintln!("warning: --project filter is not yet implemented, ignoring");
    }
    let profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let registry = Registry::new(dirs.clone());

    let skill_names = if let Some(profile_name) = &profile {
        profiles::resolve_profile(&profiles_config, profile_name, true)?
    } else {
        registry.list()?.into_iter().map(|s| s.name).collect()
    };

    let mut total_bytes: u64 = 0;
    let mut total_tokens: u64 = 0;
    let mut total_files = 0;

    for skill_name in &skill_names {
        match registry.get(skill_name)? {
            Some(skill) => {
                println!(
                    "  {} — {} files, {} text bytes (~{} tokens)",
                    skill.name,
                    skill.files.len(),
                    skill.total_bytes,
                    skill.token_estimate
                );
                total_bytes += skill.total_bytes;
                total_tokens += skill.token_estimate;
                total_files += skill.files.len();
            }
            None => println!("  {} — not found", skill_name),
        }
    }

    println!(
        "\nTotal: {} skills, {} files, {} text bytes (~{} tokens)",
        skill_names.len(),
        total_files,
        total_bytes,
        total_tokens
    );
    if let Some(p) = &profile {
        println!("Profile: {}", p);
    }

    Ok(())
}
