mod audit;
mod cmd;
mod config;
mod context;
mod git;
mod index;
mod model;
mod plugin;
mod security;
mod session;
mod tool;
mod tui;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dumbcoder", version, about = "Intranet AI Coding Assistant CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize project configuration
    Init,
    /// Ask questions about the codebase
    Ask {
        /// The question to ask
        question: String,
    },
    /// Explain a file, function, or code snippet
    Explain {
        /// File path to explain
        path: String,
        /// Optional symbol name to explain
        #[arg(long)]
        symbol: Option<String>,
    },
    /// Smart code search
    Search {
        /// Search query
        query: String,
        /// Filter by language
        #[arg(long)]
        lang: Option<String>,
    },
    /// Build or update code index
    Index {
        /// Full re-index of all files
        #[arg(long)]
        full: bool,
        /// Incremental index (changed files only)
        #[arg(long)]
        changed: bool,
    },
    /// Generate or supplement unit tests
    Test {
        /// File path
        path: String,
        /// Symbol name
        #[arg(long)]
        symbol: Option<String>,
    },
    /// Review git diff
    Review {
        /// Review staged changes
        #[arg(long)]
        staged: bool,
        /// Diff range (e.g. main...HEAD)
        #[arg(long)]
        diff: Option<String>,
    },
    /// Generate controlled code patch
    Patch {
        /// Description of the fix
        description: String,
    },
    /// Run a plugin command
    Run {
        /// Plugin name
        name: String,
        /// Task description / query
        query: String,
    },
    /// Enter interactive TUI mode
    Tui {
        /// Resume a previous session by ID
        #[arg(long)]
        resume: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cmd::init::run()?,
        Commands::Ask { question } => cmd::ask::run(&question).await?,
        Commands::Explain { path, symbol } => cmd::explain::run(&path, symbol.as_deref()).await?,
        Commands::Search { query, lang } => cmd::search::run(&query, lang.as_deref())?,
        Commands::Index { full, changed } => cmd::index::run(full, changed)?,
        Commands::Test { path, symbol } => cmd::test::run(&path, symbol.as_deref()).await?,
        Commands::Review { staged, diff } => cmd::review::run(staged, diff.as_deref()).await?,
        Commands::Patch { description } => cmd::patch::run(&description).await?,
        Commands::Run { name, query } => cmd::run::run(&name, &query).await?,
        Commands::Tui { resume } => cmd::tui::run(resume.as_deref()).await?,
    }

    Ok(())
}
