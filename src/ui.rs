use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
};

use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge},
};

use tokio::sync::{watch, mpsc};

use crate::state::{AudioHandle, AudioSnapshot};

struct TuiGuard;

impl Drop for TuiGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

pub async fn run_ui(
    handle: AudioHandle,
    shutdown_tx: watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TuiGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut snap_rx = handle.subscribe();
    let mut snap = *snap_rx.borrow();

    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<KeyEvent>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_bg = stop.clone();

    std::thread::spawn(move || {
        while !stop_bg.load(Ordering::Relaxed) {
            if event::poll(Duration::from_millis(50)).ok() == Some(true) {
                if let Ok(Event::Key(k)) = event::read() {
                    if k.kind == KeyEventKind::Press {
                        let _ = key_tx.send(k);
                    }
                }
            }
        }
    });

    loop {
        terminal.draw(|f| draw_ui(f, snap))?;

        tokio::select! {
            changed = snap_rx.changed() => {
                if changed.is_ok() {
                    snap = *snap_rx.borrow();
                }
            }

            k = key_rx.recv() => {
                let Some(k) = k else { break; };

                if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
                    let _ = shutdown_tx.send(true);
                    break;
                }

                match k.code {
                    KeyCode::Char('q') => {
                        let _ = shutdown_tx.send(true);
                        break;
                    }

                    KeyCode::Left | KeyCode::Char('-') => {
                        let next = (snap.volume - 0.05).clamp(0.0, 2.0);
                        handle.set_volume(next);
                        snap.volume = next;
                    }

                    KeyCode::Right | KeyCode::Char('=') | KeyCode::Char('+') => {
                        let next = (snap.volume + 0.05).clamp(0.0, 2.0);
                        handle.set_volume(next);
                        snap.volume = next;
                    }

                    KeyCode::Char('m') => {
                        let m = !snap.muted;
                        handle.set_muted(m);
                        snap.muted = m;
                    }

                    KeyCode::Char('r') => {
                        handle.rotate_source();
                    }

                    _ => {}
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(16)) => {}
        }
    }

    stop.store(true, Ordering::Relaxed);
    terminal.show_cursor()?;
    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, snap: AudioSnapshot) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(7)])
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::raw("tjam ").bold(),
        Span::raw("— ").dark_gray(),
        Span::raw("audio + ui").dark_gray(),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, root[0]);

    let banner = source_banner(snap.kind.name());
    let big = Paragraph::new(banner).alignment(Alignment::Center).block(Block::default());
    f.render_widget(big, root[1]);

    let controls = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(root[2]);

    let vol_pct_f32 = (snap.volume / 2.0).clamp(0.0, 1.0);
    let vol_pct: f64 = vol_pct_f32 as f64;

    let vol_label = format!("VOLUME  {:.0}%", vol_pct_f32 * 100.0);
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(vol_label))
        .ratio(vol_pct);
    f.render_widget(gauge, controls[0]);

    let mute_txt = if snap.muted { "[M] MUTE: ON" } else { "[M] MUTE: OFF" };
    let src_txt = format!("[R] ROTATE SOURCE  ({})", snap.kind.name());
    let hint = "[←/→] or [-/=] VOLUME   [CTRL+C/Q] QUIT";

    let right = Paragraph::new(vec![
        Line::from(Span::raw(mute_txt).add_modifier(Modifier::BOLD)),
        Line::from(Span::raw(src_txt)),
        Line::from(""),
        Line::from(Span::raw(hint).dark_gray()),
    ])
    .alignment(Alignment::Left)
    .block(Block::default().borders(Borders::ALL).title("CONTROLS"));
    f.render_widget(right, controls[1]);
}

fn source_banner(name: &str) -> Vec<Line<'static>> {
    let lines: [&'static str; 5] = match name {
        "Sine" => [
            "  ██████  ██ ███    ██ ███████ ",
            " ██       ██ ████   ██ ██      ",
            "  ██████  ██ ██ ██  ██ █████   ",
            "       ██ ██ ██  ██ ██ ██      ",
            "  ██████  ██ ██   ████ ███████ ",
        ],
        "Saw" => [
            " ███████  █████  ██     ██ ",
            " ██      ██   ██ ██     ██ ",
            " ███████ ███████ ██  █  ██ ",
            "      ██ ██   ██ ██ ███ ██ ",
            " ███████ ██   ██  ███ ███  ",
        ],
        "Square" => [
            " ███████  ██████  ██    ██  █████  ██████  ███████ ",
            " ██      ██    ██ ██    ██ ██   ██ ██   ██ ██      ",
            " ███████ ██    ██ ██    ██ ███████ ██████  █████   ",
            "      ██ ██ ▄▄ ██ ██    ██ ██   ██ ██   ██ ██      ",
            " ███████  ██████   ██████  ██   ██ ██   ██ ███████ ",
        ],
        "Triangle" => [
            " ████████ ██████  ██ ██  █████  ███    ██  ██████  ██      ███████ ",
            "    ██    ██   ██ ██ ██ ██   ██ ████   ██ ██       ██      ██      ",
            "    ██    ██████  ██ ██ ███████ ██ ██  ██ ██   ███ ██      █████   ",
            "    ██    ██   ██ ██ ██ ██   ██ ██  ██ ██ ██    ██ ██      ██      ",
            "    ██    ██   ██ ██ ██ ██   ██ ██   ████  ██████  ███████ ███████ ",
        ],
        "Noise" => [
            " ███    ██  ██████  ██ ███████ ███████ ",
            " ████   ██ ██    ██ ██ ██      ██      ",
            " ██ ██  ██ ██    ██ ██ ███████ █████   ",
            " ██  ██ ██ ██    ██ ██      ██ ██      ",
            " ██   ████  ██████  ██ ███████ ███████ ",
        ],
        _ => [
            "████████████████████████████",
            "         UNKNOWN SRC         ",
            "████████████████████████████",
            "                            ",
            "                            ",
        ],
    };
    lines.into_iter().map(|s| Line::from(Span::raw(s).bold())).collect()
}
