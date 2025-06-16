// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod ws_client;
use tungstenite::Message;

use crate::ws_client::ChatMessage;
use chrono::Utc;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn send_message(msg: String, id: String) {
    if let Some(tx) = ws_client::WS_TX.get() {
        let outgoing_message = ChatMessage {
            user_id: id.clone(),
            timestamp: Utc::now(),
            content: msg.clone(),
        };

        let formatted_message =
            serde_json::to_string(&outgoing_message).expect("Failes to serialize outgoing message");

        if let Err(e) = tx.send(Message::Text(formatted_message)) {
            eprint!("Failed to send message to server: {}", e);
        }
    } else {
        eprintln!("WebSocket connection is not initialized yet.");
    }
}

#[tauri::command]
async fn server_check() {
    println!("Hi");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, send_message, server_check])
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                ws_client::run_ws_client(handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
