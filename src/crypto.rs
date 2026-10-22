use argon2::Argon2;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;
use std::error::Error;
use std::fs;
use std::path::Path;

// import diaryvault
use crate::DiaryVault;

const VAULT_FILE: &str = "diary.vault";
const SALT_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;

// dervies the 256 bit key 
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
    let mut key = [0u8;32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("KDF error: {}", e))?;
    Ok(key)
}

// encrypt
fn encrypt_vault(vault: &DiaryVault, password: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let plaintext = vault.to_bytes()?;

    let mut salt = [0u8; SALT_SIZE];
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password, &salt)?;
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| "Encryption failed")?;

    let mut payload = Vec::with_capacity(SALT_SIZE + NONCE_SIZE + ciphertext.len());
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    Ok(payload)
}

// decrypt
fn decrypt_vault(file_bytes: &[u8], password: &str) -> Result<DiaryVault, Box<dyn Error>> {

    let salt = &file_bytes[..SALT_SIZE];
    let nonce_bytes = &file_bytes[SALT_SIZE..SALT_SIZE + NONCE_SIZE];
    let ciphertext = &file_bytes[SALT_SIZE + NONCE_SIZE..];

    let key_bytes = derive_key(password, salt)?;
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed")?;

    DiaryVault::from_bytes(&plaintext)
}

pub fn load_vault(password: &str) -> Result<DiaryVault, Box<dyn Error>> {
    if Path::new(VAULT_FILE).exists() {
        let encrypted_bytes = fs::read(VAULT_FILE)?;
        decrypt_vault(&encrypted_bytes, password)
    } else {
        Ok(DiaryVault::default())
    }
}

pub fn save_vault(vault: &DiaryVault, password: &str) -> Result<(), Box<dyn Error>> {
    let encrypted_bytes = encrypt_vault(vault, password)?;
    fs::write(VAULT_FILE, encrypted_bytes)?;
    Ok(())
}