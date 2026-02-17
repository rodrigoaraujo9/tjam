use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use rodio::stream::{OutputStream, OutputStreamBuilder};
use rodio::Sink;

use tokio::{signal::ctrl_c, task};

use crate::config::TICK;
use crate::key::Key;
use crate::patches::basic::{basic_source, BasicKind};
use crate::audio_system;
use crate::audio_patch::AudioSource;

/// runtime-owned audio output + currently playing notes (one sink per pressed key)
pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Sink>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self { stream, active_sinks: HashMap::new() })
    }

    fn stop_note(&mut self, keycode: Keycode) {
        if let Some(sink) = self.active_sinks.remove(&keycode) {
            sink.stop();
        }
    }

    fn stop_all(&mut self) {
        for (_, sink) in self.active_sinks.drain() {
            sink.stop();
        }
    }

    fn set_all_volume(&mut self, v: f32) {
        for sink in self.active_sinks.values_mut() {
            sink.set_volume(v);
        }
    }

    fn set_all_muted(&mut self, muted: bool) {
        if muted {
            for sink in self.active_sinks.values_mut() { sink.pause(); }
        } else {
            for sink in self.active_sinks.values_mut() { sink.play(); }
        }
    }
}

/// small state used by the runtime to decide how to play notes
struct RuntimeState {
    volume: f32,
    muted: bool,
    current_patch: Box<dyn AudioSource>,
    avaliable_patches: Vec<Box<dyn AudioSource>>,
    toggle_index: usize,
}

/// push the latest runtime state to watchers (UI)
fn publish_snapshot(tx: &tokio::sync::watch::Sender<audio_system::AudioSnapshot>, rt: &RuntimeState) {
    let _ = tx.send(audio_system::AudioSnapshot {
        volume: rt.volume,
        muted: rt.muted,
        patch_name: rt.current_patch.name().to_string(),
    });
}

/// start playing a key if not already active (one sink per key)
async fn play_note(play_state: &mut PlayState, rt: &RuntimeState, keycode: Keycode) {
    if play_state.active_sinks.contains_key(&keycode) {
        return;
    }

    let Some(key) = Key::from_keycode(keycode) else { return; };
    let freq = key.frequency();

    let sink = Sink::connect_new(&play_state.stream.mixer());
    sink.set_volume(rt.volume);
    if rt.muted { sink.pause(); }

    let src = rt.current_patch.create_source(freq);
    sink.append(src);

    play_state.active_sinks.insert(keycode, sink);
}

/// recreate all currently held notes (used when waveform/source changes)
async fn restart_active_notes(play_state: &mut PlayState, rt: &RuntimeState) {
    let keys: Vec<Keycode> = play_state.active_sinks.keys().copied().collect();
    for k in keys {
        play_state.stop_note(k);
        play_note(play_state, rt, k).await;
    }
}

/// cycle to next patch in the toggle list
fn cycle_patch(rt: &mut RuntimeState) {
    if rt.avaliable_patches.is_empty() {
        return;
    }
    rt.toggle_index = (rt.toggle_index + 1) % rt.avaliable_patches.len();
    rt.current_patch = basic_source(match rt.toggle_index {
        0 => BasicKind::Sine,
        1 => BasicKind::Saw,
        2 => BasicKind::Square,
        3 => BasicKind::Triangle,
        4 => BasicKind::Noise,
        _ => BasicKind::Sine,
    });
}

