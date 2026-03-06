use crate::db::format_duration;
use rusqlite::Connection;

pub fn cmd_search(conn: &Connection, query: &str) {
    let pattern = format!("%{query}%");

    let mut stmt = conn
        .prepare(
            "SELECT track_name, artist_name, album_name, COUNT(*) as plays, SUM(ms_played) as total_ms
             FROM music_streams
             WHERE track_name LIKE ?1
             GROUP BY track_name, artist_name, album_name
             ORDER BY plays DESC",
        )
        .expect("Failed to prepare search query");

    let rows: Vec<(String, String, String, u64, u64)> = stmt
        .query_map([&pattern], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("Failed to execute search query")
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No results found for \"{query}\".");
        return;
    }

    println!(
        "Found {} tracks matching \"{query}\":\n",
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
