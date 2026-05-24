use clap::Parser;
use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering}};

pub mod cache;
pub mod fetchers;
pub mod lyrics;
pub mod modes;
pub mod song;

use crate::lyrics::Language;

pub static SHOW_INFO: AtomicBool = AtomicBool::new(true);

pub fn info_log(message: impl ToString) {
    if SHOW_INFO.load(Ordering::Relaxed) {
        println!("[INFO] {}", message.to_string());
    }
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

    /// Maximum number of cache entries in the cache.
    ///
    /// Each entry is usually 2KiB and the default is 500.
    /// So, the default max cache size is around 1MiB.
    #[arg(short, long, default_value = "500")]
    pub max_items: usize
}
