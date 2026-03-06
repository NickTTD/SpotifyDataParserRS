use std::collections::HashMap;
use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rusqlite::Connection;

const RESET: &str = "\x1b[0m";
const WHITE_BOLD: &str = "\x1b[97;1m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const MAX_BAR_HEIGHT: usize = 18;

const PALETTE: &[&str] = &[
    "\x1b[34m",  // Blue
    "\x1b[91m",  // Bright Red
    "\x1b[32m",  // Green
    "\x1b[93m",  // Bright Yellow
    "\x1b[35m",  // Magenta
    "\x1b[96m",  // Bright Cyan
    "\x1b[33m",  // Yellow
    "\x1b[94m",  // Bright Blue
    "\x1b[31m",  // Red
    "\x1b[92m",  // Bright Green
    "\x1b[95m",  // Bright Magenta
    "\x1b[36m",  // Cyan
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn cmd_chart(conn: &Connection, year: Option<&str>) {
    match year {
        Some(y) => {
            let (labels, values, hours) = fetch_monthly(conn, y);
            if values.is_empty() {
                println!("No data to chart.");
                return;
            }
            render_static_bars(&labels, &values, &hours, &format!("Streams per month ({y})"));
        }
        None => render_timeline(conn),
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

struct MonthEntry {
    month: String,
    year: String,
    streams: u64,
    hours: f64,
    unique_tracks: u64,
}

struct MonthDetail {
    top_track: String,
    top_track_artist: String,
    top_track_plays: u64,
    top_track_hours: f64,
    top_artist: String,
    top_artist_plays: u64,
    top_artist_hours: f64,
}

struct DayEntry {
    day: String,
    day_label: String,
    streams: u64,
    hours: f64,
}

// ---------------------------------------------------------------------------
// Data fetching
// ---------------------------------------------------------------------------

fn fetch_monthly(conn: &Connection, year: &str) -> (Vec<String>, Vec<u64>, Vec<f64>) {
    let mut stmt = conn
        .prepare("SELECT month, streams, hours FROM monthly_stats WHERE month LIKE ?1")
        .unwrap();

    let rows: Vec<(String, u64, f64)> = stmt
        .query_map([format!("{year}%")], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let labels: Vec<String> = rows
        .iter()
        .map(|(m, _, _)| {
            match &m[5..] {
                "01" => "Jan", "02" => "Feb", "03" => "Mar", "04" => "Apr",
                "05" => "May", "06" => "Jun", "07" => "Jul", "08" => "Aug",
                "09" => "Sep", "10" => "Oct", "11" => "Nov", "12" => "Dec",
                other => other,
            }
            .to_string()
        })
        .collect();
    let values = rows.iter().map(|(_, s, _)| *s).collect();
    let hours = rows.iter().map(|(_, _, h)| *h).collect();
    (labels, values, hours)
}

fn fetch_month_entries(conn: &Connection) -> Vec<MonthEntry> {
    let mut stmt = conn
        .prepare("SELECT month, streams, hours, unique_tracks FROM monthly_stats")
        .unwrap();

    stmt.query_map([], |row| {
        let month: String = row.get(0)?;
        let year = month[..4].to_string();
        Ok(MonthEntry {
            month,
            year,
            streams: row.get(1)?,
            hours: row.get(2)?,
            unique_tracks: row.get(3)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

fn fetch_month_details(conn: &Connection) -> HashMap<String, MonthDetail> {
    let top_tracks = query_top_per_partition(
        conn,
        "SUBSTR(ts,1,7)",
        "track_name, artist_name",
        "SUBSTR(ts,1,7), track_name, artist_name",
    );

    let top_artists = query_top_artist_per_partition(conn, "SUBSTR(ts,1,7)", "SUBSTR(ts,1,7)");

    merge_details(&top_tracks, &top_artists)
}

fn fetch_all_daily(conn: &Connection) -> HashMap<String, Vec<DayEntry>> {
    let mut stmt = conn
        .prepare(
            "SELECT SUBSTR(ts,1,10) as day, COUNT(*), ROUND(SUM(ms_played)/3600000.0, 1)
             FROM music_streams GROUP BY day ORDER BY day",
        )
        .unwrap();

    let mut by_month: HashMap<String, Vec<DayEntry>> = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            let day: String = row.get(0)?;
            Ok((day, row.get::<_, u64>(1)?, row.get::<_, f64>(2)?))
        })
        .unwrap();

    for r in rows.flatten() {
        let month = r.0[..7].to_string();
        let raw = &r.0[8..];
        let day_label = if let Some(stripped) = raw.strip_prefix('0') {
            format!(" {stripped}")
        } else {
            raw.to_string()
        };
        by_month.entry(month).or_default().push(DayEntry {
            day: r.0,
            day_label,
            streams: r.1,
            hours: r.2,
        });
    }
    by_month
}

fn fetch_all_day_details(conn: &Connection) -> HashMap<String, HashMap<String, MonthDetail>> {
    let top_tracks = query_top_per_partition(
        conn,
        "SUBSTR(ts,1,10)",
        "track_name, artist_name",
        "SUBSTR(ts,1,10), track_name, artist_name",
    );

    let top_artists = query_top_artist_per_partition(conn, "SUBSTR(ts,1,10)", "SUBSTR(ts,1,10)");

    let flat = merge_details(&top_tracks, &top_artists);

    let mut by_month: HashMap<String, HashMap<String, MonthDetail>> = HashMap::new();
    for (key, detail) in flat {
        let month = key[..7].to_string();
        by_month.entry(month).or_default().insert(key, detail);
    }
    by_month
}

// Shared helpers for top-track and top-artist queries using window functions.
// Returns HashMap<partition_key, (track, artist, plays, hours)>.
fn query_top_per_partition(
    conn: &Connection,
    partition_expr: &str,
    select_fields: &str,
    group_fields: &str,
) -> HashMap<String, (String, String, u64, f64)> {
    let sql = format!(
        "WITH ranked AS (
            SELECT {partition_expr} as pk, {select_fields},
                   COUNT(*) as plays,
                   ROUND(SUM(ms_played)/3600000.0, 1) as hours,
                   ROW_NUMBER() OVER (PARTITION BY {partition_expr} ORDER BY COUNT(*) DESC) as rn
            FROM music_streams GROUP BY {group_fields}
        )
        SELECT pk, track_name, artist_name, plays, hours FROM ranked WHERE rn = 1"
    );

    let mut stmt = conn.prepare(&sql).unwrap();
    let mut map = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .unwrap();
    for r in rows.flatten() {
        map.insert(r.0, (r.1, r.2, r.3, r.4));
    }
    map
}

fn query_top_artist_per_partition(
    conn: &Connection,
    partition_expr: &str,
    group_suffix: &str,
) -> HashMap<String, (String, u64, f64)> {
    let sql = format!(
        "WITH ranked AS (
            SELECT {partition_expr} as pk, artist_name,
                   COUNT(*) as plays,
                   ROUND(SUM(ms_played)/3600000.0, 1) as hours,
                   ROW_NUMBER() OVER (PARTITION BY {partition_expr} ORDER BY COUNT(*) DESC) as rn
            FROM music_streams GROUP BY {group_suffix}, artist_name
        )
        SELECT pk, artist_name, plays, hours FROM ranked WHERE rn = 1"
    );

    let mut stmt = conn.prepare(&sql).unwrap();
    let mut map = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .unwrap();
    for r in rows.flatten() {
        map.insert(r.0, (r.1, r.2, r.3));
    }
    map
}

fn merge_details(
    top_tracks: &HashMap<String, (String, String, u64, f64)>,
    top_artists: &HashMap<String, (String, u64, f64)>,
) -> HashMap<String, MonthDetail> {
    let mut details = HashMap::new();
    for (key, (track, artist, plays, hours)) in top_tracks {
        let (art_name, art_plays, art_hours) = top_artists
            .get(key)
            .cloned()
            .unwrap_or_else(|| ("?".into(), 0, 0.0));
        details.insert(
            key.clone(),
            MonthDetail {
                top_track: track.clone(),
                top_track_artist: artist.clone(),
                top_track_plays: *plays,
                top_track_hours: *hours,
                top_artist: art_name,
                top_artist_plays: art_plays,
                top_artist_hours: art_hours,
            },
        );
    }
    details
}

fn days_in_month(month: &str) -> u32 {
    let y: u32 = month[..4].parse().unwrap_or(2000);
    let m: u32 = month[5..7].parse().unwrap_or(1);
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) => 29,
        2 => 28,
        _ => 30,
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Insert "clear to end of line" before every `\r\n` so overwriting
/// a longer previous line doesn't leave trailing artifacts.
fn erase_line_remainders(buf: &mut String) {
    // \x1b[K = EL (Erase in Line) from cursor to end
    *buf = buf.replace("\r\n", "\x1b[K\r\n");
}

fn compute_heights(values: &[u64], max_val: u64) -> Vec<usize> {
    values
        .iter()
        .map(|v| ((*v as f64 / max_val as f64) * MAX_BAR_HEIGHT as f64).round() as usize)
        .collect()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

fn format_detail_lines(buf: &mut String, detail: &MonthDetail, color: &str) {
    let track_name = truncate(&detail.top_track, 40);
    buf.push_str(&format!(
        "    {color}♫{RESET} {BOLD}{track_name}{RESET} {DIM}·{RESET} {} ({} plays, {:.1}h)\r\n",
        detail.top_track_artist, detail.top_track_plays, detail.top_track_hours
    ));
    buf.push_str(&format!(
        "    {color}★{RESET} {} ({} plays, {:.1}h)\r\n",
        detail.top_artist, detail.top_artist_plays, detail.top_artist_hours
    ));
}

// ---------------------------------------------------------------------------
// Static bar chart (for --year)
// ---------------------------------------------------------------------------

fn render_static_bars(labels: &[String], values: &[u64], hours: &[f64], title: &str) {
    let max_val = *values.iter().max().unwrap_or(&1);
    let label_w = labels.iter().map(|l| l.len()).max().unwrap_or(3);
    let col_w = label_w.max(3) + 1;
    let bar_w = col_w - 1;
    let heights = compute_heights(values, max_val);

    println!("  {title}\n");

    for row in (1..=MAX_BAR_HEIGHT).rev() {
        let y_val = (row as f64 / MAX_BAR_HEIGHT as f64 * max_val as f64) as u64;
        if row == MAX_BAR_HEIGHT || row == MAX_BAR_HEIGHT * 2 / 3 || row == MAX_BAR_HEIGHT / 3 {
            print!("  {:>6} ┤ ", y_val);
        } else {
            print!("         │ ");
        }
        for h in &heights {
            if *h >= row {
                print!("{}", "█".repeat(bar_w));
            } else {
                print!("{}", " ".repeat(bar_w));
            }
            print!(" ");
        }
        println!();
    }

    print!("       0 ┼─");
    for _ in values {
        print!("{}─", "─".repeat(bar_w));
    }
    println!();

    print!("           ");
    for label in labels {
        print!("{:^width$} ", label, width = bar_w);
    }
    println!();

    print!("           ");
    for v in values {
        print!("{:^width$} ", v, width = bar_w);
    }
    println!();

    let total_streams: u64 = values.iter().sum();
    let total_hours: f64 = hours.iter().sum();
    println!("\n  Total: {} streams, {:.0}h", total_streams, total_hours);
}

// ---------------------------------------------------------------------------
// Interactive monthly timeline
// ---------------------------------------------------------------------------

fn render_timeline(conn: &Connection) {
    let entries = fetch_month_entries(conn);
    if entries.is_empty() {
        println!("No data to chart.");
        return;
    }

    let details = fetch_month_details(conn);
    let daily_data = fetch_all_daily(conn);
    let daily_details = fetch_all_day_details(conn);

    let max_val = entries.iter().map(|e| e.streams).max().unwrap_or(1);
    let heights = compute_heights(
        &entries.iter().map(|e| e.streams).collect::<Vec<_>>(),
        max_val,
    );

    // Unique years in order
    let mut unique_years: Vec<String> = vec![];
    for entry in &entries {
        if unique_years.last().map(|s| s.as_str()) != Some(&entry.year) {
            unique_years.push(entry.year.clone());
        }
    }

    let year_color = |year: &str| -> &'static str {
        let idx = unique_years.iter().position(|y| y == year).unwrap_or(0);
        PALETTE[idx % PALETTE.len()]
    };

    // Column position where each year starts
    let mut year_starts: Vec<(usize, String)> = vec![];
    let mut prev_year = "";
    for (i, entry) in entries.iter().enumerate() {
        if entry.year != prev_year {
            year_starts.push((i, entry.year.clone()));
            prev_year = &entry.year;
        }
    }
    let total_cols = entries.len();

    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen).unwrap();
    stdout.execute(Hide).unwrap();
    terminal::enable_raw_mode().unwrap();

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = io::stdout().execute(Show);
        let _ = io::stdout().execute(LeaveAlternateScreen);
        orig_hook(info);
    }));

    let mut cursor_pos: usize = 0;
    let total_streams: u64 = entries.iter().map(|e| e.streams).sum();
    let total_hours: f64 = entries.iter().map(|e| e.hours).sum();

    loop {
        // Move home + build frame; clear trailing content at the end
        let mut buf = String::from("\x1b[H");
        buf.push_str("  Streams per month (timeline)\r\n\r\n");

        // Bars
        for row in (1..=MAX_BAR_HEIGHT).rev() {
            let y_val = (row as f64 / MAX_BAR_HEIGHT as f64 * max_val as f64) as u64;
            if row == MAX_BAR_HEIGHT || row == MAX_BAR_HEIGHT * 2 / 3 || row == MAX_BAR_HEIGHT / 3 {
                buf.push_str(&format!("  {:>5} ┤ ", y_val));
            } else {
                buf.push_str("        │ ");
            }

            for (i, entry) in entries.iter().enumerate() {
                if heights[i] >= row {
                    if i == cursor_pos {
                        buf.push_str(&format!("{WHITE_BOLD}█{RESET}"));
                    } else {
                        buf.push_str(&format!("{}█{RESET}", year_color(&entry.year)));
                    }
                } else {
                    buf.push(' ');
                }
            }
            buf.push_str("\r\n");
        }

        // X axis
        buf.push_str("      0 ┼─");
        buf.push_str(&"─".repeat(total_cols));
        buf.push_str("\r\n");

        // Cursor indicator
        buf.push_str("          ");
        for (i, entry) in entries.iter().enumerate() {
            if i == cursor_pos {
                buf.push_str(&format!("{}▲{RESET}", year_color(&entry.year)));
            } else {
                buf.push(' ');
            }
        }
        buf.push_str("\r\n");

        // Year labels
        let margin = 10;
        let mut label_parts: Vec<(usize, String, &str)> = vec![];
        for (idx, (col_pos, year_str)) in year_starts.iter().enumerate() {
            let next_col = year_starts
                .get(idx + 1)
                .map(|(c, _)| *c)
                .unwrap_or(total_cols + margin);
            let available = next_col - col_pos;
            let label = if available < 5 {
                format!("'{}", &year_str[2..])
            } else {
                year_str.to_string()
            };
            label_parts.push((*col_pos, label, year_color(year_str)));
        }

        buf.push_str("          ");
        let mut cur = 0;
        for (pos, label, color) in &label_parts {
            if *pos > cur {
                buf.push_str(&" ".repeat(pos - cur));
            }
            buf.push_str(&format!("{color}{label}{RESET}"));
            cur = pos + label.len();
        }
        buf.push_str("\r\n");

        // Legend
        buf.push_str("          ");
        for year in &unique_years {
            buf.push_str(&format!("{}█{RESET} {} ", year_color(year), year));
        }
        buf.push_str("\r\n");

        // Totals
        buf.push_str(&format!(
            "\r\n  Total: {} streams, {:.0}h\r\n",
            total_streams, total_hours
        ));

        // Selected month info
        let sel = &entries[cursor_pos];
        let sel_color = year_color(&sel.year);
        let daily_avg = sel.streams as f64 / days_in_month(&sel.month) as f64;

        let delta = if cursor_pos > 0 {
            let prev = entries[cursor_pos - 1].streams as f64;
            if prev > 0.0 {
                let pct = ((sel.streams as f64 - prev) / prev) * 100.0;
                if pct >= 0.0 {
                    format!("{GREEN}+{pct:.0}%{RESET}")
                } else {
                    format!("{RED}{pct:.0}%{RESET}")
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let delta_sep = if delta.is_empty() { "" } else { " │ " };

        buf.push_str(&format!(
            "\r\n  {sel_color}▶{RESET} {BOLD}{}{RESET} │ {} streams │ {:.1}h │ {:.1}/day{delta_sep}{delta}\r\n",
            sel.month, sel.streams, sel.hours, daily_avg
        ));

        if let Some(d) = details.get(&sel.month) {
            format_detail_lines(&mut buf, d, sel_color);
        }
        buf.push_str(&format!(
            "    {sel_color}◇{RESET} {} unique tracks\r\n",
            sel.unique_tracks
        ));

        // Help
        buf.push_str(&format!(
            "\r\n  {DIM}←/→ h/l: navigate  ↓/j/Enter: daily detail  0/$: start/end  q: quit{RESET}\r\n"
        ));

        buf.push_str("\x1b[J");
        erase_line_remainders(&mut buf);

        write!(stdout, "{buf}").unwrap();
        stdout.flush().unwrap();

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left | KeyCode::Char('h') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if cursor_pos + 1 < entries.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Home | KeyCode::Char('0') => cursor_pos = 0,
                KeyCode::End | KeyCode::Char('$') => cursor_pos = entries.len() - 1,
                KeyCode::Down | KeyCode::Enter | KeyCode::Char('j') => {
                    let (quit, new_month) = render_daily_view(
                        &mut stdout,
                        &entries,
                        cursor_pos,
                        &year_color,
                        &daily_data,
                        &daily_details,
                    );
                    cursor_pos = new_month;
                    if quit {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    let _ = terminal::disable_raw_mode();
    let _ = stdout.execute(Show);
    let _ = stdout.execute(LeaveAlternateScreen);
    let _ = std::panic::take_hook();
}

// ---------------------------------------------------------------------------
// Interactive daily drill-down
// ---------------------------------------------------------------------------

/// Returns `(quit, updated_month_idx)`.
fn render_daily_view(
    stdout: &mut io::Stdout,
    all_months: &[MonthEntry],
    month_idx: usize,
    year_color: &dyn Fn(&str) -> &'static str,
    daily_data: &HashMap<String, Vec<DayEntry>>,
    daily_details: &HashMap<String, HashMap<String, MonthDetail>>,
) -> (bool, usize) {
    let mut month_idx = month_idx;
    let empty_days: Vec<DayEntry> = vec![];
    let empty_details: HashMap<String, MonthDetail> = HashMap::new();

    if daily_data
        .get(&all_months[month_idx].month)
        .unwrap_or(&empty_days)
        .is_empty()
    {
        return (false, month_idx);
    }

    const COL_W: usize = 2;
    let mut cursor_pos: usize = 0;

    loop {
        let month_key = &all_months[month_idx].month;
        let entries = daily_data.get(month_key).unwrap_or(&empty_days);
        let details = daily_details.get(month_key).unwrap_or(&empty_details);
        let color = year_color(&all_months[month_idx].year);
        let max_val = entries.iter().map(|e| e.streams).max().unwrap_or(1);
        let heights = compute_heights(
            &entries.iter().map(|e| e.streams).collect::<Vec<_>>(),
            max_val,
        );
        let total_cols = entries.len();
        let total_streams: u64 = entries.iter().map(|e| e.streams).sum();
        let total_hours: f64 = entries.iter().map(|e| e.hours).sum();

        let mut buf = String::from("\x1b[H");
        buf.push_str(&format!(
            "  {color}{month_key}{RESET} · Streams per day\r\n\r\n"
        ));

        // Bars
        for row in (1..=MAX_BAR_HEIGHT).rev() {
            let y_val = (row as f64 / MAX_BAR_HEIGHT as f64 * max_val as f64) as u64;
            if row == MAX_BAR_HEIGHT
                || row == MAX_BAR_HEIGHT * 2 / 3
                || row == MAX_BAR_HEIGHT / 3
            {
                buf.push_str(&format!("  {:>5} ┤ ", y_val));
            } else {
                buf.push_str("        │ ");
            }

            for (i, _) in entries.iter().enumerate() {
                if heights[i] >= row {
                    if i == cursor_pos {
                        buf.push_str(&format!(
                            "{WHITE_BOLD}{}{RESET}",
                            "█".repeat(COL_W)
                        ));
                    } else {
                        buf.push_str(&format!("{color}{}{RESET}", "█".repeat(COL_W)));
                    }
                } else {
                    buf.push_str(&" ".repeat(COL_W));
                }
            }
            buf.push_str("\r\n");
        }

        // X axis
        buf.push_str("      0 ┼─");
        buf.push_str(&"─".repeat(total_cols * COL_W));
        buf.push_str("\r\n");

        // Cursor indicator
        buf.push_str("          ");
        for i in 0..total_cols {
            if i == cursor_pos {
                buf.push_str(&format!("{color}▲{RESET} "));
            } else {
                buf.push_str("  ");
            }
        }
        buf.push_str("\r\n");

        // Day labels
        buf.push_str("          ");
        for (i, entry) in entries.iter().enumerate() {
            if i == cursor_pos {
                buf.push_str(&format!("{WHITE_BOLD}{}{RESET}", entry.day_label));
            } else {
                buf.push_str(&format!("{color}{}{RESET}", entry.day_label));
            }
        }
        buf.push_str("\r\n");

        // Month navigation hints
        buf.push_str("          ");
        let left_len = if month_idx > 0 {
            let label = format!("◀ {}", all_months[month_idx - 1].month);
            let len = label.chars().count();
            buf.push_str(&format!("{color}{label}{RESET}"));
            len
        } else {
            0
        };
        if month_idx + 1 < all_months.len() {
            let next_label = format!("{} ▶", all_months[month_idx + 1].month);
            let chart_w = total_cols * COL_W;
            let right_len = next_label.chars().count();
            if chart_w > left_len + right_len {
                buf.push_str(&" ".repeat(chart_w - left_len - right_len));
            }
            buf.push_str(&format!("{color}{next_label}{RESET}"));
        }
        buf.push_str("\r\n");

        // Totals
        buf.push_str(&format!(
            "\r\n  Total: {} streams, {:.1}h\r\n",
            total_streams, total_hours
        ));

        // Selected day info
        let sel = &entries[cursor_pos];
        buf.push_str(&format!(
            "\r\n  {color}▶{RESET} {BOLD}{}{RESET} │ {} streams │ {:.1}h\r\n",
            sel.day, sel.streams, sel.hours
        ));

        if let Some(d) = details.get(&sel.day) {
            format_detail_lines(&mut buf, d, color);
        }

        // Help
        buf.push_str(&format!(
            "\r\n  {DIM}←/→ h/l: navigate (crosses months)  ↑/k: back  0/$: start/end  q: quit{RESET}\r\n"
        ));

        buf.push_str("\x1b[J");
        erase_line_remainders(&mut buf);

        write!(stdout, "{buf}").unwrap();
        stdout.flush().unwrap();

        let entry_count = entries.len();

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return (true, month_idx),
                KeyCode::Up | KeyCode::Char('k') => return (false, month_idx),
                KeyCode::Left | KeyCode::Char('h') => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                    } else if month_idx > 0 {
                        month_idx -= 1;
                        let new_len = daily_data
                            .get(&all_months[month_idx].month)
                            .map(|d| d.len())
                            .unwrap_or(0);
                        if new_len == 0 {
                            month_idx += 1;
                        } else {
                            cursor_pos = new_len - 1;
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if cursor_pos + 1 < entry_count {
                        cursor_pos += 1;
                    } else if month_idx + 1 < all_months.len() {
                        month_idx += 1;
                        let new_len = daily_data
                            .get(&all_months[month_idx].month)
                            .map(|d| d.len())
                            .unwrap_or(0);
                        if new_len == 0 {
                            month_idx -= 1;
                        } else {
                            cursor_pos = 0;
                        }
                    }
                }
                KeyCode::Home | KeyCode::Char('0') => cursor_pos = 0,
                KeyCode::End | KeyCode::Char('$') => cursor_pos = entry_count - 1,
                _ => {}
            }
        }
    }
}
