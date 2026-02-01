use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::style::Color;

use crate::audio_capture::Matrix;

use super::oscilloscope::Oscilloscope;
use super::types::{update_value_f, update_value_i, DataSet, GraphConfig};

pub struct OscilloscopeState {
    pub graph: GraphConfig,
    pub scope: Oscilloscope,
    datasets: Vec<DataSet>,
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

impl Default for OscilloscopeState {
    fn default() -> Self {
        Self::new()
    }
}

impl OscilloscopeState {
    pub fn new() -> Self {
        let mut graph = GraphConfig::default();

        // safe default if palette ever empty
        if graph.palette.is_empty() {
            graph.palette = vec![Color::White];
        }

        Self {
            graph,
            scope: Oscilloscope::default(),
            datasets: Vec::new(),
            fps: FpsCounter::new(),
        }
    }

    /// Call this each tick when you have (or don't have) audio.
    /// If `graph.pause` is true, it keeps the previous datasets.
    pub fn update(&mut self, audio: Option<&Matrix<f64>>) {
        self.fps.tick();

        if self.graph.pause {
            return;
        }

        self.datasets.clear();

        if let Some(data) = audio {
            if self.graph.references {
                self.datasets.extend(self.scope.references(&self.graph));
            }
            self.datasets.extend(self.scope.process(&self.graph, data));
        }
    }

    pub fn datasets(&self) -> &[DataSet] {
        &self.datasets
    }

    pub fn fps(&self) -> usize {
        self.fps.get()
    }

    /// Global keys + scope-specific keys in one place.
    /// Return true if caller should quit.
    pub fn handle_event(&mut self, event: Event) -> bool {
        let mut quit = false;

        if let Event::Key(key) = event {
            // quit shortcuts
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('w') => return true,
                    _ => {}
                }
            }

            let magnitude = match key.modifiers {
                KeyModifiers::SHIFT => 10.0,
                KeyModifiers::CONTROL => 5.0,
                KeyModifiers::ALT => 0.2,
                _ => 1.0,
            };

            // global graph keys
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
                KeyCode::Esc => {
                    self.graph.samples = self.graph.width;
                    self.graph.scale = 1.0;
                    self.scope.reset();
                }
                _ => {}
            }

            // scope-specific keys
            self.scope.handle_event(event);
        }

        quit
    }
}
