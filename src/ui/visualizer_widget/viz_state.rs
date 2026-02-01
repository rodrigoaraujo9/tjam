use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::style::Color;

use crate::audio_capture::Matrix;

use super::displays::{Oscilloscope, Spectroscope, Vectorscope};
use super::types::{update_value_f, update_value_i, DataSet, DisplayMode, GraphConfig};

#[derive(Debug, Clone, Copy)]
pub enum DisplayKind {
    Oscilloscope,
    Vectorscope,
    Spectroscope,
}

pub struct VisualizerState {
    pub graph: GraphConfig,

    oscilloscope: Oscilloscope,
    vectorscope: Vectorscope,
    spectroscope: Spectroscope,

    mode: DisplayKind,

    datasets: Vec<DataSet>,
    last_audio: Option<Matrix<f64>>,
    fps: FpsCounter,
}

struct FpsCounter {
    frames: usize,
    framerate: usize,
    last_update: std::time::Instant,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frames: 0,
            framerate: 0,
            last_update: std::time::Instant::now(),
        }
    }

    fn tick(&mut self) {
        self.frames += 1;
        if self.last_update.elapsed().as_secs() >= 1 {
            self.framerate = self.frames;
            self.frames = 0;
            self.last_update = std::time::Instant::now();
        }
    }

    fn get(&self) -> usize {
        self.framerate
    }
}

impl Default for VisualizerState {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualizerState {
    pub fn new() -> Self {
        let mut graph = GraphConfig::default();
        if graph.palette.is_empty() {
            graph.palette = vec![Color::White];
        }

        let mut spectro = Spectroscope::default();
        spectro.sampling_rate = graph.sampling_rate;
        spectro.buffer_size = graph.width;

        Self {
            graph,
            oscilloscope: Oscilloscope::default(),
            vectorscope: Vectorscope::default(),
            spectroscope: spectro,
            mode: DisplayKind::Oscilloscope,
            datasets: Vec::new(),
            last_audio: None,
            fps: FpsCounter::new(),
        }
    }

    pub fn mode(&self) -> DisplayKind {
        self.mode
    }

    pub fn fps(&self) -> usize {
        self.fps.get()
    }

    pub fn datasets(&self) -> &[DataSet] {
        &self.datasets
    }

    pub fn current_display(&self) -> &dyn DisplayMode {
        match self.mode {
            DisplayKind::Oscilloscope => &self.oscilloscope,
            DisplayKind::Vectorscope => &self.vectorscope,
            DisplayKind::Spectroscope => &self.spectroscope,
        }
    }

    pub fn current_display_mut(&mut self) -> &mut dyn DisplayMode {
        match self.mode {
            DisplayKind::Oscilloscope => &mut self.oscilloscope,
            DisplayKind::Vectorscope => &mut self.vectorscope,
            DisplayKind::Spectroscope => &mut self.spectroscope,
        }
    }

    pub fn update(&mut self, audio: Option<&Matrix<f64>>) {
        self.fps.tick();

        // Freeze correctly when paused: store last frame only when not paused.
        if !self.graph.pause {
            self.last_audio = audio.cloned();
        }

        // Keep spectro settings in sync with graph
        self.spectroscope.sampling_rate = self.graph.sampling_rate;
        self.spectroscope.buffer_size = self.graph.width;

        self.datasets.clear();

        let Some(data) = self.last_audio.as_ref() else { return; };

        // references first (immutable borrow of the active display field only)
        if self.graph.references {
            let refs = match self.mode {
                DisplayKind::Oscilloscope => self.oscilloscope.references(&self.graph),
                DisplayKind::Vectorscope => self.vectorscope.references(&self.graph),
                DisplayKind::Spectroscope => self.spectroscope.references(&self.graph),
            };
            self.datasets.extend(refs);
        }

        // then process (mutable borrow of the active display field only)
        let processed = match self.mode {
            DisplayKind::Oscilloscope => self.oscilloscope.process(&self.graph, data),
            DisplayKind::Vectorscope => self.vectorscope.process(&self.graph, data),
            DisplayKind::Spectroscope => self.spectroscope.process(&self.graph, data),
        };

        self.datasets.extend(processed);
    }


    /// Returns true => quit requested
    pub fn handle_event(&mut self, event: Event) -> bool {
        let mut quit = false;

        if let Event::Key(key) = event {
            if key.modifiers == KeyModifiers::CONTROL {
                if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('w')) {
                    return true;
                }
            }

            let magnitude = match key.modifiers {
                KeyModifiers::SHIFT => 10.0,
                KeyModifiers::CONTROL => 5.0,
                KeyModifiers::ALT => 0.2,
                _ => 1.0,
            };

            match key.code {
                KeyCode::Up => update_value_f(&mut self.graph.scale, 0.01, magnitude, 0.0..10.0),
                KeyCode::Down => update_value_f(&mut self.graph.scale, -0.01, magnitude, 0.0..10.0),

                KeyCode::Right => update_value_i(
                    &mut self.graph.samples,
                    true,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),
                KeyCode::Left => update_value_i(
                    &mut self.graph.samples,
                    false,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),

                KeyCode::Char(' ') => self.graph.pause = !self.graph.pause,
                KeyCode::Char('s') => self.graph.scatter = !self.graph.scatter,
                KeyCode::Char('h') => self.graph.show_ui = !self.graph.show_ui,
                KeyCode::Char('r') => self.graph.references = !self.graph.references,
                KeyCode::Char('q') => quit = true,

                KeyCode::Tab => {
                    self.mode = match self.mode {
                        DisplayKind::Oscilloscope => DisplayKind::Vectorscope,
                        DisplayKind::Vectorscope => DisplayKind::Spectroscope,
                        DisplayKind::Spectroscope => DisplayKind::Oscilloscope,
                    };
                }

                KeyCode::Esc => {
                    self.graph.samples = self.graph.width;
                    self.graph.scale = 1.0;
                    // per-mode resets are handled by each mode’s Esc handler as well
                }

                _ => {}
            }

            // give the active display a chance to handle keys too
            self.current_display_mut().handle(event);
        }

        quit
    }
}
