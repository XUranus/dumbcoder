use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::config::Config;
use crate::security::SecurityFilter;

pub fn run(query: &str, lang: Option<&str>) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    let mut cmd = Command::new("rg");
    cmd.arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=5");

    if let Some(l) = lang {
        cmd.arg("--type").arg(l);
    }

    cmd.arg(query).arg(&root);

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut count = 0;

    for line in stdout.lines() {
        if let Some((file_part, rest)) = line.split_once(':') {
            let file_path = std::path::Path::new(file_part);
            if !security.is_path_allowed(file_path, &root) {
                continue;
            }
            if let Some((line_num, content)) = rest.split_once(':') {
                count += 1;
                println!(
                    "  {}:{}: {}",
                    file_part.blue(),
                    line_num.yellow(),
                    content
                );
            }
        }
    }

    if count == 0 {
        println!("  No results found for: {query}");
    } else {
        eprintln!("\n  {} result(s) found", count);
    }

    Ok(())
}
