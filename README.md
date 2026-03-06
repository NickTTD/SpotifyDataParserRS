# Statify CLI

Rust CLI tool that imports your Spotify Extended Streaming History into SQLite and provides interactive charts, search, and statistics.

## Setup

1. Request your data from [Spotify Privacy Settings](https://www.spotify.com/account/privacy/) (Extended Streaming History)
2. Copy the `Spotify Extended Streaming History/` folder (with `Streaming_History_Audio_*.json` files) to the project root
3. Build and import:

```bash
cargo build --release
./target/release/spotify-stats import
```

## Usage

Running with no arguments launches the **interactive timeline** — a navigable monthly chart of your entire listening history:

```bash
cargo run --release
```

- Navigate months with `←/→` or `h/l`
- Press `↓/j/Enter` on any month to drill down into a **daily view**
- In the daily view, moving past the first/last day crosses into adjacent months
- Press `↑/k` to go back, `q` to quit

### Search tracks

```bash
cargo run --release -- search "Ride"
```

### Top tracks

Shows all tracks with at least N plays:

```bash
cargo run --release -- top --min 150
```

### Yearly statistics

```bash
cargo run --release -- stats
```

### Static chart by year

```bash
cargo run --release -- chart --year 2024
```

## Dependencies

- **rusqlite** — SQLite storage (bundled)
- **clap** — CLI argument parsing
- **crossterm** — interactive terminal UI
- **serde** + **serde_json** — JSON deserialization
- **rayon** — parallel file processing
