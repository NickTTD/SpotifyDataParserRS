use serde::Deserialize;

fn default_false() -> Option<bool> {
    Some(false)
}

#[derive(Deserialize)]
pub struct RawStreamEntry {
    pub ts: String,
    #[serde(default)]
    pub platform: String,
    pub ms_played: u64,
    #[serde(default)]
    pub conn_country: String,

    // Music
    pub master_metadata_track_name: Option<String>,
    #[serde(default)]
    pub master_metadata_album_artist_name: Option<String>,
    #[serde(default)]
    pub master_metadata_album_album_name: Option<String>,
    pub spotify_track_uri: Option<String>,

    // Podcasts
    pub episode_name: Option<String>,
    #[serde(default)]
    pub episode_show_name: Option<String>,
    pub spotify_episode_uri: Option<String>,

    // Audiobooks
    pub audiobook_title: Option<String>,
    pub audiobook_uri: Option<String>,
    pub audiobook_chapter_uri: Option<String>,
    #[serde(default)]
    pub audiobook_chapter_title: Option<String>,

    // Playback metadata
    #[serde(default)]
    pub reason_start: String,
    #[serde(default)]
    pub reason_end: String,
    #[serde(default = "default_false")]
    pub shuffle: Option<bool>,
    #[serde(default = "default_false")]
    pub skipped: Option<bool>,
}

pub enum StreamKind<'a> {
    Music(&'a RawStreamEntry),
    Podcast(&'a RawStreamEntry),
    Audiobook(&'a RawStreamEntry),
    Unknown,
}

impl RawStreamEntry {
    pub fn classify(&self) -> StreamKind<'_> {
        if self
            .master_metadata_track_name
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        {
            StreamKind::Music(self)
        } else if self
            .episode_name
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        {
            StreamKind::Podcast(self)
        } else if self
            .audiobook_title
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        {
            StreamKind::Audiobook(self)
        } else {
            StreamKind::Unknown
        }
    }
}
