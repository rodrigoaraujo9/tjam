use rodio::Source;
use std::time::Duration;
use crate::patches::types::DynSrc;
use crate::audio_patch::AudioSource;
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};

pub struct BasicNoiseSource {
    pub amplitude: f32,
    pub duration: Duration,
    pub sample_rate: u32,
    pub seed: u64,
}

impl Default for BasicNoiseSource {
    fn default() -> Self {
        Self {
            amplitude: AMP_DEFAULT,
            duration: ENDLESS,
            sample_rate: SAMPLE_RATE,
            seed: 0x1234_5678_9ABC_DEF0,
        }
    }
}

impl AudioSource for BasicNoiseSource {
    fn create_source(&self, _frequency: f32) -> DynSrc {
        Box::new(Noise::new(self.seed, self.sample_rate).amplify(self.amplitude).take_duration(self.duration))
    }

    fn name(&self) -> &'static str {
        "Noise"
    }
}

struct Noise {
    rng: u64,
    sr: u32,
}

impl Noise {
    fn new(seed: u64, sr: u32) -> Self {
        Self { rng: seed, sr }
    }

    fn next_noise(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let y = x.wrapping_mul(0x2545F4914F6CDD1D);

        let u = (y >> 40) as u32;
        let f = u as f32 / ((1u32 << 24) as f32);
        2.0 * f - 1.0
    }
}

impl Iterator for Noise {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        Some(self.next_noise())
    }
}

impl Source for Noise {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sr }
    fn total_duration(&self) -> Option<Duration> { None }
}
