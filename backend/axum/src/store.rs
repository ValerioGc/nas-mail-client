//! SQLite database and the AES-GCM key that encrypts the SMTP password.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Resolves the database path from NAS_DB_PATH, then STATE_DIRECTORY, then backend/data in debug
/// builds.
pub fn database_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("NAS_DB_PATH").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    if let Some(directory) = env::var_os("STATE_DIRECTORY")
        .and_then(|value| value.into_string().ok())
        .and_then(|value| value.split(':').next().map(str::to_owned))
        .filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(directory).join("mail-client.sqlite3"));
    }
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/mail-client.sqlite3"));
    }
    Err("Set NAS_DB_PATH or run under systemd with STATE_DIRECTORY.".to_owned())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Loads the key next to the database, creating it once with mode 0600.
pub fn secret_cipher(database_path: &Path) -> Result<Aes256Gcm, io::Error> {
    let key_path = database_path.with_extension("key");
    let mut options = OpenOptions::new();
    options.write(true).read(true).create_new(true).mode(0o600);

    let key = match options.open(&key_path) {
        Ok(mut file) => {
            let key = Aes256Gcm::generate_key(&mut OsRng);
            file.write_all(&key)?;
            file.sync_all()?;
            key.to_vec()
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !fs::symlink_metadata(&key_path)?.file_type().is_file() {
                return Err(invalid("The mail credential key is not a regular file."));
            }
            fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
            let key = fs::read(&key_path)?;
            if key.len() != 32 {
                return Err(invalid("The mail credential key has an invalid length."));
            }
            key
        }
        Err(error) => return Err(error),
    };
    Aes256Gcm::new_from_slice(&key).map_err(|_| invalid("Unable to use the mail credential key."))
}

pub fn encrypt_secret(secret: &str, cipher: &Aes256Gcm) -> Result<String, String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, secret.as_bytes())
        .map_err(|_| "Unable to encrypt the SMTP password.".to_owned())?;
    let mut payload = nonce.to_vec();
    payload.extend(ciphertext);
    Ok(format!("v1:{}", hex::encode(payload)))
}

pub fn decrypt_secret(secret: &str, cipher: &Aes256Gcm) -> Result<String, String> {
    let encoded = secret
        .strip_prefix("v1:")
        .ok_or("Unsupported encrypted password format.")?;
    let payload = hex::decode(encoded).map_err(|error| error.to_string())?;
    if payload.len() <= 12 {
        return Err("Encrypted password data is incomplete.".to_owned());
    }
    let (nonce, ciphertext) = payload.split_at(12);
    let nonce: [u8; 12] = nonce
        .try_into()
        .map_err(|_| "Encrypted password nonce is invalid.".to_owned())?;
    let plaintext = cipher
        .decrypt(&Nonce::from(nonce), ciphertext)
        .map_err(|_| "Unable to decrypt the SMTP password.".to_owned())?;
    String::from_utf8(plaintext).map_err(|error| error.to_string())
}

pub async fn open(path: &Path) -> Result<(SqlitePool, Aes256Gcm), Box<dyn std::error::Error>> {
    if let Some(directory) = path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
    {
        tokio::fs::create_dir_all(directory).await?;
    }
    let cipher = secret_cipher(path)?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok((pool, cipher))
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
