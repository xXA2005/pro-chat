use futures::{StreamExt, SinkExt};
use tokio::{net::TcpListener, sync::broadcast};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tokio::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

type Tx = broadcast::Sender<Message>;

async fn handle_connection(peer: TcpStream, tx: Tx, clients: Arc<Mutex<HashMap<u32, Tx>>>, client_id: u32) {
    let ws_stream = accept_async(peer)
        .await
        .unwrap();

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let mut rx = tx.subscribe();

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(msg.clone()).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = ws_receiver.next().await {
        if let Ok(msg) = msg {
            let clients_guard = clients.lock().unwrap();
            for (id, client_tx) in clients_guard.iter() {
                if *id != client_id {
                    let _ = client_tx.send(msg.clone());
                }
            }
        }
    }

    clients.lock().unwrap().remove(&client_id);
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("running on: {}", addr);

    let (tx, _) = broadcast::channel(100);
    let clients = Arc::new(Mutex::new(HashMap::new()));
    let mut client_id_counter = 0;

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        let client_id = {
            let mut guard = clients.lock().unwrap();
            client_id_counter += 1;
            guard.insert(client_id_counter, tx.clone());
            client_id_counter
        };

        tokio::spawn(async move {
            handle_connection(stream, tx, clients, client_id).await;
        });
    }
}
