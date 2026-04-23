use std::{collections::HashMap, path::{Path, PathBuf}};

use bincode_next::config;
use serde::{Deserialize, Serialize};
use tokio::{fs::{self, OpenOptions}, io::{self, AsyncReadExt, AsyncWriteExt}};

use crate::{info_log, lyrics::Lyrics, song::SongData};

/// A simple cache implementation for lyrics.
#[derive(Clone, Debug)]
pub struct Cache {
    map: HashMap<String, Vec<CacheEntry>>,
    location: PathBuf,
}

impl Cache {
    fn new(location: PathBuf) -> Cache {
        Cache {
            map: HashMap::new(),
            location,
        }
    }

    /// Reads the cache in that file, if theres a problem, will just generate an empty cache.
    /// Creates a file if not existing.
    /// Only errors when [OpenOptions::open] errors.
    pub async fn read_from_file(path: &Path) -> io::Result<Self> {
        // Create all parent directories if not found.
        fs::create_dir_all(path.parent().expect("No parent directories?")).await?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await?;

        let mut cache = Cache::new(path.into());

        let mut buf = vec![];
        file.read_to_end(&mut buf).await?;
        let entries: Vec<CacheEntry> = match bincode_next::serde::decode_from_slice(&buf, config::standard()) {
            Ok((entries, _)) => entries,

            // Empty or corrupted file (or just a random file), will overwrite anyways.
            Err(e) => {
                info_log(format!("Error when parsing: {}", e));
                return Ok(cache)
            },
        };
        dbg!(&entries.iter().map(|e| e.data.clone()).collect::<Vec<_>>());

        for entry in entries {
            cache.save_lyrics(&entry.data, &entry.lyrics, entry.occurences);
        }

        Ok(cache)
    }

    /// Saves the cache.
    /// Errors when unable to open the cache file (doesn't create a new one).
    /// Also errors when error with [AsyncWriteExt::write_all_buf].
    pub async fn save_to_file(&self) -> io::Result<()> {
        let entries = Into::<Vec<CacheEntry>>::into(self.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .open(&self.location)
            .await?;

        let serialized = bincode_next::serde::encode_to_vec(entries, config::standard())
            .expect("Serialization failure");
        file.write_all(&serialized).await?;
        
        Ok(())
    }

    /// Requests the closest matching lyrics from the cache.
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

    /// Saves the lyrics to the cache.
    pub fn save_lyrics(&mut self, data: &SongData, lyrics: &Option<Lyrics>, occurences: usize) {
        let entries = match self.map.get_mut(&data.title) {
            Some(entries) => entries,
            None => {
                self.map.insert(data.title.clone(), vec![]);
                self.map.get_mut(&data.title).expect("Should be inserted now. (unreachable?)")
            },
        };

        entries.push(CacheEntry::new(data.clone(), lyrics.clone(), occurences));
    }
}

/// Quantifies the closeness of two metadata.
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CacheEntry {
    occurences: usize,
    data: SongData,
    lyrics: Option<Lyrics>,
}

impl CacheEntry {
    fn new(data: SongData, lyrics: Option<Lyrics>, occurences: usize) -> CacheEntry {
        CacheEntry {
            occurences,
            data,
            lyrics,
        }
    }
}

impl From<Cache> for Vec<CacheEntry> {
    fn from(value: Cache) -> Self {
        value.map.into_iter()
            .map(|(_, b)| b)
            .flatten()
            .collect()
    }
}
