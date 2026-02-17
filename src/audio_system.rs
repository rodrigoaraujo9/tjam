use tokio::sync::{mpsc, watch, OnceCell, Mutex};
use crate::audio_patch::AudioSource;
use crate::fx::adsr::Adsr;


/// current audio state that the UI can read (volume/mute + which source is active).
#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub patch_name: String,
}

/// cmds that the UI sends to the audio runtime to change behavior
pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    TogglePatch(Vec<Box<dyn AudioSource>>),
    SetPatch(Box<dyn AudioSource>),
    SetAdsr(Adsr),
}

/// handle used by the UI: send commands + subscribe to live snapshots
#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::UnboundedSender<AudioCommand>,
    snapshot_rx: watch::Receiver<AudioSnapshot>,
}

impl AudioHandle {
    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(v));
    }

    pub fn set_muted(&self, m: bool) {
        let _ = self.tx.send(AudioCommand::SetMuted(m));
    }

    pub fn toggle_patch(&self, patches: Vec<Box<dyn AudioSource>>) {
        let _ = self.tx.send(AudioCommand::TogglePatch(patches));
    }

    pub fn set_patch(&self, patch: Box<dyn AudioSource>) {
        let _ = self.tx.send(AudioCommand::SetPatch(patch));
    }

    pub fn set_adsr(&self, adsr: Adsr) {
        let _ = self.tx.send(AudioCommand::SetAdsr(adsr));
    }

    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> {
        self.snapshot_rx.clone()
    }
}

/// internal singleton state: exposes a handle + owns the runtime channels.
struct AudioSystem {
    handle: AudioHandle,
    cmd_rx: Mutex<Option<mpsc::UnboundedReceiver<AudioCommand>>>,
    snapshot_tx: watch::Sender<AudioSnapshot>,
}

/// global singleton so UI and audio task share the same channels without passing them everywhere
static AUDIO: OnceCell<AudioSystem> = OnceCell::const_new();

pub async fn get_handle() -> &'static AudioHandle {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
            let initial = AudioSnapshot {
                volume: 1.0,
                muted: false,
                patch_name: "Sine".to_string(),
            };
            let (snapshot_tx, snapshot_rx) = watch::channel(initial);
            AudioSystem {
                handle: AudioHandle { tx: cmd_tx, snapshot_rx },
                cmd_rx: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
            }
        })
        .await
        .handle
}

pub async fn take_runtime_channels(
) -> (mpsc::UnboundedReceiver<AudioCommand>, watch::Sender<AudioSnapshot>, AudioSnapshot) {
    let sys = AUDIO.get_or_init(|| async { unreachable!("call get_handle() first") }).await;
    let mut guard = sys.cmd_rx.lock().await;
    let rx = guard.take().expect("audio runtime already taken");
    let initial = sys.snapshot_tx.borrow().clone();
    (rx, sys.snapshot_tx.clone(), initial)
}
