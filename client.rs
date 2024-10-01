use futures::{SinkExt, StreamExt};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey, Pkcs1v15Encrypt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::error::Error;
use std::io::Write;
use std::path::Path;
use tokio::sync::mpsc;
use base64::{encode, decode};
use sqlx::SqlitePool;
use sqlx::Row;
use std::fs::{self, File};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "ws://127.0.0.1:8080"; // change this manually tbh

    let private_key_path = input("your private key path: ");
    let private_key = load_private_key(&private_key_path)?;

    let public_key_path = input("your friend public key path: ");
    let public_key = load_public_key(&public_key_path)?;

    let pool = setup_database().await?;

    load_and_print_messages(&pool).await?;

    let (ws_stream, _) = connect_async(url).await?;
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
                let encrypted_message = encrypt_message(&message, &public_key)?;

                if write.send(Message::Text(encrypted_message.clone())).await.is_err() {
                    println!("failed to send msg");
                    return Ok(());
                }

                save_message(&pool, "sent", &message).await?;
            },

            Some(Ok(message)) = read.next() => {
                if let Message::Text(text) = message {
                    if let Ok(decrypted_message) = decrypt_message(&text, &private_key) {
                        println!("fr: {}", decrypted_message);
                        save_message(&pool, "received", &decrypted_message).await?;
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


async fn setup_database() -> Result<SqlitePool, Box<dyn Error>> {
    let db_path = "db/messages.db";

    let dir = Path::new(db_path).parent().unwrap();
    fs::create_dir_all(dir)?;

    if !Path::new(db_path).exists() {
        File::create(db_path)?;
        println!("Database file created at: {}", db_path);
    }

    let pool = SqlitePool::connect(&format!("sqlite://{}", db_path)).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
             id INTEGER PRIMARY KEY,
             direction TEXT NOT NULL,
             message TEXT NOT NULL
         )"
    ).execute(&pool).await?;

    println!("Database setup complete.");
    Ok(pool)
}


async fn save_message(pool: &SqlitePool, direction: &str, message: &str) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        "INSERT INTO messages (direction, message) VALUES (?1, ?2)"
    )
    .bind(direction)
    .bind(message)
    .execute(pool)
    .await?;

    Ok(())
}

async fn load_and_print_messages(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    let rows = sqlx::query(
        "SELECT direction, message FROM messages"
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let direction: String = row.get(0);
        let msg: String = row.get(1);
        println!("{}: {}", direction, msg);
    }
    
    Ok(())
}
