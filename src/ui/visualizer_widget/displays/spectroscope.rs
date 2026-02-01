use std::collections::VecDeque;

use crossterm::event::{Event, KeyCode};
use ratatui::{
    style::Style,
    text::Span,
    widgets::{Axis, GraphType},
};

use crate::audio_capture::Matrix;

use super::super::types::{update_value_i, DataSet, Dimension, DisplayMode, GraphConfig};

use rustfft::{num_complex::Complex, FftPlanner};

pub struct Spectroscope {
    pub sampling_rate: u32,
    pub buffer_size: u32,
    pub average: u32,
    pub buf: Vec<VecDeque<Vec<f64>>>,
    pub window: bool,
    pub log_y: bool,
}

fn magnitude(c: Complex<f64>) -> f64 {
    ((c.re * c.re) + (c.im * c.im)).sqrt()
}

pub fn hann_window(samples: &[f64]) -> Vec<f64> {
    let mut windowed_samples = Vec::with_capacity(samples.len());
    let samples_len = samples.len() as f64;
    for (i, sample) in samples.iter().enumerate() {
        let two_pi_i = 2.0 * std::f64::consts::PI * i as f64;
        let c = (two_pi_i / samples_len).cos();
        let multiplier = 0.5 * (1.0 - c);
        windowed_samples.push(sample * multiplier)
    }
    windowed_samples
}

impl Default for Spectroscope {
    fn default() -> Self {
        Self {
            sampling_rate: 48_000,
            buffer_size: 2048,
            average: 1,
            buf: Vec::new(),
            window: false,
            log_y: true,
        }
    }
}

impl DisplayMode for Spectroscope {
    fn mode_str(&self) -> &'static str {
        "spectro"
    }

    fn channel_name(&self, index: usize) -> String {
        match index {
            0 => "L".into(),
            1 => "R".into(),
            _ => format!("{}", index),
        }
    }

    fn header(&self, _: &GraphConfig) -> String {
        let window_marker = if self.window { "-|-" } else { "---" };
        if self.average <= 1 {
            format!(
                "live  {}  {:.3}Hz bins",
                window_marker,
                self.sampling_rate as f64 / self.buffer_size as f64
            )
        } else {
            format!(
                "{}x avg ({:.1}s)  {}  {:.3}Hz bins",
                self.average,
                (self.average * self.buffer_size) as f64 / self.sampling_rate as f64,
                window_marker,
                self.sampling_rate as f64 / (self.buffer_size * self.average) as f64
            )
        }
    }

    fn axis(&self, cfg: &GraphConfig, dimension: Dimension) -> Axis {
        let (name, bounds) = match dimension {
            Dimension::X => (
                "frequency -",
                [
                    20.0f64.ln(),
                    ((cfg.samples as f64 / cfg.width as f64) * 20000.0).ln(),
                ],
            ),
            Dimension::Y => (
                if self.log_y { "| level" } else { "| amplitude" },
                [0.0, cfg.scale * 7.5],
            ),
        };
        let mut a = Axis::default();
        if cfg.show_ui {
            a = a.title(Span::styled(name, Style::default().fg(cfg.labels_color)));
        }
        a.style(Style::default().fg(cfg.axis_color)).bounds(bounds)
    }

    fn process(&mut self, cfg: &GraphConfig, data: &Matrix<f64>) -> Vec<DataSet> {
        if self.average == 0 {
            self.average = 1;
        }

        if !cfg.pause {
            for (i, chan) in data.iter().enumerate() {
                if self.buf.len() <= i {
                    self.buf.push(VecDeque::new());
                }
                self.buf[i].push_back(chan.clone());
                while self.buf[i].len() > self.average as usize {
                    self.buf[i].pop_front();
                }
            }
        }

        let mut out = Vec::new();
        let mut planner: FftPlanner<f64> = FftPlanner::new();
        let sample_len = self.buffer_size * self.average;
        let resolution = self.sampling_rate as f64 / sample_len as f64;
        let fft = planner.plan_fft_forward(sample_len as usize);

        for (n, chan_queue) in self.buf.iter().enumerate().rev() {
            let mut chunk = chan_queue.iter().flatten().copied().collect::<Vec<f64>>();
            if chunk.is_empty() {
                continue;
            }
            if self.window {
                chunk = hann_window(chunk.as_slice());
            }

            let mut max_val = *chunk.iter().max_by(|a, b| a.total_cmp(b)).unwrap_or(&1.0);
            if max_val < 1.0 {
                max_val = 1.0;
            }

            let mut tmp: Vec<Complex<f64>> = chunk
                .iter()
                .map(|x| Complex {
                    re: *x / max_val,
                    im: 0.0,
                })
                .collect();

            fft.process(tmp.as_mut_slice());

            out.push(DataSet::new(
                Some(self.channel_name(n)),
                tmp[..=tmp.len() / 2]
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        (
                            (i as f64 * resolution).ln(),
                            if self.log_y { magnitude(*x).ln() } else { magnitude(*x) },
                        )
                    })
                    .collect(),
                cfg.marker_type,
                if cfg.scatter { GraphType::Scatter } else { GraphType::Line },
                cfg.palette(n),
            ));
        }

        out
    }

    fn handle(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::PageUp => update_value_i(&mut self.average, true, 1, 1.0, 1..65535),
                KeyCode::PageDown => update_value_i(&mut self.average, false, 1, 1.0, 1..65535),
                KeyCode::Char('w') => self.window = !self.window,
                KeyCode::Char('l') => self.log_y = !self.log_y,
                _ => {}
            }
        }
    }

    fn references(&self, cfg: &GraphConfig) -> Vec<DataSet> {
        let lower = 0.0;
        let upper = cfg.scale * 7.5;

        vec![
            DataSet::new(
                None,
                vec![(0.0, 0.0), ((cfg.samples as f64).ln(), 0.0)],
                cfg.marker_type,
                GraphType::Line,
                cfg.axis_color,
            ),
            // (kept from your version)
            DataSet::new(None, vec![(20.0f64.ln(), lower), (20.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(50.0f64.ln(), lower), (50.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(100.0f64.ln(), lower), (100.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(200.0f64.ln(), lower), (200.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(500.0f64.ln(), lower), (500.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(1000.0f64.ln(), lower), (1000.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(2000.0f64.ln(), lower), (2000.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(5000.0f64.ln(), lower), (5000.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(10000.0f64.ln(), lower), (10000.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
            DataSet::new(None, vec![(20000.0f64.ln(), lower), (20000.0f64.ln(), upper)], cfg.marker_type, GraphType::Line, cfg.axis_color),
        ]
    }
}
