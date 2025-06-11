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
            peers.lock().unwrap().push(tx.clone());

            // Task 1: handle messages *from a client*
            let recv_task = {
                let peers = peers.clone();

                //spawns new thread for each connection
                tokio::spawn(async move {
                    //while we still have good messages from the ws_reciever, load them into all the  client's reciever (rx) channel, using our transmitter
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        if msg.is_text() || msg.is_binary() {
                            println!("Received: {:?}", msg);

                            let peers = peers.lock().unwrap();
                            for peer in peers.iter() {
                                let _ = peer.send(msg.clone()); // 👈 THIS is where msg goes into rx
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
            peers
                .lock()
                .unwrap()
                .retain(|p: &mpsc::UnboundedSender<Message>| !p.same_channel(&tx));
            println!("❌ Connection closed");
        });
    }
}
