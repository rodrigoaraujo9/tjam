mod key;
mod play;
mod config;
mod state;
mod audio_source;
mod ui;
use play::run_audio;

use ratatui::style::Stylize;
use ratatui::widgets::{Block, Paragraph};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // run_audio().await;
    ratatui::run(|terminal| {
            terminal.draw(|frame| {
                let block = Block::bordered().title("tjam");
                let greeting = Paragraph::new("welcome to tjam!")
                    .centered()
                    .yellow()
                    .block(block).dark_gray();
                frame.render_widget(greeting, frame.area());
            })?;
            std::thread::sleep(std::time::Duration::from_secs(5));
            Ok(())
        })
}
