use std::collections::HashMap;

use crate::{lyrics::Lyrics, song::{SongData}};

/// A simple cache implementation for lyrics.
#[derive(Debug)]
pub struct Cache {
    map: HashMap<String, Vec<CacheEntry>>
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            map: HashMap::new(),
        }
    }

    pub fn get_lyrics(&mut self, data: &SongData) -> Option<Lyrics> {
        let entries = self.map.get_mut(&data.title)?;

        let found = entries.iter_mut().fold(None::<&mut CacheEntry>, |closest_match, entry| {
            let closesness = match &closest_match {
                Some(closest) => get_closeness(&closest.data, &data),
                None => 0,
            };
            let entry_closeness = get_closeness(&entry.data, &data);

            if closesness > entry_closeness {
                // Closest match is closest.
                closest_match
            } else if entry_closeness > 0 {
                // Entry is closest but not 0.
                Some(entry)
            } else {
                // Both are 0.
                closest_match
            }
        })?;
        found.occurences += 1;

        found.lyrics.clone()
    }

    pub fn save_lyrics(&mut self, data: &SongData, lyrics: &Option<Lyrics>) {
        let entries = match self.map.get_mut(&data.title) {
            Some(entries) => entries,
            None => {
                self.map.insert(data.title.clone(), vec![]);
                self.map.get_mut(&data.title).expect("Should be inserted now. (unreachable?)")
            },
        };

        entries.push(CacheEntry::new(data.clone(), lyrics.clone()));
    }
}

fn get_closeness(a: &SongData, b: &SongData) -> u8 {
    let mut closeness = 0;
    if attribute_close(&a.artist, &b.artist) { closeness += 1 };
    if attribute_close(&a.album, &b.album) { closeness += 1 };

    // Duration match should be worth more.
    if attribute_close(&a.duration, &b.duration) { closeness += 2 };

    closeness
}

/// Matching two optionals based on the target specifier.
/// Hard to explain, just read the code.
fn attribute_close<T>(attribute: &Option<T>, target: &Option<T>) -> bool
where
    T: PartialEq,
{
    match (attribute, target) {
        // If both are some, then compare
        (Some(a), Some(b)) => *a == *b,
        // If target is none, then it matches attribute because it's an unspecific match.
        (Some(_), None) => true,
        // If target is some, it doesn't match because its too specific.
        (None, Some(_)) => false,
        // If both none, then non specific matches non specific.
        (None, None) => true,
    }
}

#[derive(Clone, Debug)]
struct CacheEntry {
    occurences: usize,
    data: SongData,
    lyrics: Option<Lyrics>,
}

impl CacheEntry {
    fn new(data: SongData, lyrics: Option<Lyrics>) -> CacheEntry {
        CacheEntry {
            occurences: 0,
            data,
            lyrics,
        }
    }
}
