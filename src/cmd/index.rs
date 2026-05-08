use anyhow::Result;

use crate::config::{Config, DUMBCODER_DIR};
use crate::index::IndexStore;
use crate::security::SecurityFilter;
use crate::util;

pub fn run(full: bool, changed: bool) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");

    util::header("Indexing codebase");
    util::info("project root", &root.display().to_string());
    util::info("database", &db_path.display().to_string());

    let store = IndexStore::open(&db_path)?;

    let do_full = full || !changed;

    let stats = if do_full {
        eprintln!("  Running full index...");
        store.index_all(&root, &security)?
    } else {
        eprintln!("  Running incremental index (changed files only)...");
        store.index_changed(&root, &security)?
    };

    let (total_files, total_symbols) = store.total_stats()?;

    util::header("Index complete");
    util::info("files scanned", &stats.files_scanned.to_string());
    util::info("files indexed", &stats.files_indexed.to_string());
    util::info("symbols found", &stats.symbols_found.to_string());
    util::info("elapsed", &format!("{}ms", stats.elapsed_ms));
    util::info("total indexed files", &total_files.to_string());
    util::info("total indexed symbols", &total_symbols.to_string());

    Ok(())
}
