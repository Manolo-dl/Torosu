use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager};
use tokio_tungstenite::connect_async;
use tokio::select;

use crate::{AppConfigState, TosuListenerState};

pub async fn start_tosu_listener(app: AppHandle, url: String, shutdown: Arc<AtomicBool>) {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            log::info!("tosu listener shutting down");
            break;
        }

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                log::info!("Connected to TOSU WebSocket server");
                let (_write, mut read) = ws_stream.split();

                loop {
                    if shutdown.load(Ordering::SeqCst) {
                        log::info!("tosu listener shutting down (mid-read)");
                        return;
                    }

                    select! {
                        msg = read.next() => {
                            match msg {
                                Some(Ok(msg)) if msg.is_text() => {
                                    let text = msg.into_text().unwrap_or_default();
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                        let _ = app.emit("tosu-data", json);
                                    }
                                }
                                Some(Ok(_)) => {}
                                Some(Err(e)) => {
                                    log::error!("Error receiving message: {}", e);
                                    break;
                                }
                                None => break, // stream cerrado
                            }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                            // solo para re-chequear el shutdown flag periódicamente
                        }
                    }
                }

                log::info!("Disconnected from TOSU WebSocket server, retrying...");
            }
            Err(e) => {
                log::error!("Failed to connect to TOSU: {}", e);
            }
        }

        // Sleep interrumpible: revisa el flag cada 100ms en vez de dormir 3s de una
        for _ in 0..30 {
            if shutdown.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

#[tauri::command]
pub async fn connect_tosu(app: AppHandle, state: tauri::State<'_, TosuListenerState>) -> Result<(), String> {

    let config_state = app.state::<AppConfigState>();
    let tosu_config = config_state.config.lock().unwrap().clone().tosu;


    let url = format!("ws://{}:{}/websocket/v2", tosu_config.ip, tosu_config.port);

    if state.started.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let shutdown = state.shutdown.clone();
    tauri::async_runtime::spawn(async move {
        start_tosu_listener(app, url, shutdown).await;
    });

    Ok(())
}