/// main audio runtime: listens to keyboard state + UI commands, and drives Rodio sinks
pub async fn run_audio(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // ensure global audio system is initialized and grab runtime channels
    let _handle = audio_system::get_handle().await.clone();
    let (mut cmd_rx, snapshot_tx, initial) = audio_system::take_runtime_channels().await;

    // boot runtime state from last published snapshot
    let mut rt = RuntimeState {
        volume: initial.volume,
        muted: initial.muted,
        current_patch: basic_source(BasicKind::Sine),
        avaliable_patches: vec![
            basic_source(BasicKind::Sine),
            basic_source(BasicKind::Saw),
            basic_source(BasicKind::Square),
            basic_source(BasicKind::Triangle),
            basic_source(BasicKind::Noise),
        ],
        toggle_index: 0,
    };

    // own the audio output + active notes
    let mut play_state = PlayState::new()?;
    publish_snapshot(&snapshot_tx, &rt);

    // flag used to stop the polling thread cleanly
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_bg = stop_flag.clone();

    // poll thread sends deltas: (now_keys, prev_keys, did_toggle_b_edge)
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<(HashSet<Keycode>, HashSet<Keycode>, bool)>>();

    let focused_bg = focused.clone();

    // blocking thread: polls pressed keys at a fixed interval and emits changes
    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();

        let mut prev: HashSet<Keycode> = HashSet::new();
        let mut was_focused = true;

        loop {
            // stop requested by async side
            if stop_flag_bg.load(Ordering::Relaxed) {
                let _ = tx.send(None);
                break;
            }

            std::thread::sleep(Duration::from_millis(TICK));

            // if unfocused, force-release any previously held keys once
            let is_focused = focused_bg.load(Ordering::Relaxed);

            if !is_focused {
                if was_focused {
                    if !prev.is_empty() {
                        let empty: HashSet<Keycode> = HashSet::new();
                        let _ = tx.send(Some((empty, prev.clone(), false)));
                        prev.clear();
                    }
                    was_focused = false;
                }
                continue;
            }

            // on refocus, resync baseline without triggering a flood of note-on events
            if !was_focused {
                prev = device_state.get_keys().into_iter().collect();
                was_focused = true;
                continue;
            }

            // normal focused polling -> compute set difference vs previous snapshot
            let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

            if now.contains(&Keycode::Escape)
                || (now.contains(&Keycode::C) && now.contains(&Keycode::LControl))
            {
                let _ = tx.send(None);
                break;
            }

            // only emit when something changed; also compute rising edge for 'B'
            if now != prev {
                let toggle_b = now.contains(&Keycode::B) && !prev.contains(&Keycode::B);
                let _ = tx.send(Some((now.clone(), prev.clone(), toggle_b)));
                prev = now;
            }
        }
    });

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    // async event loop: keyboard deltas + UI commands + shutdown.
    loop {
        tokio::select! {
            // OS-level interrupt
            _ = &mut ctrl_c => break,

            // application-level shutdown flag
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }

            // key delta messages from polling thread
            msg = rx.recv() => {
                match msg {
                    Some(Some((now, prev, toggle_b))) => {
                        // edge-trigger waveform toggle (B).
                        if toggle_b {
                            cycle_patch(&mut rt);
                            publish_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play_state, &rt).await;
                        }

                        // newly pressed keys = note-on
                        for k in now.difference(&prev) {
                            if *k == Keycode::B { continue; }
                            play_note(&mut play_state, &rt, *k).await;
                        }

                        // released keys = note-off
                        for k in prev.difference(&now) {
                            if *k == Keycode::B { continue; }
                            play_state.stop_note(*k);
                        }
                    }
                    // none means "stop polling / exit"
                    Some(None) | None => break,
                }
            }

            // UI commands (volume/mute/source) coming from AudioHandle
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break; };

                match cmd {
                    audio_system::AudioCommand::SetVolume(v) => {
                        rt.volume = v.clamp(0.0, 2.0);
                        play_state.set_all_volume(rt.volume);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    audio_system::AudioCommand::SetMuted(m) => {
                        rt.muted = m;
                        play_state.set_all_muted(rt.muted);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    audio_system::AudioCommand::TogglePatch(patches) => {
                        if !patches.is_empty() {
                            rt.avaliable_patches = patches;
                            rt.toggle_index = 0;
                            rt.current_patch = basic_source(BasicKind::Sine); // Reset to first
                            publish_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play_state, &rt).await;
                        }
                    }
                    audio_system::AudioCommand::SetPatch(patch) => {
                        rt.current_patch = patch;
                        publish_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                }
            }
        }
    }

    // shutdown: stop poll thread, kill all notes, wait for thread join
    stop_flag.store(true, Ordering::Relaxed);
    play_state.stop_all();
    let _ = poll_handle.await;
    Ok(())
}
