use crate::audio_patch::AudioSource;
use rodio::Source;
use rodio::source::SineWave;
use std::time::Duration;
use crate::audio_patch::DynSrc;
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};


pub struct BasicSineSource {
    pub amplitude: f32,
    pub duration: Duration,
    pub sample_rate: u32,
}

impl Default for BasicSineSource {
    fn default() -> Self {
        Self {
            amplitude: AMP_DEFAULT,
            duration: ENDLESS,
            sample_rate: SAMPLE_RATE,
        }
    }
}

impl AudioSource for BasicSineSource {
    fn create_source(&self, frequency: f32) -> DynSrc {
        Box::new(
            SineWave::new(frequency)
                .take_duration(self.duration)
                .amplify(self.amplitude)
        )
    }

    fn name(&self) -> &'static str {
        "Sine"
    }
}

fn default_source() -> Box<dyn AudioSource> {
    Box::new(BasicSineSource::default())
}
