mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;
use skills_core::{AppDirs, Database};

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
    Add { source: String },
    Remove { name: String },
    Update { name: Option<String>, #[arg(long)] all: bool },
    Info { name: String },
    Create { name: String, #[arg(long)] description: Option<String> },
    Open { name: String },
    Files { name: String },
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
    Delete { name: String },
    Show { name: String },
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
pub enum AgentAction {
    List,
    Add {
        name: String,
        #[arg(long)]
        project_path: String,
        #[arg(long)]
        global_path: String,
    },
    Remove { name: String },
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
        Commands::Status { project } => commands::status::run(&dirs, &db, project).await?,
        Commands::Log { project: _, source: _, limit } => {
            commands::util::show_log(&db, limit).await?;
        }
        Commands::CheckConflicts { project: _ } => {
            println!("check-conflicts: not yet implemented");
        }
        Commands::Doctor => {
            println!("doctor: not yet implemented");
        }
        Commands::Budget { profile: _, project: _ } => {
            println!("budget: not yet implemented");
        }
    }

    Ok(())
}
