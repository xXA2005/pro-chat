use futures::{SinkExt, StreamExt};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey, Pkcs1v15Encrypt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::error::Error;
use std::io::Write;
use tokio::sync::mpsc;
use base64::{encode, decode}; // deprectated ahh
use std::fs;

#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8080"; // change this manually tbh

    let private_key_path = input("your private key path: ");
    let private_key = load_private_key(&private_key_path).unwrap();

    let public_key_path = input("your friend public key path: ");
    let public_key = load_public_key(&public_key_path).unwrap();

    let (ws_stream, _) = connect_async(url).await.unwrap();
    println!("connected");

    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            Some(message) = rx.recv() => {
                let encrypted_message = encrypt_message(&message, &public_key).unwrap();

                if write.send(Message::Text(encrypted_message.clone())).await.is_err() {
                    println!("failed to send msg");
                    return;
                }
            },

            Some(Ok(message)) = read.next() => {
                if let Message::Text(text) = message {
                    if let Ok(decrypted_message) = decrypt_message(&text, &private_key) {
                        println!("fr: {}", decrypted_message);
                    }
                }
            }
        }
    }
}


fn input(prompt: &str) -> String {
    let mut path = String::new();
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut path).unwrap();
    path.trim().to_string()
}

fn load_private_key(path: &str) -> Result<RsaPrivateKey, Box<dyn Error>> {
    let private_pem = fs::read_to_string(path)?;
    let private_key = RsaPrivateKey::from_pkcs1_pem(&private_pem)?;
    Ok(private_key)
}

fn load_public_key(path: &str) -> Result<RsaPublicKey, Box<dyn Error>> {
    let public_pem = fs::read_to_string(path)?;
    let public_key = RsaPublicKey::from_pkcs1_pem(&public_pem)?;
    Ok(public_key)
}

fn encrypt_message(message: &str, public_key: &RsaPublicKey) -> Result<String, Box<dyn Error>> {
    let mut rng = rand::thread_rng();
    let encrypted_data = public_key.encrypt(&mut rng, Pkcs1v15Encrypt, message.as_bytes())?;
    Ok(encode(encrypted_data))
}

fn decrypt_message(encrypted_message: &str, private_key: &RsaPrivateKey) -> Result<String, Box<dyn Error>> {    
    let encrypted_data = decode(encrypted_message)?;
    let decrypted_data = private_key.decrypt(Pkcs1v15Encrypt, &encrypted_data)?;
    Ok(String::from_utf8(decrypted_data)?)
}
