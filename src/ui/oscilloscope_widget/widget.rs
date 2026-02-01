use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Cell, Chart, Dataset, Row, StatefulWidget, Table},
};

use crate::audio_capture::Matrix;
use super::types::Dimension;
use super::OscilloscopeState;

pub struct OscilloscopeWidget<'a> {
    audio: Option<&'a Matrix<f64>>,
}

impl<'a> OscilloscopeWidget<'a> {
    pub fn new(audio: Option<&'a Matrix<f64>>) -> Self {
        Self { audio }
    }
}

impl<'a> StatefulWidget for OscilloscopeWidget<'a> {
    type State = OscilloscopeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.update(self.audio);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area);

        let mut chart_area = chunks[0];

        if state.graph.show_ui && chart_area.height > 0 {
            let header_area = Rect {
                x: chart_area.x,
                y: chart_area.y,
                width: chart_area.width,
                height: 1,
            };
            render_header(header_area, buf, state);

            chart_area.y += 1;
            chart_area.height = chart_area.height.saturating_sub(1);
        }

        if chart_area.height == 0 || chart_area.width == 0 {
            return;
        }

        let datasets: Vec<Dataset> = state.datasets().iter().map(Dataset::from).collect();

        let chart = Chart::new(datasets)
            .x_axis(state.scope.axis(&state.graph, Dimension::X))
            .y_axis(state.scope.axis(&state.graph, Dimension::Y));

        ratatui::widgets::Widget::render(chart, chart_area, buf);
    }
}

fn render_header(area: Rect, buf: &mut Buffer, state: &OscilloscopeState) {
    let fps = state.fps();
    let scope_header = state.scope.header(&state.graph);

    let title_color = *state.graph.palette.first().unwrap();

    let table = Table::new(
        vec![Row::new(vec![
            Cell::from("oscillo::tjam").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(scope_header),
            Cell::from(format!("-{:.2}x+", state.graph.scale)),
            Cell::from(format!("{}/{} spf", state.graph.samples, state.graph.width)),
            Cell::from(format!("{}fps", fps)),
            Cell::from(if state.graph.scatter { "***" } else { "---" }),
            Cell::from(if state.graph.pause { "||" } else { "|>" }),
        ])],
        vec![
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(7),
            Constraint::Percentage(13),
            Constraint::Percentage(6),
            Constraint::Percentage(6),
            Constraint::Percentage(6),
        ],
    )
    .style(Style::default().fg(state.graph.labels_color));

    ratatui::widgets::Widget::render(table, area, buf);
}
