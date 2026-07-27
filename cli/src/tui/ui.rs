use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::bam_boost_handler::format_jitosol;
use crate::batch_claim::ClaimState;
use crate::scanner::{EpochStatus, Stats};
use crate::tui::app::{App, Screen, SetupField, PAGE_SIZE};

/// Renders the current screen of the TUI. Pure rendering: no state mutation.
pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Setup => draw_setup(frame, app),
        Screen::Dashboard => draw_dashboard(frame, app, frame.area()),
        Screen::Confirm => {
            draw_dashboard(frame, app, frame.area());
            draw_confirm_popup(frame, app);
        }
        Screen::Progress => draw_progress(frame, app),
    }
}

fn setup_field_title(base: &str, field: SetupField, app: &App) -> String {
    if app.setup_focus == field {
        format!("{base} ◀")
    } else {
        base.to_string()
    }
}

fn setup_field_style(field: SetupField, app: &App) -> Style {
    if app.setup_focus == field {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn draw_setup(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let has_error = app.setup_error.is_some();
    let mut constraints = vec![
        Constraint::Length(3), // Network
        Constraint::Length(3), // RPC URL
        Constraint::Length(3), // Claimant
        Constraint::Length(3), // Start
    ];
    if has_error {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(1)); // spacer
    constraints.push(Constraint::Length(1)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let network_title = setup_field_title("Network", SetupField::Network, app);
    let network = Paragraph::new(app.network.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(network_title)
            .border_style(setup_field_style(SetupField::Network, app)),
    );
    frame.render_widget(network, chunks[0]);

    let rpc_title = setup_field_title("RPC URL", SetupField::RpcUrl, app);
    let rpc = Paragraph::new(app.rpc_url.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(rpc_title)
            .border_style(setup_field_style(SetupField::RpcUrl, app)),
    );
    frame.render_widget(rpc, chunks[1]);

    let claimant_title = setup_field_title("Claimant", SetupField::Claimant, app);
    let claimant = Paragraph::new(app.claimant_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(claimant_title)
            .border_style(setup_field_style(SetupField::Claimant, app)),
    );
    frame.render_widget(claimant, chunks[2]);

    let start_title = setup_field_title("[Start scan]", SetupField::Start, app);
    let start = Paragraph::new("").block(
        Block::default()
            .borders(Borders::ALL)
            .title(start_title)
            .border_style(setup_field_style(SetupField::Start, app)),
    );
    frame.render_widget(start, chunks[3]);

    let mut idx = 4;
    if let Some(err) = &app.setup_error {
        let error_para =
            Paragraph::new(err.as_str()).style(Style::default().fg(ratatui::style::Color::Red));
        frame.render_widget(error_para, chunks[idx]);
        idx += 1;
    }
    idx += 1; // skip spacer

    let footer = Paragraph::new("Tab: next · Enter on [Start scan]: scan · Esc: quit");
    frame.render_widget(footer, chunks[idx]);
}

fn status_str(status: &EpochStatus) -> &'static str {
    if status.amount.is_none() {
        "not eligible"
    } else if status.claimed {
        "claimed"
    } else if status.expired {
        "expired"
    } else {
        "unclaimed"
    }
}

fn sel_str(app: &App, epoch: u64) -> &'static str {
    if app.selected.contains(&epoch) {
        "[x]"
    } else {
        "[ ]"
    }
}

/// Height of the bordered "Unclaimed" panel: 2 border rows plus one row per
/// claimable epoch, capped at 8 so a long claimable list scrolls instead of
/// crowding out the paginated history panel below it. Floored at 3 (the
/// minimum a bordered block needs to show one line of content) so the
/// "Nothing to claim" message is always visible when the set is empty.
fn unclaimed_panel_height(claimable_count: usize) -> u16 {
    (claimable_count as u16 + 2).clamp(3, 8)
}

fn draw_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let header = Paragraph::new(format!(
        "Claimant: {}    Network: {}",
        app.claimant_input, app.network
    ))
    .block(Block::default().borders(Borders::ALL).title("Dashboard"));
    frame.render_widget(header, outer[0]);

    if app.scanning {
        let rest = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(outer[1]);
        let scanning = Paragraph::new("Scanning...");
        frame.render_widget(scanning, rest[0]);
        draw_dashboard_footer(frame, app, rest[1]);
        return;
    }

    let claimable = app.claimable_sorted();
    let rest = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(unclaimed_panel_height(claimable.len())),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(outer[1]);

    draw_unclaimed_panel(frame, app, &claimable, rest[0]);
    draw_all_epochs_panel(frame, app, rest[1]);
    draw_dashboard_footer(frame, app, rest[2]);
}

