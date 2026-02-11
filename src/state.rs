use tokio::sync::{OnceCell, Notify, RwLock};
use std::sync::Arc;

use crate::audio_patch::AudioSource;

use crate::patches::saw::BasicSawSource;
use crate::patches::sine::BasicSineSource;
use crate::patches::square::BasicSquareSource;
use crate::patches::triangle::BasicTriangleSource;
use crate::patches::noise::BasicNoiseSource;

type SourceFactory = fn() -> Box<dyn AudioSource>;

fn make_sine() -> Box<dyn AudioSource> { Box::new(BasicSineSource::default()) }
fn make_square() -> Box<dyn AudioSource> { Box::new(BasicSquareSource::default()) }
fn make_triangle() -> Box<dyn AudioSource> { Box::new(BasicTriangleSource::default()) }
fn make_noise() -> Box<dyn AudioSource> { Box::new(BasicNoiseSource::default()) }
fn make_saw() -> Box<dyn AudioSource> { Box::new(BasicSawSource::default()) }


static SOURCES: &[SourceFactory] = &[
    make_sine,
    make_saw,
    make_square,
    make_triangle,
    make_noise,
];

pub struct AudioState {
    pub source: Arc<RwLock<Box<dyn AudioSource>>>,
    pub volume: Arc<RwLock<f32>>,
    pub muted: Arc<RwLock<bool>>,
    pub source_idx: Arc<RwLock<usize>>,
    pub volume_notify: Arc<Notify>,
    pub mute_notify: Arc<Notify>,
    pub source_notify: Arc<Notify>,
}

impl AudioState {
    pub fn new() -> Self {
        let idx = 0usize;
        Self {
            source: Arc::new(RwLock::new((SOURCES[idx])())),
            volume: Arc::new(RwLock::new(1.0)),
            muted: Arc::new(RwLock::new(false)),
            source_idx: Arc::new(RwLock::new(idx)),
            volume_notify: Arc::new(Notify::new()),
            mute_notify: Arc::new(Notify::new()),
            source_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn set_volume(&self, v: f32) {
        *self.volume.write().await = v.clamp(0.0, 2.0);
        self.volume_notify.notify_waiters();
    }

    pub async fn set_muted(&self, m: bool) {
        *self.muted.write().await = m;
        self.mute_notify.notify_waiters();
    }

    pub async fn rotate_source(&self) {
        let mut idx = self.source_idx.write().await;
        *idx = (*idx + 1) % SOURCES.len();
        *self.source.write().await = (SOURCES[*idx])();
        self.source_notify.notify_waiters();
    }
}

static AUDIO_STATE: OnceCell<AudioState> = OnceCell::const_new();

pub async fn get_state() -> &'static AudioState {
    AUDIO_STATE.get_or_init(|| async { AudioState::new() }).await
}
