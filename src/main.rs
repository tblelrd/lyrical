use anyhow::Result;
use clap::Parser;
use std::sync::atomic::Ordering;

use lyrical::{Cli, SHOW_INFO, cache::Cache, modes::default::run_default};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cache = Cache::new();

    SHOW_INFO.store(!cli.hide_info_log, Ordering::Relaxed);
    run_default(cli.dont_romanize, cache).await?;
    Ok(())
}
