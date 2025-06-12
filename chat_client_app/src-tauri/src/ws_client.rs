use futures::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tungstenite::client::IntoClientRequest;

pub static WS_TX: OnceCell<UnboundedSender<Message>> = OnceCell::new();

pub async fn run_ws_client(app_handle: AppHandle) {
    let url = "ws://localhost:9001".into_client_request().unwrap();
    let (stream, _response) = match connect_async(url).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("WebSocket Handshake Failed: {}", e);
            return;
        }
    };

    println!("New Websocket Connection established");

    let (mut ws_sender, mut ws_reciever) = stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    WS_TX.set(tx).unwrap();
    // Spawn task to receive messages from server
    let app_handle_clone = app_handle.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(response)) = ws_reciever.next().await {
            println!("Received from server: {}", response);
            let _ = app_handle_clone
                .emit("ws-message", response.to_string())
                .unwrap();
        }
    });

    // Spawn task to send messages to server
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(text) = msg.into_text() {
                if ws_sender.send(Message::Text(text)).await.is_err() {
                    eprintln!("WebSocket send failed.");
                    break;
                }
            } else {
                eprintln!("Received non-text message, skipping.");
            }
        }
    });

    // Await both tasks to complete
    let _ = tokio::join!(recv_task, send_task);
}
