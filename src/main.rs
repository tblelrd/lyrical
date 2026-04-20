use clap::Parser;
use anyhow::Result;
use std::process::Command;

use crate::{lyrics::Language, modes::default::run_default, song::{Player, get_flag_from_player}};

mod lyrics;
mod modes;
mod song;

pub const UPDATE_PERIOD: f64 = 0.1f64;

fn info_log(message: impl ToString) {
    println!("[INFO] {}", message.to_string());
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run_default(cli.dont_romanize).await?;
    Ok(())
}
