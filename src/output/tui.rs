use std::collections::{HashMap, HashSet};
use std::io::{self, Stdout};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::model::sensor::{SensorCategory, SensorId, SensorReading, SensorUnit};

/// Run the interactive TUI sensor dashboard.
///
/// Blocks until the user presses 'q' or Esc. Reads sensor data from the
/// shared `state` map on each tick (every `poll_interval_ms` milliseconds).
pub fn run(
    state: Arc<RwLock<HashMap<SensorId, SensorReading>>>,
    poll_interval_ms: u64,
    alert_rules: Vec<crate::sensors::alerts::AlertRule>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &state, poll_interval_ms, alert_rules);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

/// Group key derived from a SensorId: "source/chip"
fn group_key(id: &SensorId) -> String {
    format!("{}/{}", id.source, id.chip)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &Arc<RwLock<HashMap<SensorId, SensorReading>>>,
    poll_interval_ms: u64,
    alert_rules: Vec<crate::sensors::alerts::AlertRule>,
) -> io::Result<()> {
    let start = Instant::now();
    let mut scroll_offset: usize = 0;
    let mut collapsed: HashSet<String> = HashSet::new();
    let mut cursor: usize = 0;
    let mut last_total_rows: usize;
    let mut alert_engine = crate::sensors::alerts::AlertEngine::new(alert_rules);
    let mut active_alerts: Vec<String> = Vec::new();

    // Auto-collapse high-count groups on first render
    let mut auto_collapsed = false;

    loop {
        let elapsed = start.elapsed();

        // Snapshot sensor state and check alerts
        let snapshot = snapshot_sorted(state);
        {
            let readings_map: HashMap<SensorId, SensorReading> = snapshot.iter().cloned().collect();
            let new_alerts = alert_engine.check(&readings_map);
            if !new_alerts.is_empty() {
                active_alerts = new_alerts;
            }
        }

        // On first render, auto-collapse groups with > 32 entries
        if !auto_collapsed && !snapshot.is_empty() {
            auto_collapsed = true;
            let mut group_counts: HashMap<String, usize> = HashMap::new();
            for (id, _) in &snapshot {
                *group_counts.entry(group_key(id)).or_default() += 1;
            }
            for (key, count) in &group_counts {
                if *count > 32 {
                    collapsed.insert(key.clone());
                }
            }
        }

        let (display_rows, group_indices) = build_rows(&snapshot, &collapsed);
        last_total_rows = display_rows.len();

        // Clamp scroll and cursor
        scroll_offset = scroll_offset.min(last_total_rows.saturating_sub(1));
        if !group_indices.is_empty() {
            cursor = cursor.min(group_indices.len() - 1);
        }

        let sensor_count = snapshot.len();
        let max_samples = snapshot
            .iter()
            .map(|(_, r)| r.sample_count)
            .max()
            .unwrap_or(0);

        let elapsed_str = format_elapsed(elapsed);
        let collapsed_count = collapsed.len();

        draw(
            terminal,
            display_rows,
            &group_indices,
            cursor,
            scroll_offset,
            sensor_count,
            max_samples,
            collapsed_count,
            &elapsed_str,
            &active_alerts,
        )?;

        let timeout = Duration::from_millis(poll_interval_ms);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            cursor -= 1;
                            // Auto-scroll to keep cursor visible
                            if let Some(&row_idx) = group_indices.get(cursor) {
                                if row_idx < scroll_offset {
                                    scroll_offset = row_idx;
                                }
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 < group_indices.len() {
                            cursor += 1;
                            // Auto-scroll down
                            if let Some(&row_idx) = group_indices.get(cursor) {
                                let term_height = terminal.size()?.height as usize;
                                let visible = term_height.saturating_sub(6);
                                if row_idx >= scroll_offset + visible {
                                    scroll_offset = row_idx.saturating_sub(visible / 2);
                                }
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        // Toggle collapse on the group at cursor
                        if let Some(key) = get_group_at_cursor(&snapshot, &group_indices, cursor) {
                            if collapsed.contains(&key) {
                                collapsed.remove(&key);
                            } else {
                                collapsed.insert(key);
                            }
                        }
                    }
                    KeyCode::Char('c') => {
                        // Collapse all
                        let groups = unique_groups(&snapshot);
                        for g in groups {
                            collapsed.insert(g);
                        }
                    }
                    KeyCode::Char('e') => {
                        // Expand all
                        collapsed.clear();
                    }
                    KeyCode::PageUp => {
                        scroll_offset = scroll_offset.saturating_sub(20);
                        // Move cursor up to nearest visible group
                        while cursor > 0 {
                            if let Some(&ri) = group_indices.get(cursor) {
                                if ri >= scroll_offset {
                                    break;
                                }
                            }
                            cursor -= 1;
                        }
                    }
                    KeyCode::PageDown => {
                        scroll_offset = scroll_offset.saturating_add(20);
                        // Move cursor down to nearest visible group
                        while cursor + 1 < group_indices.len() {
                            if let Some(&ri) = group_indices.get(cursor) {
                                if ri >= scroll_offset {
                                    break;
                                }
                            }
                            cursor += 1;
                        }
                    }
                    KeyCode::Home => {
                        scroll_offset = 0;
                        cursor = 0;
                    }
                    KeyCode::End => {
                        scroll_offset = last_total_rows.saturating_sub(1);
                        cursor = group_indices.len().saturating_sub(1);
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        scroll_offset = scroll_offset.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        scroll_offset = scroll_offset.saturating_add(3);
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn snapshot_sorted(
    state: &Arc<RwLock<HashMap<SensorId, SensorReading>>>,
) -> Vec<(SensorId, SensorReading)> {
    let map = state.read().unwrap_or_else(|e| e.into_inner());
    let mut entries: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    entries.sort_by(|(a, _), (b, _)| a.natural_cmp(b));
    entries
}

fn unique_groups(snapshot: &[(SensorId, SensorReading)]) -> Vec<String> {
    let mut groups = Vec::new();
    let mut seen = HashSet::new();
    for (id, _) in snapshot {
        let key = group_key(id);
        if seen.insert(key.clone()) {
            groups.push(key);
        }
    }
    groups
}

fn get_group_at_cursor(
    snapshot: &[(SensorId, SensorReading)],
    _group_indices: &[usize],
    cursor: usize,
) -> Option<String> {
    let groups = unique_groups(snapshot);
    groups.get(cursor).cloned()
}

struct GroupSummary {
    count: usize,
    current_min: f64,
    current_max: f64,
    global_min: f64,
    global_max: f64,
    avg: f64,
    unit: String,
    precision: usize,
}

fn compute_group_summaries(
    snapshot: &[(SensorId, SensorReading)],
) -> HashMap<String, GroupSummary> {
    let mut summaries: HashMap<String, GroupSummary> = HashMap::new();

    for (id, reading) in snapshot {
        let key = group_key(id);
        let entry = summaries.entry(key).or_insert_with(|| GroupSummary {
            count: 0,
            current_min: f64::MAX,
            current_max: f64::MIN,
            global_min: f64::MAX,
            global_max: f64::MIN,
            avg: 0.0,
            unit: format!("{}", reading.unit),
            precision: format_precision(&reading.unit),
        });

        entry.count += 1;
        entry.current_min = entry.current_min.min(reading.current);
        entry.current_max = entry.current_max.max(reading.current);
        entry.global_min = entry.global_min.min(reading.min);
        entry.global_max = entry.global_max.max(reading.max);
        // Running mean of averages
        entry.avg += (reading.avg - entry.avg) / entry.count as f64;
    }

    summaries
}

/// Build display rows with collapsible groups.
/// Returns (rows, group_header_row_indices) where group_header_row_indices[i]
/// is the row index of the i-th group header.
fn build_rows(
    snapshot: &[(SensorId, SensorReading)],
    collapsed: &HashSet<String>,
) -> (Vec<Row<'static>>, Vec<usize>) {
    // Pre-compute per-group summary stats
    let group_summaries = compute_group_summaries(snapshot);

    let mut rows = Vec::new();
    let mut group_indices = Vec::new();
    let mut current_group: Option<String> = None;

    for (id, reading) in snapshot {
        let key = group_key(id);

        if current_group.as_ref() != Some(&key) {
            current_group = Some(key.clone());

            let is_collapsed = collapsed.contains(&key);
            let summary = group_summaries.get(&key);
            let count = summary.map(|s| s.count).unwrap_or(0);
            let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
            let header_text = format!(
                " {arrow} {key} ({count} sensor{})",
                if count == 1 { "" } else { "s" }
            );

            group_indices.push(rows.len());

            let header_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            let summary_style = Style::default().fg(Color::DarkGray);

            let header_row = if is_collapsed {
                if let Some(s) = summary {
                    let precision = s.precision;
                    Row::new(vec![
                        Cell::from(header_text).style(header_style),
                        Cell::from(format!(
                            "{:.prec$}\u{2013}{:.prec$}",
                            s.current_min,
                            s.current_max,
                            prec = precision
                        ))
                        .style(summary_style),
                        Cell::from(format!("{:.prec$}", s.global_min, prec = precision))
                            .style(summary_style),
                        Cell::from(format!("{:.prec$}", s.global_max, prec = precision))
                            .style(summary_style),
                        Cell::from(format!("{:.prec$}", s.avg, prec = precision))
                            .style(summary_style),
                        Cell::from(s.unit.clone()).style(summary_style),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(header_text).style(header_style),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                    ])
                }
            } else {
                Row::new(vec![
                    Cell::from(header_text).style(header_style),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
            };
            rows.push(header_row);

            if is_collapsed {
                continue;
            }
        }

        let key = group_key(id);
        if collapsed.contains(&key) {
            continue;
        }

        let precision = format_precision(&reading.unit);
        let current_str = format!("{:.prec$}", reading.current, prec = precision);
        let min_str = format!("{:.prec$}", reading.min, prec = precision);
        let max_str = format!("{:.prec$}", reading.max, prec = precision);
        let avg_str = format!("{:.prec$}", reading.avg, prec = precision);
        let unit_str = format!("{}", reading.unit);

        let style = value_style(reading);

        let row = Row::new(vec![
            Cell::from(format!("  {}", reading.label)),
            Cell::from(current_str).style(style),
            Cell::from(min_str),
            Cell::from(max_str),
            Cell::from(avg_str),
            Cell::from(unit_str),
        ]);
        rows.push(row);
    }

    (rows, group_indices)
}

#[allow(clippy::too_many_arguments)]
fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    rows: Vec<Row<'static>>,
    group_indices: &[usize],
    cursor: usize,
    scroll_offset: usize,
    sensor_count: usize,
    max_samples: u64,
    collapsed_count: usize,
    elapsed_str: &str,
    active_alerts: &[String],
) -> io::Result<()> {
    let total_groups = group_indices.len();

    terminal.draw(|frame| {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(size);

        // Top bar
        let is_root = unsafe { libc::geteuid() } == 0;
        let priv_hint = if is_root {
            ""
        } else {
            " | \u{26a0} run as root for SMART, DMI serials, MSR"
        };
        let title = format!(
            " sinfo \u{2014} Sensor Monitor | {} sensors | {} groups ({} collapsed) | {}{}",
            sensor_count, total_groups, collapsed_count, elapsed_str, priv_hint
        );
        let header_block = Paragraph::new(title)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(header_block, chunks[0]);

        // Highlight the cursor's group header row
        let cursor_row_idx = group_indices.get(cursor).copied();

        // Main table
        let table_header = Row::new(vec![
            Cell::from("Label").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Current").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Min").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Max").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Avg").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Unit").style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .height(1)
        .bottom_margin(1)
        .style(Style::default().bg(Color::DarkGray));

        // Apply cursor highlight and scrolling
        let visible_rows: Vec<Row> = rows
            .into_iter()
            .enumerate()
            .skip(scroll_offset)
            .map(|(idx, row)| {
                if Some(idx) == cursor_row_idx {
                    row.style(Style::default().bg(Color::DarkGray))
                } else {
                    row
                }
            })
            .collect();

        let table = Table::new(
            visible_rows,
            [
                Constraint::Min(28),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(8),
            ],
        )
        .header(table_header)
        .block(Block::default().borders(Borders::NONE));

        frame.render_widget(table, chunks[1]);

        // Bottom bar
        let status = if active_alerts.is_empty() {
            format!(
                " q: quit | \u{2191}\u{2193}: navigate | Enter: toggle | c/e: collapse/expand | Sensors: {} | Samples: {}",
                sensor_count, max_samples
            )
        } else {
            format!(" \u{26a0} {} | {}", active_alerts.join(" | "), {
                format!("Sensors: {} | Samples: {}", sensor_count, max_samples)
            })
        };
        let status_style = if active_alerts.is_empty() {
            Style::default().fg(Color::DarkGray).bg(Color::Black)
        } else {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        };
        let status_bar = Paragraph::new(status).style(status_style);
        frame.render_widget(status_bar, chunks[2]);
    })?;

    Ok(())
}

fn format_precision(unit: &SensorUnit) -> usize {
    match unit {
        SensorUnit::Celsius
        | SensorUnit::Volts
        | SensorUnit::Millivolts
        | SensorUnit::Watts
        | SensorUnit::Milliwatts
        | SensorUnit::Amps
        | SensorUnit::Milliamps => 1,
        SensorUnit::Rpm | SensorUnit::Mhz | SensorUnit::Percent => 0,
        SensorUnit::BytesPerSec
        | SensorUnit::MegabytesPerSec
        | SensorUnit::Bytes
        | SensorUnit::Megabytes => 1,
        SensorUnit::Unitless => 1,
    }
}

fn value_style(reading: &SensorReading) -> Style {
    match reading.category {
        SensorCategory::Temperature => {
            if reading.current > 80.0 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if reading.current >= 60.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        }
        SensorCategory::Fan => {
            if reading.current == 0.0 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            }
        }
        SensorCategory::Power => Style::default().fg(Color::Magenta),
        SensorCategory::Voltage => Style::default().fg(Color::Blue),
        SensorCategory::Frequency => Style::default().fg(Color::Cyan),
        SensorCategory::Utilization => {
            if reading.current > 90.0 {
                Style::default().fg(Color::Red)
            } else if reading.current > 70.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        }
        _ => Style::default(),
    }
}

fn format_elapsed(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}
