mod key;
mod input;
mod play;
use play::{run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
