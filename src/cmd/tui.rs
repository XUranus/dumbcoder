use anyhow::Result;

use crate::tui;

pub async fn run(resume: Option<&str>) -> Result<()> {
    tui::run(resume).await
}
