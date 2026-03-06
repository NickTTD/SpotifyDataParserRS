use rusqlite::{Connection, Result};
use std::path::Path;

const DB_NAME: &str = "spotify.db";

pub fn open_db() -> Result<Connection> {
    let db_path = Path::new(DB_NAME);
    Connection::open(db_path)
}

pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS music_streams (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            platform TEXT NOT NULL DEFAULT '',
            ms_played INTEGER NOT NULL,
            conn_country TEXT NOT NULL DEFAULT '',
            spotify_track_uri TEXT,
            track_name TEXT NOT NULL,
            artist_name TEXT NOT NULL DEFAULT '',
            album_name TEXT NOT NULL DEFAULT '',
            reason_start TEXT NOT NULL DEFAULT '',
            reason_end TEXT NOT NULL DEFAULT '',
            shuffle INTEGER NOT NULL DEFAULT 0,
            skipped INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS podcast_streams (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            platform TEXT NOT NULL DEFAULT '',
            ms_played INTEGER NOT NULL,
            conn_country TEXT NOT NULL DEFAULT '',
            spotify_episode_uri TEXT,
            episode_name TEXT NOT NULL,
            show_name TEXT NOT NULL DEFAULT '',
            reason_start TEXT NOT NULL DEFAULT '',
            reason_end TEXT NOT NULL DEFAULT '',
            shuffle INTEGER NOT NULL DEFAULT 0,
            skipped INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS audiobook_streams (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            platform TEXT NOT NULL DEFAULT '',
            ms_played INTEGER NOT NULL,
            conn_country TEXT NOT NULL DEFAULT '',
            audiobook_uri TEXT,
            audiobook_title TEXT NOT NULL,
            chapter_uri TEXT,
            chapter_title TEXT NOT NULL DEFAULT '',
            reason_start TEXT NOT NULL DEFAULT '',
            reason_end TEXT NOT NULL DEFAULT '',
            shuffle INTEGER NOT NULL DEFAULT 0,
            skipped INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_music_ts ON music_streams(ts);
        CREATE INDEX IF NOT EXISTS idx_music_ts_month ON music_streams(SUBSTR(ts,1,7));
        CREATE INDEX IF NOT EXISTS idx_music_ts_day ON music_streams(SUBSTR(ts,1,10));
        CREATE INDEX IF NOT EXISTS idx_music_track ON music_streams(track_name);
        CREATE INDEX IF NOT EXISTS idx_music_artist ON music_streams(artist_name);
        CREATE INDEX IF NOT EXISTS idx_podcast_ts ON podcast_streams(ts);
        CREATE INDEX IF NOT EXISTS idx_audiobook_ts ON audiobook_streams(ts);

        CREATE VIEW IF NOT EXISTS monthly_stats AS
        SELECT SUBSTR(ts, 1, 7) AS month,
               COUNT(*) AS streams,
               SUM(ms_played) AS total_ms,
               ROUND(SUM(ms_played) / 3600000.0, 1) AS hours,
               COUNT(DISTINCT track_name || '|||' || artist_name) AS unique_tracks
        FROM music_streams
        GROUP BY month
        ORDER BY month;
        ",
    )
}

pub fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{hours:04}H:{mins:02}M:{secs:02}S")
}
