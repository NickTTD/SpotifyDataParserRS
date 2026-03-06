use crate::models::{RawStreamEntry, StreamKind};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

const DATA_DIR: &str = "Spotify Extended Streaming History";

fn find_json_files() -> Vec<PathBuf> {
    let dir = Path::new(DATA_DIR);
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .expect("Could not read directory 'Spotify Extended Streaming History'")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            if name.ends_with(".json")
                && (name.starts_with("Streaming_History_Audio_")
                    || name.starts_with("Streaming_History_Video_"))
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

fn parse_file(path: &Path) -> Vec<RawStreamEntry> {
    let data = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", path.display());
        String::new()
    });
    serde_json::from_str(&data).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {e}", path.display());
        Vec::new()
    })
}

pub fn cmd_import(conn: &mut Connection) {
    let files = find_json_files();
    println!("Importing {} files...", files.len());

    conn.execute_batch(
        "DELETE FROM music_streams; DELETE FROM podcast_streams; DELETE FROM audiobook_streams;",
    )
    .expect("Failed to clear tables");

    let mut music_count: u64 = 0;
    let mut podcast_count: u64 = 0;
    let mut audiobook_count: u64 = 0;
    let mut skipped: u64 = 0;

    for file in &files {
        let entries = parse_file(file);
        let file_total = entries.len();
        let tx = conn.savepoint().expect("Failed to create savepoint");

        for entry in &entries {
            match entry.classify() {
                StreamKind::Music(e) => {
                    tx.execute(
                        "INSERT INTO music_streams (ts, platform, ms_played, conn_country, spotify_track_uri, track_name, artist_name, album_name, reason_start, reason_end, shuffle, skipped)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        rusqlite::params![
                            e.ts,
                            e.platform,
                            e.ms_played,
                            e.conn_country,
                            e.spotify_track_uri,
                            e.master_metadata_track_name.as_deref().unwrap_or(""),
                            e.master_metadata_album_artist_name.as_deref().unwrap_or(""),
                            e.master_metadata_album_album_name.as_deref().unwrap_or(""),
                            e.reason_start,
                            e.reason_end,
                            e.shuffle.unwrap_or(false) as i32,
                            e.skipped.unwrap_or(false) as i32,
                        ],
                    )
                    .expect("Failed to insert music stream");
                    music_count += 1;
                }
                StreamKind::Podcast(e) => {
                    tx.execute(
                        "INSERT INTO podcast_streams (ts, platform, ms_played, conn_country, spotify_episode_uri, episode_name, show_name, reason_start, reason_end, shuffle, skipped)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        rusqlite::params![
                            e.ts,
                            e.platform,
                            e.ms_played,
                            e.conn_country,
                            e.spotify_episode_uri,
                            e.episode_name.as_deref().unwrap_or(""),
                            e.episode_show_name.as_deref().unwrap_or(""),
                            e.reason_start,
                            e.reason_end,
                            e.shuffle.unwrap_or(false) as i32,
                            e.skipped.unwrap_or(false) as i32,
                        ],
                    )
                    .expect("Failed to insert podcast stream");
                    podcast_count += 1;
                }
                StreamKind::Audiobook(e) => {
                    tx.execute(
                        "INSERT INTO audiobook_streams (ts, platform, ms_played, conn_country, audiobook_uri, audiobook_title, chapter_uri, chapter_title, reason_start, reason_end, shuffle, skipped)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        rusqlite::params![
                            e.ts,
                            e.platform,
                            e.ms_played,
                            e.conn_country,
                            e.audiobook_uri,
                            e.audiobook_title.as_deref().unwrap_or(""),
                            e.audiobook_chapter_uri,
                            e.audiobook_chapter_title.as_deref().unwrap_or(""),
                            e.reason_start,
                            e.reason_end,
                            e.shuffle.unwrap_or(false) as i32,
                            e.skipped.unwrap_or(false) as i32,
                        ],
                    )
                    .expect("Failed to insert audiobook stream");
                    audiobook_count += 1;
                }
                StreamKind::Unknown => {
                    skipped += 1;
                }
            }
        }

        tx.commit().expect("Failed to commit transaction");
        println!(
            "  {} — {} entries",
            file.file_name().unwrap_or_default().to_string_lossy(),
            file_total,
        );
    }

    let total = music_count + podcast_count + audiobook_count;
    println!("\nImport complete:");
    println!("  Music:      {music_count}");
    println!("  Podcasts:   {podcast_count}");
    println!("  Audiobooks: {audiobook_count}");
    println!("  Skipped:    {skipped}");
    println!("  Total:      {total}");
}
