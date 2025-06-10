mod ws_server;

#[tokio::main]
async fn main() {
    ws_server::run_ws_server().await;
    println!("Hello, world!");
}
