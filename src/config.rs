use tokio::time::Duration;

//play.rs
pub const TICK: u64 = 10;

//key.rs
pub const BASE_FREQ: f32 = 440.0;
pub const A4_SEMITONES: i32 = 57;
pub const SEMITONES_PER_OCTAVE: i32 = 12;
pub const KEYBOARD_BASE_OCTAVE: i32 = 4;

//audio_source.rs
pub const AMP_DEFAULT:f32 = 1.0;

//patches
pub const SAMPLE_RATE: u32 = 48_000;
pub const ENDLESS: Duration = Duration::from_secs(3600);
