use crate::db::format_duration;
use rusqlite::Connection;

pub fn cmd_top(conn: &Connection, min_plays: u64) {
    let mut stmt = conn
        .prepare(
            "SELECT track_name, artist_name, album_name, COUNT(*) as plays, SUM(ms_played) as total_ms
             FROM music_streams
             GROUP BY track_name, artist_name, album_name
             HAVING plays >= ?1
             ORDER BY plays DESC",
        )
        .expect("Failed to prepare top query");

    let rows: Vec<(String, String, String, u64, u64)> = stmt
        .query_map([min_plays], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("Failed to execute top query")
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No tracks with >= {min_plays} plays.");
        return;
    }

    println!(
        "Tracks with >= {min_plays} plays ({} tracks):\n",
        rows.len()
    );

    for (i, (track, artist, album, count, total_ms)) in rows.iter().enumerate() {
        println!("  #{:<4} {track} - {artist} [{album}]", i + 1);
        println!(
            "        {count} plays | total time: {}\n",
            format_duration(*total_ms)
        );
    }
}
