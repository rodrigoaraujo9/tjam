use std::collections::HashMap;
use std::time::Duration;
use rodio::{OutputStreamBuilder, Sink};
use rodio::source::{SineWave, Source};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, Write};
mod key;
use key::Key;

fn main() -> Result<(), Box<dyn std::error::Error>>  {
    let stream_handle = OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");
    let mut active_sinks: HashMap<char, Sink> = HashMap::new();
    println!("press a key:");
    enable_raw_mode()?;
    loop {
        if poll(Duration::from_millis(10))? {
            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    break;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Some(key) = Key::from_keycode(c) {
                        if let Some(old_sink) = active_sinks.remove(&c) {
                            old_sink.stop();
                        }
                        let freq = key.frequency();
                        io::stdout().flush()?;
                        let sink = rodio::Sink::connect_new(&stream_handle.mixer());

                        let source = SineWave::new(freq)
                            .amplify(0.20);

                        sink.append(source);
                        active_sinks.insert(c, sink);
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Release,
                    ..
                }) => {
                    if let Some(sink) = active_sinks.remove(&c) {
                        sink.stop();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    ..
                }) => {
                    break;
                }
                _ => {}
            }
        }
    }
    disable_raw_mode()?;
    Ok(())
}
