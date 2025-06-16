use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use serde_json;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tungstenite::client::IntoClientRequest;

// Assuming ChatMessage is defined as above
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
}

pub static WS_TX: OnceCell<UnboundedSender<Message>> = OnceCell::new();

pub async fn run_ws_client(app_handle: AppHandle) {
    let url = "ws://localhost:9001".into_client_request().unwrap();
    let (stream, _response) = match connect_async(url).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("WebSocket Handshake Failed: {}", e);
            let code: i32 = 404;
            if let Err(e) = app_handle.emit("conn_stat", code) {
                eprintln!("Failed to emit conn_stat: {}", e);
            }
            return;
        }
    };

    let code: i32 = 200;
    if let Err(e) = app_handle.emit("conn_stat", code) {
        eprintln!("Failed to emit conn_stat: {}", e);
    }

    println!("New Websocket Connection established");

    let (mut ws_sender, mut ws_reciever) = stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    WS_TX.set(tx).unwrap();
    // Spawn task to receive messages from server
    let app_handle_clone = app_handle.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(response) = ws_reciever.next().await {
            match response {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ChatMessage>(&text) {
                        Ok(chat_message) => {
                            println!("[CLIENT] Received: {:?}", chat_message);
                            // Emit a Tauri event to the frontend
                            app_handle_clone
                                .emit("ws-message", chat_message)
                                .expect("Failed to emit event");
                        }

                        Err(e) => eprintln!("[CLIENT] Failed to parse incoming message: {}", e),
                    }
                }

                Ok(Message::Binary(bytes)) => println!("[CLIENT] Received binary: {:?}", bytes),
                Ok(_) => {} // Ignore other message types for now
                Err(e) => eprintln!("[CLIENT] WebSocket read error: {}", e),
            }
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
