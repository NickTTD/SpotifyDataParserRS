use crate::db::format_duration;
use rusqlite::Connection;

pub fn cmd_stats(conn: &Connection) {
    let mut stmt = conn
        .prepare(
            "SELECT
                SUBSTR(ts, 1, 4) as year,
                COUNT(*) as streams,
                SUM(ms_played) as total_ms,
                COUNT(DISTINCT track_name || '|||' || artist_name || '|||' || album_name) as unique_tracks
             FROM music_streams
             GROUP BY year
             ORDER BY year",
        )
        .expect("Failed to prepare stats query");

    let rows: Vec<(String, u64, u64, u64)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("Failed to execute stats query")
        .filter_map(|r| r.ok())
        .collect();

    let total_streams: u64 = rows.iter().map(|(_, s, _, _)| s).sum();
    let total_ms: u64 = rows.iter().map(|(_, _, ms, _)| ms).sum();

    let total_unique: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT track_name || '|||' || artist_name || '|||' || album_name) FROM music_streams",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    println!(
        "  {:<6} {:>10} {:>8} {:>10} {:>16}",
        "Year", "Streams", "%", "Unique", "Time"
    );
    println!("  {}", "-".repeat(54));

    for (year, streams, ms, unique) in &rows {
        let pct = (*streams as f64 / total_streams as f64) * 100.0;
        println!(
            "  {:<6} {:>10} {:>7.1}% {:>10} {:>16}",
            year,
            streams,
            pct,
            unique,
            format_duration(*ms)
        );
    }

    println!("  {}", "-".repeat(54));
    println!(
        "  {:<6} {:>10} {:>7.1}% {:>10} {:>16}",
        "TOTAL",
        total_streams,
        100.0,
        total_unique,
        format_duration(total_ms)
    );
}
