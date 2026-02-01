use ratatui::{
    style::{Color, Style},
    symbols::Marker,
    widgets::{Dataset, GraphType},
};

pub enum Dimension {
    X,
    Y,
}

#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub pause: bool,
    pub samples: u32,
    pub sampling_rate: u32,
    pub scale: f64,
    pub width: u32,
    pub scatter: bool,
    pub references: bool,
    pub show_ui: bool,
    pub marker_type: Marker,
    pub palette: Vec<Color>,
    pub labels_color: Color,
    pub axis_color: Color,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            axis_color: Color::DarkGray,
            labels_color: Color::Cyan,
            palette: vec![Color::Red, Color::Yellow, Color::Green, Color::Magenta],
            scale: 1.0,
            width: 2048,
            samples: 2048,
            sampling_rate: 48_000,
            references: true,
            show_ui: true,
            scatter: false,
            pause: false,
            marker_type: Marker::Braille,
        }
    }
}

impl GraphConfig {
    pub fn palette(&self, index: usize) -> Color {
        *self
            .palette
            .get(index % self.palette.len())
            .unwrap_or(&Color::White)
    }
}

pub struct DataSet {
    pub name: Option<String>,
    pub data: Vec<(f64, f64)>,
    pub marker_type: Marker,
    pub graph_type: GraphType,
    pub color: Color,
}

impl DataSet {
    pub fn new(
        name: Option<String>,
        data: Vec<(f64, f64)>,
        marker_type: Marker,
        graph_type: GraphType,
        color: Color,
    ) -> Self {
        Self {
            name,
            data,
            marker_type,
            graph_type,
            color,
        }
    }
}

impl<'a> From<&'a DataSet> for Dataset<'a> {
    fn from(ds: &'a DataSet) -> Dataset<'a> {
        let mut out = Dataset::default();
        if let Some(name) = &ds.name {
            out = out.name(name.clone());
        }
        out.marker(ds.marker_type)
            .graph_type(ds.graph_type)
            .style(Style::default().fg(ds.color))
            .data(&ds.data)
    }
}

pub(crate) fn update_value_f(
    val: &mut f64,
    base: f64,
    magnitude: f64,
    range: std::ops::Range<f64>,
) {
    let delta = base * magnitude;
    let next = *val + delta;

    if next > range.end {
        *val = range.end
    } else if next < range.start {
        *val = range.start
    } else {
        *val = next;
    }
}

pub(crate) fn update_value_i(
    val: &mut u32,
    inc: bool,
    base: u32,
    magnitude: f64,
    range: std::ops::Range<u32>,
) {
    let delta = (base as f64 * magnitude) as u32;

    if inc {
        let next = val.saturating_add(delta);
        *val = next.min(range.end);
    } else {
        let next = val.saturating_sub(delta);
        *val = next.max(range.start);
    }
}
