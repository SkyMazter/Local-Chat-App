use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

type Tx = mpsc::UnboundedSender<Message>; // Sender type alias
type PeerList = Arc<Mutex<HashMap<std::net::SocketAddr, Tx>>>; // Shared list of peers

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
}

pub async fn run_ws_server() {
    let addr = "0.0.0.0:9001"; // Listen on all interfaces, port 9001

    //Start a TCP listener and bind it to port 9001
    let try_socket = TcpListener::bind(&addr).await;
    println!("Listening on: {}", addr);

    let listener = match try_socket {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind: {}", e);
            return;
        }
    };

    println!("Succesfully listening on port 9001!");

    let peers: PeerList = Arc::new(Mutex::new(HashMap::new()));

    //loop still runs while the listener does not throw an error, if the listener is ok, it will return variables stream and _addr, which are usable in the loop.
    while let Ok((stream, soc_ddr)) = listener.accept().await {
        let peers = peers.clone();

        //spawn new instance of connection, each connection gets its own thread
        tokio::spawn(async move {
            println!("Incoming connection from {}", soc_ddr);

            //this is where the websocket handshake happens, If successful, it returns a `ws_stream`, which we use for messaging.
            //stores the ws connection in "ws_stream"
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket Handshake Failed: {}", e);
                    return;
                }
            };

            println!("New Websocket Connection established");

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            // Create a channel to send messages *to this client* (tx = transmiter, rx = reciever)
            let (tx, mut rx) = mpsc::unbounded_channel();
            // Add this peer's sender to the list
            peers.lock().unwrap().insert(soc_ddr, tx);

            // Task 1: handle messages *from a client*
            let recv_task = {
                let peers_clone = peers.clone(); // Clone for the recv_task
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        if msg.is_binary() || msg.is_text() {
                            match serde_json::from_str::<ChatMessage>(&msg.to_string()) {
                                Ok(mut chat_message) => {
                                    chat_message.timestamp = Utc::now();
                                    let serialized_message = serde_json::to_string(&chat_message)
                                        .expect("Failed to serialize ChatMessage");

                                    let peers = peers_clone.lock().unwrap(); // Use peers_for_recv_task
                                    for (&peer_addr, peer) in peers.iter() {
                                        let _ =
                                            peer.send(Message::Text(serialized_message.clone()));

                                        println!("Broadcasted Message form {}", peer_addr);
                                    }
                                }

                                Err(e) => eprintln!("Failed to parse message from: {}", e),
                            }
                        }
                    }
                })
            };

            // Task 2: handle messages *to a client*
            let send_task = tokio::spawn(async move {
                //while we still have good messages in the RX channel ready to be sent, send them using ws_sender, await its response, if it returns an error then break
                while let Some(msg) = rx.recv().await {
                    if ws_sender.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            // Wait for either to finish
            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }

            // On disconnect: remove the sender from the peer list
            //“Go through each peer in the list (peers), call it p. For each one, compare it to the tx (which belongs to the client that disconnected). If it is the same, remove it. If it's different, keep it.”
            peers.lock().unwrap().remove(&soc_ddr); // Remove peer on disconnect

            println!("❌ Connection closed");
        });
    }
}
