use clap::Parser;
use std::{path::PathBuf, process::Command, sync::atomic::{AtomicBool, Ordering}};

pub mod cache;
pub mod fetchers;
pub mod lyrics;
pub mod modes;
pub mod song;

use crate::{lyrics::Language, song::{Player, get_flag_from_player}};

pub static SHOW_INFO: AtomicBool = AtomicBool::new(true);

pub fn info_log(message: impl ToString) {
    if SHOW_INFO.load(Ordering::Relaxed) {
        println!("[INFO] {}", message.to_string());
    }
}

pub fn command(command: &str) -> String {
    let mut parts = command.split_whitespace().collect::<Vec<&str>>();

    let stdout = Command::new(parts.remove(0))
        .args(parts)
        .output()
        .unwrap_or_else(|error| panic!("Failed to execute command '{}' with error '{}'", command, error))
        .stdout;

    String::from_utf8(stdout).expect("Stdout was not valid UTF-8")
}

pub fn get_position(player: &Option<Player>) -> f64 {
    let flag = get_flag_from_player(player);
    command(&format!("playerctl {flag} position"))
        .trim()
        .parse()
        .unwrap_or(0.)
}

pub fn to_pinyin(line: &str) -> String {
    let mut translated = String::new();
    let mut last_pinyin = false;
    for character in line.chars() {
        let res = match mandarin_to_pinyin::lookup_chars(&[character]) {
            Ok(pinyin) => match pinyin.vec[0] {
                Some(ref result) => result[0].clone(),
                None => character.to_string()
            },
            Err(_) => character.to_string(),
        };

        if last_pinyin { translated += " " };
        translated += &res;
        last_pinyin = res.len() != 1;
    }

    translated.trim().to_string()
}

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// List of languages that shouldn't be romanized,
    /// separated by a comma.
    #[arg(value_enum, value_delimiter = ',', short, long)]
    pub dont_romanize: Vec<Language>,

    /// Hide the information log (such as the current song that's being requested).
    #[arg(long)]
    pub hide_info_log: bool,

    /// The directory that stores the cache file(s).
    /// This is $XDG_CACHE_HOME or $HOME/.cache by default.
    #[arg(short, long)]
    pub cache_dir: Option<PathBuf>,
}
