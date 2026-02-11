use std::time::Duration;

use rodio::Source;
use rodio::source::SawtoothWave;

use crate::patches::types::DynSrc;
use crate::audio_patch::AudioSource;
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};


pub struct BasicSawSource {
    pub amplitude: f32,
    pub duration: Duration,
    pub sample_rate: u32,
}

impl Default for BasicSawSource {
    fn default() -> Self {
        Self {
            amplitude: AMP_DEFAULT,
            duration: ENDLESS,
            sample_rate: SAMPLE_RATE,
        }
    }
}

impl AudioSource for BasicSawSource {
    fn create_source(&self, frequency: f32) -> DynSrc {
        Box::new(
            SawtoothWave::new(frequency)
                .amplify(self.amplitude)
                .take_duration(self.duration),
        )
    }

    fn name(&self) -> &'static str {
        "Saw"
    }
}
