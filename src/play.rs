use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use rodio::stream::{OutputStreamBuilder, OutputStream};
use rodio::Sink;

use tokio::signal::ctrl_c;
use tokio::task;
use tokio::sync::Notify;
use std::sync::Arc;

use crate::config::TICK;
use crate::key::Key;
use crate::state;

pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Sink>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self {
            stream,
            active_sinks: HashMap::new(),
        })
    }
}

pub async fn play_note(play_state: &mut PlayState, keycode: Keycode) {
    if play_state.active_sinks.contains_key(&keycode) {
        return;
    }

    let Some(key) = Key::from_keycode(keycode) else { return; };
    let freq = key.frequency();

    let sink = Sink::connect_new(&play_state.stream.mixer());

    let audio_state = state::get_state().await;

    let audio_source = {
        let src_guard = audio_state.source.read().await;
        src_guard.create_source(freq)
    };

    let volume = *audio_state.volume.read().await;
    sink.set_volume(volume);

    if *audio_state.muted.read().await {
        sink.pause();
    }

    sink.append(audio_source);
    play_state.active_sinks.insert(keycode, sink);
}

pub fn stop_note(play_state: &mut PlayState, keycode: Keycode) {
    if let Some(sink) = play_state.active_sinks.remove(&keycode) {
        sink.stop();
    }
}

pub fn stop_all(play_state: &mut PlayState) {
    for (_, sink) in play_state.active_sinks.drain() {
        sink.stop();
    }
}

pub async fn sync_volume(play_state: &mut PlayState) {
    let audio_state = state::get_state().await;
    let volume = *audio_state.volume.read().await;
    for sink in play_state.active_sinks.values_mut() {
        sink.set_volume(volume);
    }
}

pub async fn sync_muted_state(play_state: &mut PlayState) {
    let audio_state = state::get_state().await;
    let muted = *audio_state.muted.read().await;

    if muted {
        for sink in play_state.active_sinks.values_mut() {
            sink.pause();
        }
    } else {
        for sink in play_state.active_sinks.values_mut() {
            sink.play();
        }
    }
}

pub async fn restart_active_notes(play_state: &mut PlayState) {
    let keys: Vec<Keycode> = play_state.active_sinks.keys().copied().collect();
    for k in keys {
        stop_note(play_state, k);
        play_note(play_state, k).await;
    }
}

pub async fn run_audio() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut prev: HashSet<Keycode> = HashSet::new();

        loop {
            std::thread::sleep(Duration::from_millis(TICK));
            let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

            if now.contains(&Keycode::Escape)
                || (now.contains(&Keycode::C) && now.contains(&Keycode::LControl))
            {
                let _ = tx.send(None);
                break;
            }

            if now != prev {
                let toggle_b = now.contains(&Keycode::B) && !prev.contains(&Keycode::B);

                if tx.send(Some((now.clone(), prev.clone(), toggle_b))).is_err() {
                    break;
                }
                prev = now;
            }
        }
    });

    let mut play_state = PlayState::new()?;

    let audio_state = state::get_state().await;
    let volume_notify: Arc<Notify> = audio_state.volume_notify.clone();
    let mute_notify: Arc<Notify> = audio_state.mute_notify.clone();
    let source_notify: Arc<Notify> = audio_state.source_notify.clone();

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => break,

            msg = rx.recv() => {
                match msg {
                    Some(Some((now, prev, toggle_b))) => {
                        if toggle_b {
                            let audio_state = state::get_state().await;
                            audio_state.rotate_source().await;
                            restart_active_notes(&mut play_state).await;
                        }

                        for k in now.difference(&prev) {
                            if *k == Keycode::B { continue; }
                            play_note(&mut play_state, *k).await;
                        }
                        for k in prev.difference(&now) {
                            if *k == Keycode::B { continue; }
                            stop_note(&mut play_state, *k);
                        }
                    }
                    Some(None) | None => break,
                }
            }

            _ = volume_notify.notified() => {
                sync_volume(&mut play_state).await;
            }

            _ = mute_notify.notified() => {
                sync_muted_state(&mut play_state).await;
            }

            _ = source_notify.notified() => {
                restart_active_notes(&mut play_state).await;
            }
        }
    }

    stop_all(&mut play_state);
    let _ = poll_handle.await;

    Ok(())
}
