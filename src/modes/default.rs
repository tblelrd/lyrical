use anyhow::Result;
use futures::future::OptionFuture;
use std::{thread, time::Duration};

use crate::{fetchers, get_position, lyrics::Language, song::{Song, SongData}};

pub const UPDATE_PERIOD: f64 = 0.1f64;

fn to_pinyin(line: &str) -> String {
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

pub async fn run_default(dont_romanize: Vec<Language>) -> Result<()> {
    // Initialize chinese to pinyin map
    mandarin_to_pinyin::init_map(None).expect("Cant be bothered catching this one");

    // let mut metadata: Option<Metadata> = None;
    let mut song: Option<Song> = None;
    let mut previous_line = "".to_string();
    loop {
        thread::sleep(Duration::from_secs_f64(UPDATE_PERIOD));

        let data = SongData::get_data();
        if match (&data, &song) {
            // True if metadata and lyrics exist and are different
            // e.g. changing songs
            (Some(data), Some(song)) => data != &song.data,
            // True if lyrics exist but metadata doesn't
            // e.g. no more songs
            (None, Some(_)) => true,
            // True if metadata exists but lyrics don't
            // e.g. just started playing
            (Some(_), None) => true,
            // Else false
            // e.g. still playing same song
            _ => false,
        } {
            // Requests the song if exists, or None if no data.
            song = OptionFuture::from(
                data.map(|data| async {
                    let lyrics = fetchers::fetch_all(&data).await;
                    Song::new(data, lyrics)
                }),
            ).await;

            if let Some(song) = &song {
                // Empty line on no lyrics.
                if let None = &song.lyrics {
                    println!("");
                }
            // Empty line on no song.
            } else {
                println!("");
            }
        }

        let Some(song) = &song else { continue; };
        let Some(lyrics) = &song.lyrics else { continue; };

        let line = lyrics.get_line_at_time(get_position(&song.data.player));
        if line == previous_line { continue }
        previous_line = line.to_string();

        // TODO: Translate lyrics immediately on request,
        // to prevent exactly this.
        let line = if !dont_romanize.iter().any(|l| *l == lyrics.language) {
            match lyrics.language {
                Language::Japanese => kakasi::convert(line).romaji,
                Language::Korean => korean_romanize::convert(line),
                Language::Chinese => to_pinyin(line),
                Language::Other => line.to_string(),
            }
        } else { line.to_string() };

        println!("{}", line);
    }
}
