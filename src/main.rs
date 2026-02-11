mod key;
mod play;
mod config;
mod state;
mod audio_patch;
mod ui;
mod patches;

use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handle = state::get_handle().await.clone();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let ui_fut = {
        let shutdown_tx = shutdown_tx.clone();
        async move {
            let res = ui::run_ui(handle, shutdown_tx.clone()).await;
            let _ = shutdown_tx.send(true);
            res
        }
    };

    let audio_fut = play::run_audio(shutdown_rx);

    let run_all = async { tokio::try_join!(audio_fut, ui_fut).map(|_| ()) };

    tokio::select! {
        r = run_all => { r?; }
        _ = tokio::signal::ctrl_c() => {
            let _ = shutdown_tx.send(true);
        }
    }

    Ok(())
}
