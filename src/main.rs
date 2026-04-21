use clap::Parser;
use anyhow::Result;
use std::{process::Command, sync::atomic::{AtomicBool, Ordering}};

use crate::{lyrics::Language, modes::default::run_default, song::{Player, get_flag_from_player}};

mod lyrics;
mod modes;
mod song;

static SHOW_INFO: AtomicBool = AtomicBool::new(true);

fn info_log(message: impl ToString) {
    if SHOW_INFO.load(Ordering::Relaxed) {
        println!("[INFO] {}", message.to_string());
    }
}

fn command(command: &str) -> String {
    let mut parts = command.split_whitespace().collect::<Vec<&str>>();

    let stdout = Command::new(parts.remove(0))
        .args(parts)
        .output()
        .unwrap_or_else(|error| panic!("Failed to execute command '{}' with error '{}'", command, error))
        .stdout;

    String::from_utf8(stdout).expect("Stdout was not valid UTF-8")
}

fn get_position(player: &Option<Player>) -> f64 {
    let flag = get_flag_from_player(player);
    command(&format!("playerctl {flag} position"))
        .trim()
        .parse()
        .unwrap_or(0.)
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// List of languages that shouldn't be romanized,
    /// separated by a comma.
    #[arg(value_enum, value_delimiter = ',', short, long)]
    dont_romanize: Vec<Language>,

    /// Hide the information log (such as the current song that's being requested).
    #[arg(long)]
    hide_info_log: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    SHOW_INFO.store(!cli.hide_info_log, Ordering::Relaxed);
    run_default(cli.dont_romanize).await?;
    Ok(())
}
