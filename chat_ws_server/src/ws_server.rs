use futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

type Tx = mpsc::UnboundedSender<Message>; // Sender type alias
type PeerList = Arc<Mutex<Vec<Tx>>>; // Shared list of peers

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

    let peers: PeerList = Arc::new(Mutex::new(Vec::new()));

    //loop still runs while the listener does not throw an error, if the listener is ok, it will return variables stream and _addr, which are usable in the loop.
    while let Ok((stream, _addr)) = listener.accept().await {
        let peers = peers.clone(); // NEW

        tokio::spawn(async move {
            println!("Incoming connection from {}", addr);

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

            // let (mut write, mut read) = ws_stream.split();

            // while let Some(Ok(msg)) = read.next().await {
            //     if msg.is_text() || msg.is_binary() {
            //         println!("Recieved Message: {:?}", msg);

            //         if let Err(e) = write.send(msg).await {
            //             eprintln!("Send error: {}", e);
            //             break;
            //         }
            //     }
            // }

            // println!("❌ Connection closed");

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            // Create a channel to send messages *to this client*
            let (tx, mut rx) = mpsc::unbounded_channel(); 
            peers.lock().unwrap().push(tx.clone()); 

            // Task 1: handle messages *to this client*
            let send_task = tokio::spawn(async move {
                // NEW
                while let Some(msg) = rx.recv().await {
                    if ws_sender.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            // Task 2: handle messages *from this client*
            let recv_task = {
                let peers = peers.clone(); // NEW
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        if msg.is_text() || msg.is_binary() {
                            println!("Received: {:?}", msg);

                            let peers = peers.lock().unwrap();
                            for peer in peers.iter() {
                                let _ = peer.send(msg.clone()); // broadcast
                            }
                        }
                    }
                })
            };

            // Wait for either to finish
            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }

            // On disconnect: remove the sender from the peer list
            peers.lock().unwrap().retain(|p| !p.same_channel(&tx)); // NEW
            println!("❌ Connection closed");
        });
    }
}
