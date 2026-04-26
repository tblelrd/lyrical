use anyhow::Result;
use clap::Parser;
use std::{path::PathBuf, sync::atomic::Ordering};

use lyrical::{Cli, MAX_SIZE, SHOW_INFO, cache::Cache, modes::default::run_default};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cache_dir = cli.cache_dir.unwrap_or(
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".cache"))
            })
            .map(|p| p.join("lyrical"))
            .expect("Couldn't find cache directory, please specify using --cache-dir")
    );
    let cache_path = &cache_dir.join("cache");

    MAX_SIZE.store(cli.max_items, Ordering::Relaxed);
    SHOW_INFO.store(!cli.hide_info_log, Ordering::Relaxed);

    let cache = Cache::read_from_file(cache_path, MAX_SIZE.load(Ordering::Relaxed)).await?;

    run_default(cli.dont_romanize, cache).await?;

    Ok(())
}
