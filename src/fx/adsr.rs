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

impl Adsr {
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
    adsr: Adsr,
    sample_rate: u32,
    gate: Gate,
    stage: Stage,
    stage_pos: u64,
    current_amp: f32,
    target_amp: f32,
}

impl AdsrSource {
    pub fn new(input: SynthSource, adsr: Adsr, sample_rate: u32, gate: Gate) -> Self {
        Self {
            input,
            adsr,
            sample_rate,
            gate,
            stage: Stage::Attack,
            stage_pos: 0,
            current_amp: 0.0,
            target_amp: 0.0,
        }
    }

    fn stage_len_samples(&self, stage: Stage) -> u64 {
        let sr = self.sample_rate as f32;
        let s = match stage {
            Stage::Attack => self.adsr.attack_s.max(0.0),
            Stage::Decay => self.adsr.decay_s.max(0.0),
            Stage::Release => self.adsr.release_s.max(0.0),
            Stage::Sustain | Stage::Done => 0.0,
        };
        (s * sr).round() as u64
    }

    fn step_envelope(&mut self) -> f32 {
        // implement step for attack, decay, sustain and release
        // match the stage we are in -> act accordingly
        // stage pos (step) depends on sample rate directly
        // we will be changing amplitude (current_amp) according to stage_pos
        // y = x * env; on next -> iterator, so multiplicative
        //
        // attack  -> amp starts at 0 and increments (propportionally to sample rate and attack_s) until = target_amp
        // release -> amp starts at target_amp and decrements (propportionally to sample rate and attack_s) until = 0
        // implement decay and sustain later -> slightly more complex
        -1.0
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
        let y = x * env;
        if self.stage == Stage::Done {
            return None;
        }
        Some(y)
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
