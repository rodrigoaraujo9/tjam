use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use tokio::task::LocalSet;

use crate::audio_capture::Matrix;
use crate::play::run_audio;

use crate::ui::visualizer_widget::{VisualizerState, VisualizerWidget};

mod audio_capture;
mod audio_source;
mod config;
mod key;
mod play;
mod ui;
mod state;

struct App {
    viz: VisualizerState,
    cached_capture: Option<std::sync::Arc<audio_capture::AudioCapture>>,
}

impl App {
    fn new() -> Self {
        Self {
            viz: VisualizerState::new(),
            cached_capture: None,
        }
    }

    async fn refresh_capture(&mut self) {
        if self.cached_capture.is_none() {
            self.cached_capture = crate::state::get_audio_capture().await;
        }
    }

    fn read_audio_frame(&self) -> Option<Matrix<f64>> {
        self.cached_capture.as_ref()?.get_data()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let local = LocalSet::new();

    let res = local
        .run_until(async {
            let audio_handle = tokio::task::spawn_local(async {
                if let Err(e) = run_audio().await {
                    eprintln!("audio error: {e}");
                }
            });

            let mut app = App::new();
            let tick = Duration::from_millis(16);

            let tui_res = run_tui(&mut terminal, &mut app, tick).await;

            audio_handle.abort();
            tui_res
        })
        .await;

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    res?;
    Ok(())
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tick: Duration,
) -> io::Result<()> {
    loop {
        app.refresh_capture().await;

        let audio_frame = app.read_audio_frame();

        terminal.draw(|f| {
            let area = f.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(area);

            let viz_area = chunks[0];

            let widget = VisualizerWidget::new(audio_frame.as_ref());
            f.render_stateful_widget(widget, viz_area, &mut app.viz);
        })?;

        while event::poll(Duration::from_millis(0))? {
            let ev = event::read()?;

            if let Event::Key(k) = &ev {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
            }

            if is_quit_event(&ev) {
                return Ok(());
            }

            if handle_global_controls(&ev).await {
                return Ok(());
            }

            if app.viz.handle_event(ev) {
                return Ok(());
            }
        }

        tokio::time::sleep(tick).await;
    }
}

fn is_quit_event(ev: &Event) -> bool {
    match ev {
        Event::Key(k) if k.modifiers == KeyModifiers::CONTROL => {
            matches!(k.code, KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('w'))
        }
        _ => false,
    }
}

async fn handle_global_controls(ev: &Event) -> bool {
    let Event::Key(k) = ev else { return false; };

    match k.code {
        KeyCode::Char('q') => return true,

        KeyCode::Char('m') => {
            crate::state::toggle_mute().await;
        }

        KeyCode::Char('-') | KeyCode::Char('_') => {
            let v = crate::state::get_volume().await;
            crate::state::set_volume(v - 0.05).await;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let v = crate::state::get_volume().await;
            crate::state::set_volume(v + 0.05).await;
        }

        _ => {}
    }

    false
}
