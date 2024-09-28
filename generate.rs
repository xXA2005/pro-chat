use rsa::pkcs8::LineEnding;
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::fs;
use std::io::Write;
use std::path::Path;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs1::EncodeRsaPrivateKey;

fn main() {
    let folder_name = input("Enter the folder name to store keys: ");
    generate_keys(&folder_name).unwrap();
}

fn input(prompt: &str) -> String {
    let mut path = String::new();
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut path).unwrap();
    path.trim().to_string()
}

fn generate_keys(user: &str) -> Result<(), Box<dyn std::error::Error>> {
    let folder_path = Path::new(user);
    fs::create_dir_all(folder_path)?;
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 4096)?;
    let public_key = RsaPublicKey::from(&private_key);
    private_key.write_pkcs1_pem_file(&folder_path.join("private_key.pem"), LineEnding::LF).unwrap();
    public_key.write_pkcs1_pem_file(&folder_path.join("public_key.pem"), LineEnding::LF).unwrap();
    Ok(())
}
