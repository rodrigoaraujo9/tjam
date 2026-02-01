use device_query::{DeviceQuery, DeviceState, Keycode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::collections::HashMap;
use std::time::Duration;
use rodio::stream::OutputStreamBuilder;
use rodio::Sink;
use rodio::source::{SineWave, Source};

use crate::key::Key;
use crate::input::keycode_to_char;


pub struct Play {
    _stream: rodio::OutputStream,
    active_sinks: HashMap<char, Sink>,
}

impl Play {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self {
            _stream: stream,
            active_sinks: HashMap::new(),
        })
    }

    pub fn play_note(&mut self, c: char) {
        if self.active_sinks.contains_key(&c) {
            return;
        }

        if let Some(key) = Key::from_keycode(c) {
            let freq = key.frequency();

            let sink = Sink::connect_new(&self._stream.mixer());
            let source = SineWave::new(freq)
                .take_duration(Duration::from_secs(3600))
                .amplify(0.20);
            sink.append(source);
            self.active_sinks.insert(c, sink);
        }
    }

    pub fn stop_note(&mut self, c: char) {
        if let Some(sink) = self.active_sinks.remove(&c) {
            sink.stop();
        }
    }

    pub fn stop_all(&mut self) {
        for (_, sink) in self.active_sinks.drain() {
            sink.stop();
        }
    }
}

impl Drop for Play {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut audio = Play::new()?;
    let device_state = DeviceState::new();
    let mut prev_keys: Vec<Keycode> = vec![];

    enable_raw_mode()?;

    loop {
        let keys = device_state.get_keys();

        if keys.contains(&Keycode::Escape) ||
           (keys.contains(&Keycode::C) && keys.contains(&Keycode::LControl)) {
            break;
        }

        for keycode in &keys {
            if !prev_keys.contains(keycode) {
                if let Some(c) = keycode_to_char(keycode) {
                    audio.play_note(c);
                }
            }
        }

        for keycode in &prev_keys {
            if !keys.contains(keycode) {
                if let Some(c) = keycode_to_char(keycode) {
                    audio.stop_note(c);
                }
            }
        }

        prev_keys = keys;
    }

    disable_raw_mode()?;

    Ok(())
}
