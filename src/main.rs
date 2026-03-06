mod commands;
mod db;
mod import;
mod models;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "statify", about = "Spotify Extended Streaming History analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Import Spotify JSON files into SQLite
    Import,
    /// Search tracks by name
    Search {
        /// Text to search in track names
        query: String,
    },
    /// Ranking of most played tracks
    Top {
        /// Minimum play count to appear in the ranking
        #[arg(long, default_value_t = 10)]
        min: u64,
    },
    /// General statistics by year
    Stats,
    /// Bar chart of streams
    Chart {
        /// Filter by year (e.g. 2024)
        #[arg(long)]
        year: Option<String>,
    },
}

fn db_is_empty(conn: &rusqlite::Connection) -> bool {
    conn.query_row("SELECT COUNT(*) FROM music_streams", [], |row| row.get::<_, u64>(0))
        .unwrap_or(0)
        == 0
}

fn main() {
    let cli = Cli::parse();
    let mut conn = db::open_db().expect("Failed to open database");
    db::create_tables(&conn).expect("Failed to create tables");

    if db_is_empty(&conn) && cli.command.is_none() {
        println!("Database is empty, importing data...\n");
        import::cmd_import(&mut conn);
        println!();
    }

    match cli.command {
        None => commands::chart::cmd_chart(&conn, None),
        Some(Commands::Import) => import::cmd_import(&mut conn),
        Some(Commands::Search { ref query }) => commands::search::cmd_search(&conn, query),
        Some(Commands::Top { min }) => commands::top::cmd_top(&conn, min),
        Some(Commands::Stats) => commands::stats::cmd_stats(&conn),
        Some(Commands::Chart { ref year }) => commands::chart::cmd_chart(&conn, year.as_deref()),
    }
}