/// Bordered panel listing only the claimable epochs (descending), with a
/// `[x]`/`[ ]` selection marker and a reversed highlight on `app.cursor`.
fn draw_unclaimed_panel(frame: &mut Frame, app: &App, claimable: &[u64], area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Unclaimed");

    if claimable.is_empty() {
        frame.render_widget(Paragraph::new("Nothing to claim").block(block), area);
        return;
    }

    let rows: Vec<Row> = claimable
        .iter()
        .map(|&epoch| {
            let amount = app
                .statuses
                .iter()
                .find(|s| s.epoch == epoch)
                .and_then(|s| s.amount)
                .unwrap_or(0);
            Row::new(vec![
                sel_str(app, epoch).to_string(),
                epoch.to_string(),
                format_jitosol(amount),
            ])
        })
        .collect();

    // No column header row here: the panel height budget (`unclaimed_panel_height`)
    // is exactly 2 border rows + 1 row per claimable epoch, so a header row
    // would silently eat one epoch's worth of visible space. The block title
    // ("Unclaimed") plus self-explanatory columns stand in for column labels.
    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(20),
        ],
    )
    .block(block)
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut table_state = TableState::default().with_selected(Some(app.cursor));
    frame.render_stateful_widget(table, area, &mut table_state);
}

/// Bordered, read-only, paginated table over ALL statuses (descending by
/// epoch), sliced to `app.page * PAGE_SIZE .. + PAGE_SIZE`.
fn draw_all_epochs_panel(frame: &mut Frame, app: &App, area: Rect) {
    let mut sorted: Vec<&EpochStatus> = app.statuses.iter().collect();
    sorted.sort_unstable_by_key(|s| std::cmp::Reverse(s.epoch));

    let max_page = app.statuses.len().saturating_sub(1) / PAGE_SIZE;
    let start = app.page * PAGE_SIZE;

    let rows: Vec<Row> = sorted
        .iter()
        .skip(start)
        .take(PAGE_SIZE)
        .map(|status| {
            let amount_str = status
                .amount
                .map(format_jitosol)
                .unwrap_or_else(|| "-".to_string());
            Row::new(vec![
                status.epoch.to_string(),
                amount_str,
                status_str(status).to_string(),
            ])
        })
        .collect();

    let title = format!("All epochs — page {}/{}", app.page + 1, max_page + 1);
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(15),
        ],
    )
    .header(Row::new(vec!["Epoch", "Amount (JitoSOL)", "Status"]))
    .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(table, area);
}

fn draw_dashboard_footer(frame: &mut Frame, app: &App, area: Rect) {
    let stats_line = Stats::from(app.statuses.as_slice()).format();
    let lines = vec![
        Line::from(stats_line),
        Line::from("space select · a all · c claim · ←/→ page · r rescan · q quit"),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_confirm_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let mut epochs: Vec<u64> = app.selected.iter().copied().collect();
    epochs.sort_unstable();

    let mut lines: Vec<Line> = epochs
        .iter()
        .take(10)
        .map(|e| Line::from(e.to_string()))
        .collect();
    if epochs.len() > 10 {
        lines.push(Line::from(format!("… and {} more", epochs.len() - 10)));
    }

    let total: u64 = epochs
        .iter()
        .filter_map(|e| {
            app.statuses
                .iter()
                .find(|s| s.epoch == *e)
                .and_then(|s| s.amount)
        })
        .sum();
    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "Total: {} JitoSOL",
        format_jitosol(total)
    )));
    lines.push(Line::from(format!("Keypair path: {}", app.keypair_input)));
    if let Some(err) = &app.setup_error {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!("Error: {err}"),
            Style::default().fg(ratatui::style::Color::Red),
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("y confirm · n cancel"));

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm claim"),
        )
        .alignment(Alignment::Left);
    frame.render_widget(popup, area);
}

