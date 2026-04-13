use kakasi::IsJapanese;
use serde_json::Value;

#[derive(Debug, Default)]
/// Languages currently supported to be romanized.
pub enum Language {
    /// This should be English and
    /// other latin alphabet based langauges,
    /// or unsupported languages.
    #[default]
    Other,
    Chinese,
    Japanese,
}

/// A struct that contains the language and the lyrics
/// of a song. The lyrics should be timestamped.
pub struct Lyrics {
    pub language: Language,
    pub duration: f64,
    pub lyrics: Vec<(f64, String)>,
}

impl Lyrics {
    /// Creates a [Lyrics] object from
    /// a [Value] json object using the
    /// `syncedLyrics` attribute.
    pub fn from_json(object: &Value) -> Option<Lyrics> {
        let synced = object["syncedLyrics"].as_str()?.to_string();

        let timestamped: Vec<(f64, String)> = synced
            .split('\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            // Map into an option of a tuple.
            // The ? syntax will propogate the None up into the full option
            // so instead of (Option<f32, String) it will be Option<(f32, String)>
            // That's why the Some is at the front, to explicitly declare that it will
            // return an option.
            .map(|s| Some((get_time_from_string(&s[..10])?, s[10..].trim().to_string())))
            // The turbofish is to tell collect to use the option implementation rather than the
            // default vec<> collect method.
            // Then we do ? again to propogate any none values in the lyrics to return
            // none.
            .collect::<Option<_>>()?;

        let duration = timestamped
            .iter()
            .fold(0f64, |a, (b, _)| a + b);

        let language = object["plainLyrics"].as_str().map_or_else(
            // No plainLyrics, make plain lyrics.
            || get_language_from_text(&timestamped
                .iter()
                .map(|(_, line)| line)
                .fold(String::new(), |a, b| a + b + &"\n")),
            // Yes plainLyrics, just plug that in.
            get_language_from_text
        );

        Some(Lyrics {
            language,
            duration,
            lyrics: timestamped,
        })
    }

    pub fn get_line_at_time(&self, time: f64) -> &str {
        match self.lyrics.iter().rev().find(|l| time > l.0) {
            Some(l) => &l.1,
            None => "",
        }
    }
}

fn get_language_from_text(lyrics: &str) -> Language{
    match kakasi::is_japanese(lyrics) {
        IsJapanese::False => Language::Other,
        IsJapanese::Maybe => Language::Chinese,
        IsJapanese::True => Language::Japanese,
    }
}

/// Expects a string slice that is in the format
/// of `[mm:ss:xx]` where mm is minutes, ss is seconds
/// and xx is milliseconds.
fn get_time_from_string(time: &str) -> Option<f64> {
    let minutes: f64 = time[1..3].parse().ok()?;
    let seconds: f64 = time[4..6].parse().ok()?;
    let milliseconds: f64 = time[7..9].parse().ok()?;

    Some(
        (minutes * 60.) + (seconds) + (milliseconds / 100.)
    )
}
