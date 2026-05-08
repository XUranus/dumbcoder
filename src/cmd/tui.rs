use anyhow::Result;

use crate::tui;

pub async fn run() -> Result<()> {
    tui::run().await
}