fn claim_state_str(state: &ClaimState) -> String {
    match state {
        ClaimState::Started => "…".to_string(),
        ClaimState::Success(sig) => format!("OK {sig}"),
        ClaimState::Failed(err) => format!("FAIL {err}"),
        ClaimState::Skipped(reason) => format!("skip {reason}"),
    }
}

fn draw_progress(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let has_error = app.progress_error.is_some();
    let mut constraints = vec![Constraint::Min(1)];
    if has_error {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1)); // footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let rows: Vec<Row> = app
        .progress_rows
        .iter()
        .map(|ev| Row::new(vec![ev.epoch.to_string(), claim_state_str(&ev.state)]))
        .collect();

    let table = Table::new(rows, [Constraint::Length(10), Constraint::Min(1)])
        .header(Row::new(vec!["Epoch", "State"]))
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .row_highlight_style(Style::default());

    let mut table_state =
        TableState::default().with_selected(Some(app.progress_rows.len().saturating_sub(1)));
    frame.render_stateful_widget(table, chunks[0], &mut table_state);

    let mut idx = 1;
    if let Some(err) = &app.progress_error {
        let error_para = Paragraph::new(format!("Claim run failed: {err}"))
            .style(Style::default().fg(ratatui::style::Color::Red));
        frame.render_widget(error_para, chunks[idx]);
        idx += 1;
    }

    if app.claim_done {
        let footer = Paragraph::new("b back to dashboard · q quit");
        frame.render_widget(footer, chunks[idx]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::EpochStatus;
    use crate::tui::app::App;
    use ratatui::{backend::TestBackend, Terminal};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    /// Extracts the text of a horizontal band of rows `[y, y + height)`,
    /// scoped to that band only (unlike `buffer_text`, which flattens the
    /// entire screen). Used to assert on one dashboard panel without
    /// accidentally matching content rendered by a different panel.
    fn rect_text(terminal: &Terminal<TestBackend>, y: u16, height: u16) -> String {
        let buffer = terminal.backend().buffer();
        let area = buffer.area;
        let mut out = String::new();
        for row in y..(y + height).min(area.height) {
            for col in 0..area.width {
                if let Some(cell) = buffer.cell((col, row)) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn dashboard_shows_epochs_and_unclaimed_summary() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![
            EpochStatus {
                epoch: 1000,
                amount: Some(1_500_000_000),
                claimed: false,
                expired: false,
            },
            EpochStatus {
                epoch: 999,
                amount: Some(1_000_000_000),
                claimed: true,
                expired: false,
            },
            EpochStatus {
                epoch: 998,
                amount: None,
                claimed: false,
                expired: false,
            },
        ];

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("1000"));
        assert!(text.contains("unclaimed"));
        assert!(text.contains("claimed"));
        assert!(text.contains("not eligible"));
        assert!(text.contains("1.500000000")); // formatted JitoSOL
    }

    #[test]
    fn setup_screen_shows_fields() {
        let app = App::new();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Network"));
        assert!(text.contains("RPC URL"));
        assert!(text.contains("Claimant"));
    }

    #[test]
    fn confirm_popup_shows_setup_error_when_keypair_missing() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Confirm;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![EpochStatus {
            epoch: 1,
            amount: Some(10),
            claimed: false,
            expired: false,
        }];
        app.selected = std::collections::HashSet::from([1]);
        app.setup_error = Some("keypair path is required".to_string());

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("keypair path is required"));
    }

    #[test]
    fn unclaimed_panel_scrolls_to_keep_cursor_visible() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        // 30 claimable epochs (all unclaimed, not expired) -> claimable_sorted()
        // is [329, 328, ..., 300]. cursor = 25 lands on epoch 304.
        app.statuses = (0..30)
            .map(|i| EpochStatus {
                epoch: 300 + i,
                amount: Some(1_000_000_000),
                claimed: false,
                expired: false,
            })
            .collect();
        app.cursor = 25;

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();

        // Unclaimed panel occupies rows [3, 3 + height); height is capped at 8
        // for a 30-row claimable list. Scope the assertion to that panel only,
        // since the All-epochs panel below also renders overlapping epoch
        // numbers on its own first page.
        let panel = rect_text(&terminal, 3, unclaimed_panel_height(30));

        // The epoch under the cursor must be visible...
        assert!(panel.contains("304"));
        // ...and the topmost claimable epoch must have scrolled out of view.
        assert!(!panel.contains("329"));
    }

    /// (a) Mixed fixture: the Unclaimed panel must contain only the claimable
    /// epochs (unclaimed, not expired, not "not eligible") and never show the
    /// empty-state message when there is something to claim.
    #[test]
    fn dashboard_unclaimed_panel_contains_only_claimable_epochs() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![
            EpochStatus {
                epoch: 101,
                amount: Some(10),
                claimed: true,
                expired: false,
            },
            EpochStatus {
                epoch: 202,
                amount: Some(20),
                claimed: false,
                expired: false,
            },
            EpochStatus {
                epoch: 303,
                amount: Some(30),
                claimed: false,
                expired: true,
            },
            EpochStatus {
                epoch: 404,
                amount: Some(40),
                claimed: false,
                expired: false,
            },
            EpochStatus {
                epoch: 505,
                amount: None,
                claimed: false,
                expired: false,
            },
        ];

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();

        // claimable_sorted() == [404, 202] -> panel height = min(2+2, 8) = 4.
        let panel = rect_text(&terminal, 3, 4);
        assert!(panel.contains("404"));
        assert!(panel.contains("202"));
        assert!(!panel.contains("101"));
        assert!(!panel.contains("303"));
        assert!(!panel.contains("505"));
        assert!(!panel.contains("Nothing to claim"));
    }

    /// (b) All-epochs table is sorted descending; page 0 shows the newest
    /// epoch and hides the oldest once the fixture exceeds PAGE_SIZE rows.
    #[test]
    fn dashboard_all_epochs_page_zero_sorted_desc_hides_oldest() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = (0..25)
            .map(|i| EpochStatus {
                epoch: 1000 + i,
                amount: Some(1),
                claimed: false,
                expired: false,
            })
            .collect();

        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("1024")); // newest epoch, visible on page 0
        assert!(!text.contains("1000")); // oldest epoch, only on page 1
    }

    /// (c) Setting `app.page = 1` shows the second slice and the page
    /// indicator advances accordingly.
    #[test]
    fn dashboard_page_one_shows_second_slice_and_indicator() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = (0..25)
            .map(|i| EpochStatus {
                epoch: 1000 + i,
                amount: Some(1),
                claimed: false,
                expired: false,
            })
            .collect();
        app.page = 1;

        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("page 2/2"));
        assert!(text.contains("1000")); // oldest epoch, now visible on page 1
    }

    /// (d) The footer's first line is exactly `Stats::format()`.
    #[test]
    fn dashboard_footer_shows_stats_line() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![
            EpochStatus {
                epoch: 1,
                amount: Some(1_500_000_000),
                claimed: true,
                expired: false,
            },
            EpochStatus {
                epoch: 2,
                amount: Some(2_000_000_000),
                claimed: false,
                expired: false,
            },
            EpochStatus {
                epoch: 3,
                amount: Some(500_000_000),
                claimed: false,
                expired: true,
            },
        ];
        let expected = crate::scanner::Stats::from(app.statuses.as_slice()).format();

        // Wide terminal so the full stats line fits on one row unwrapped.
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains(&expected));
    }

    /// (e) The `"expired"` status string is rendered in the All-epochs table.
    #[test]
    fn dashboard_all_epochs_shows_expired_status() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![EpochStatus {
            epoch: 7,
            amount: Some(1),
            claimed: false,
            expired: true,
        }];

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("expired"));
    }

    /// (f) An empty claimable set shows "Nothing to claim" in the Unclaimed
    /// panel instead of an empty table.
    #[test]
    fn dashboard_unclaimed_panel_shows_nothing_to_claim_when_empty() {
        let mut app = App::new();
        app.screen = crate::tui::app::Screen::Dashboard;
        app.claimant_input = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".into();
        app.statuses = vec![EpochStatus {
            epoch: 1,
            amount: Some(10),
            claimed: true,
            expired: false,
        }];

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("Nothing to claim"));
    }
}
