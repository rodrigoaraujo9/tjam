use std::time::Duration;

use rodio::Source;
use rodio::source::SquareWave;

use crate::audio_patch::AudioSource;
use crate::patches::types::DynSrc;
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};


pub struct BasicSquareSource {
    pub amplitude: f32,
    pub duration: Duration,
    pub sample_rate: u32,
}

impl Default for BasicSquareSource {
    fn default() -> Self {
        Self {
            amplitude: AMP_DEFAULT,
            duration: ENDLESS,
            sample_rate: SAMPLE_RATE,
        }
    }
}

impl AudioSource for BasicSquareSource {
    fn create_source(&self, frequency: f32) -> DynSrc {
        Box::new(
            SquareWave::new(frequency)
                .amplify(self.amplitude)
                .take_duration(self.duration),
        )
    }

    fn name(&self) -> &'static str {
        "Square"
    }
}
