use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct StreamEntry {
    ts: String,
    ms_played: u64,
    master_metadata_track_name: Option<String>,
    master_metadata_album_artist_name: Option<String>,
    master_metadata_album_album_name: Option<String>,
}

type TrackKey = (String, String, String);
type TrackStats = (u64, u64); // (count, total_ms)

impl StreamEntry {
    fn key(&self) -> Option<TrackKey> {
        let track = self.master_metadata_track_name.clone()?;
        if track.is_empty() {
            return None;
        }
        Some((
            track,
            self.master_metadata_album_artist_name.clone().unwrap_or_default(),
            self.master_metadata_album_album_name.clone().unwrap_or_default(),
        ))
    }

    fn year(&self) -> &str {
        self.ts.get(..4).unwrap_or("????")
    }
}

fn find_json_files() -> Vec<PathBuf> {
    let dir = Path::new("Spotify Extended Streaming History");
    fs::read_dir(dir)
        .expect("No se pudo leer el directorio 'Spotify Extended Streaming History'")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            if name.starts_with("Streaming_History_Audio_") && name.ends_with(".json") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn parse_file(path: &Path) -> Vec<StreamEntry> {
    let data = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error leyendo {}: {e}", path.display());
        String::new()
    });
    serde_json::from_str(&data).unwrap_or_else(|e| {
        eprintln!("Error parseando {}: {e}", path.display());
        Vec::new()
    })
}

fn load_all(files: &[PathBuf]) -> Vec<StreamEntry> {
    files.par_iter().flat_map(|p| parse_file(p)).collect()
}

fn group_entries(entries: &[StreamEntry]) -> Vec<(TrackKey, TrackStats)> {
    let mut grouped: HashMap<TrackKey, TrackStats> = HashMap::new();
    for entry in entries {
        if let Some(key) = entry.key() {
            let stats = grouped.entry(key).or_default();
            stats.0 += 1;
            stats.1 += entry.ms_played;
        }
    }
    let mut sorted: Vec<_> = grouped.into_iter().collect();
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    sorted
}

fn count_unique(entries: &[&StreamEntry]) -> usize {
    entries
        .iter()
        .filter_map(|e| e.key())
        .collect::<HashSet<_>>()
        .len()
}

fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{hours:04}H:{mins:02}M:{secs:02}S")
}

fn print_grouped(entries: &[(TrackKey, TrackStats)]) {
    for (i, ((track, artist, album), (count, total_ms))) in entries.iter().enumerate() {
        println!("  #{:<4} {track} - {artist} [{album}]", i + 1);
        println!(
            "        {count} reproducciones | tiempo total: {}\n",
            format_duration(*total_ms)
        );
    }
}

// --- Comandos ---

fn cmd_search(query: &str, files: &[PathBuf]) {
    println!("Buscando \"{query}\" en {} archivos...\n", files.len());

    let query_lower = query.to_lowercase();
    let results: Vec<StreamEntry> = files
        .par_iter()
        .flat_map(|path| {
            parse_file(path)
                .into_iter()
                .filter(|e| {
                    e.master_metadata_track_name
                        .as_ref()
                        .is_some_and(|name| name.to_lowercase().contains(&query_lower))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    if results.is_empty() {
        println!("No se encontraron resultados para \"{query}\".");
        return;
    }

    let grouped = group_entries(&results);
    println!("Se encontraron {} reproducciones:\n", results.len());
    print_grouped(&grouped);
}

fn cmd_top(min_plays: u64, files: &[PathBuf]) {
    println!(
        "Ranking de canciones con >= {min_plays} reproducciones ({} archivos)...\n",
        files.len()
    );

    let all = load_all(files);
    let grouped = group_entries(&all);
    let filtered: Vec<_> = grouped
        .into_iter()
        .filter(|(_, (count, _))| *count >= min_plays)
        .collect();

    if filtered.is_empty() {
        println!("No hay canciones con >= {min_plays} reproducciones.");
        return;
    }

    println!("{} canciones encontradas:\n", filtered.len());
    print_grouped(&filtered);
}

fn cmd_stats(files: &[PathBuf]) {
    println!("Cargando {} archivos...\n", files.len());

    let all = load_all(files);
    let total_streams = all.len() as u64;
    let total_ms: u64 = all.iter().map(|e| e.ms_played).sum();
    let unique_tracks = group_entries(&all).len();

    let mut by_year: HashMap<&str, (u64, u64, Vec<&StreamEntry>)> = HashMap::new();
    for entry in &all {
        let stats = by_year.entry(entry.year()).or_default();
        stats.0 += 1;
        stats.1 += entry.ms_played;
        stats.2.push(entry);
    }

    let mut years: Vec<_> = by_year.into_iter().collect();
    years.sort_by_key(|(y, _)| *y);

    println!(
        "  {:<6} {:>10} {:>8} {:>10} {:>16}",
        "Año", "Streams", "%", "Únicas", "Tiempo"
    );
    println!("  {}", "-".repeat(54));
    for (year, (count, ms, entries)) in &years {
        let pct = (*count as f64 / total_streams as f64) * 100.0;
        println!(
            "  {:<6} {:>10} {:>7.1}% {:>10} {:>16}",
            year,
            count,
            pct,
            count_unique(entries),
            format_duration(*ms)
        );
    }
    println!("  {}", "-".repeat(54));
    println!(
        "  {:<6} {:>10} {:>7.1}% {:>10} {:>16}",
        "TOTAL",
        total_streams,
        100.0,
        unique_tracks,
        format_duration(total_ms)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Uso:");
        eprintln!("  cargo run -- \"nombre de canción\"    Buscar canción");
        eprintln!("  cargo run -- --top N                 Ranking (mínimo N reproducciones)");
        eprintln!("  cargo run -- --stats                 Totales generales");
        std::process::exit(1);
    }

    let files = find_json_files();

    if args[0] == "--stats" {
        cmd_stats(&files);
    } else if args[0] == "--top" {
        let min_plays: u64 = args
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                eprintln!("Uso: cargo run -- --top N");
                std::process::exit(1);
            });
        cmd_top(min_plays, &files);
    } else {
        cmd_search(&args[0], &files);
    }
}
