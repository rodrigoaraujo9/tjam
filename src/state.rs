use tokio::sync::{RwLock, OnceCell, Notify};
use std::sync::Arc;
use crate::audio_source::{AudioSource, WaveSource};

pub struct AudioState {
    pub source: Arc<RwLock<Box<dyn AudioSource>>>,
    pub volume: Arc<RwLock<f32>>,
    pub muted: Arc<RwLock<bool>>,
    pub volume_notify: Arc<RwLock<Option<Arc<Notify>>>>,
    pub mute_notify: Arc<RwLock<Option<Arc<Notify>>>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            source: Arc::new(RwLock::new(Box::new(WaveSource::default()))),
            volume: Arc::new(RwLock::new(1.0)),
            muted: Arc::new(RwLock::new(false)),
            volume_notify: Arc::new(RwLock::new(None)),
            mute_notify: Arc::new(RwLock::new(None)),
        }
    }
}

static AUDIO_STATE: OnceCell<AudioState> = OnceCell::const_new();

pub async fn get_state() -> &'static AudioState {
    AUDIO_STATE.get_or_init(|| async { AudioState::new() }).await
}
