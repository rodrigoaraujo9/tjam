use std::time::Duration;

use rodio::Source;
use rodio::source::TriangleWave;

use crate::audio_patch::AudioSource;
use crate::patches::types::DynSrc;
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};


pub struct BasicTriangleSource {
    pub amplitude: f32,
    pub duration: Duration,
    pub sample_rate: u32,
}

impl Default for BasicTriangleSource {
    fn default() -> Self {
        Self {
            amplitude: AMP_DEFAULT,
            duration: ENDLESS,
            sample_rate: SAMPLE_RATE,
        }
    }
}

impl AudioSource for BasicTriangleSource {
    fn create_source(&self, frequency: f32) -> DynSrc {
        Box::new(
            TriangleWave::new(frequency)
                .amplify(self.amplitude)
                .take_duration(self.duration),
        )
    }

    fn name(&self) -> &'static str {
        "Triangle"
    }
}
