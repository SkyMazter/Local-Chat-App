use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

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

    //loop still runs while the listener does not throw an error, if the listener is ok, it will return variables stream and _addr, which are usable in the loop.
    while let Ok((stream, _addr)) = listener.accept().await {
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

            let (mut write, mut read) = ws_stream.split();

            while let Some(Ok(msg)) = read.next().await {
                if msg.is_text() || msg.is_binary() {
                    println!("Recieved Message: {:?}", msg);

                    if let Err(e) = write.send(msg).await {
                        eprintln!("Send error: {}", e);
                        break;
                    }
                }
            }

            println!("❌ Connection closed");
        });
    }
}
