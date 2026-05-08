use anyhow::Result;

use crate::config::Config;
use crate::util;

pub fn run() -> Result<()> {
    let root = std::env::current_dir()?;
    let config = Config::default();
    config.save(&root)?;

    util::header("dumbcoder initialized");
    util::info("project root", &root.display().to_string());

    let langs = util::detect_project_language(&root);
    util::info("detected language", &langs.join(", "));
    util::info("git repo", if util::is_git_repo(&root) { "yes" } else { "no" });
    util::info(
        "config",
        &root.join(".dumbcoder/config.toml").display().to_string(),
    );

    println!();
    println!("  Next steps:");
    println!("    dumbcoder ask \"your question\"");
    println!("    dumbcoder explain src/main.rs");
    println!("    dumbcoder search \"your query\"");
    println!();

    Ok(())
}
