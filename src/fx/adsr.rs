use rodio::Source;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::audio_patch::Node;

pub type SynthSource = Box<dyn Source<Item = f32> + Send>;

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    pub attack_s: f32,   // seconds
    pub decay_s: f32,    // seconds
    pub sustain: f32,    // 0..1
    pub release_s: f32,  // seconds
}

pub struct AdsrEnvelope {
    pub attack_samples: u64,
    pub decay_samples: u64,
    pub release_samples: u64,
    pub sustain: f32,    // 0..1
    pub attack_step: f32,
    pub decay_step: f32,
    pub release_step: f32,
}

impl Adsr {
    pub fn to_envelope(&self, sample_rate: u32) -> AdsrEnvelope {
           let sr = sample_rate as f32;
           let attack_samples = (self.attack_s * sr).round() as u64;
           let decay_samples = (self.decay_s * sr).round() as u64;
           let release_samples = (self.release_s * sr).round() as u64;

           AdsrEnvelope {
               attack_samples,
               decay_samples,
               release_samples,
               sustain:self.sustain,
               attack_step: if attack_samples > 0 { 1.0 / attack_samples as f32 } else { 1.0 },
               decay_step: if decay_samples > 0 { (1.0 - self.sustain) / decay_samples as f32 } else { 1.0 - self.sustain },
               release_step: if release_samples > 0 { self.sustain / release_samples as f32 } else { self.sustain },
           }
       }

    pub fn new(attack_s: f32, decay_s: f32, sustain: f32, release_s: f32) -> Self {
        Self { attack_s, decay_s, sustain, release_s }
    }
}

pub type Gate = Arc<AtomicBool>; // on off

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

pub struct AdsrNode {
    pub adsr: Adsr,
    pub sample_rate: u32,
    pub gate: Gate,
}

impl AdsrNode {
    pub fn new(adsr: Adsr, sample_rate: u32, gate: Gate) -> Self {
        Self { adsr, sample_rate, gate }
    }
}

pub struct AdsrSource {
    input: SynthSource,
    envelope: AdsrEnvelope,
    sample_rate: u32,
    gate: Gate,
    stage: Stage,
    stage_pos: u64,
    current_amp: f32,
}

impl AdsrSource {
    pub fn new(input: SynthSource, adsr: Adsr, sample_rate: u32, gate: Gate) -> Self {
        Self {
            input,
            envelope: adsr.to_envelope(sample_rate),
            sample_rate,
            gate,
            stage: Stage::Attack,
            stage_pos: 0,
            current_amp: 0.0,
        }
    }

    fn step_envelope(&mut self) -> f32 {
        if !self.gate.load(Ordering::Relaxed)
            && self.stage != Stage::Release
            && self.stage != Stage::Done
        {
            self.stage = Stage::Release;
            self.stage_pos = 0;
            self.envelope.release_step = if self.envelope.release_samples > 0 {
                self.current_amp / self.envelope.release_samples as f32
            } else {
                self.current_amp
            };
        }

        match self.stage {
            Stage::Attack => {
                self.stage_pos += 1;
                if self.stage_pos >= self.envelope.attack_samples {
                    self.stage = Stage::Decay;
                    self.stage_pos = 0;
                    self.current_amp = 1.0;
                };
                self.current_amp += self.envelope.attack_step;
            }
            Stage::Decay => {
                self.stage_pos+=1;
                if self.stage_pos >=self.envelope.decay_samples {
                    self.stage=Stage::Sustain;
                    self.stage_pos=0;
                    self.current_amp=self.envelope.sustain;
                };
                self.current_amp-=self.envelope.decay_step;
            }
            Stage::Sustain => {
                self.current_amp = self.envelope.sustain;
            }
            Stage::Release => {
                self.stage_pos+=1;
                if self.stage_pos >=self.envelope.release_samples {
                    self.stage=Stage::Done;
                    self.stage_pos=0;
                    self.current_amp=0.0;
                };
                self.current_amp-=self.envelope.release_step;
            },
            Stage::Done => {
                self.current_amp = 0.0;
            }
        };
        self.current_amp.clamp(0.0, 1.0)
    }
}

impl Iterator for AdsrSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.stage == Stage::Done {
            return None;
        }
        let x = self.input.next()?;
        let env = self.step_envelope();
        if self.stage == Stage::Done {
            return None;
        }
        Some(x * env)
    }
}

impl Source for AdsrSource {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }
    fn channels(&self) -> u16 {
        self.input.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Node for AdsrNode {
    fn apply(&self, input: SynthSource) -> SynthSource {
        Box::new(AdsrSource::new(
            input,
            self.adsr,
            self.sample_rate,
            self.gate.clone(),
        ))
    }

    fn name(&self) -> &'static str {
        "ADSR"
    }
}
