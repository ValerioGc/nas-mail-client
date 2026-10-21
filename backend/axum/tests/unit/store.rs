use super::*;
use std::os::unix::fs::PermissionsExt;

fn directory(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("valid system time")
        .as_nanos();
    let path = env::temp_dir().join(format!("nas-mail-{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

#[test]
fn the_key_is_kept_private_and_reused() {
    let directory = directory("key");
    let database = directory.join("mail.sqlite3");
    let cipher = secret_cipher(&database).expect("create the key");
    let secret = encrypt_secret("smtp-secret", &cipher).expect("encrypt");
    assert!(secret.starts_with("v1:"));
    assert!(!secret.contains("smtp-secret"));

    let again = secret_cipher(&database).expect("read the key");
    assert_eq!(
        decrypt_secret(&secret, &again).expect("decrypt"),
        "smtp-secret"
    );
    let mode = fs::metadata(directory.join("mail.key"))
        .expect("key metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn a_broken_secret_is_refused() {
    let cipher = Aes256Gcm::new_from_slice(&[7; 32]).expect("key");
    assert!(decrypt_secret("plain", &cipher).is_err());
    assert!(decrypt_secret("v1:00", &cipher).is_err());
    assert!(decrypt_secret("v1:zz", &cipher).is_err());
    let other = Aes256Gcm::new_from_slice(&[8; 32]).expect("key");
    let secret = encrypt_secret("x", &other).expect("encrypt");
    assert!(decrypt_secret(&secret, &cipher).is_err());
}

#[test]
fn a_key_of_the_wrong_length_is_refused() {
    let directory = directory("short");
    fs::write(directory.join("mail.key"), [1, 2, 3]).expect("write key");
    assert!(secret_cipher(&directory.join("mail.sqlite3")).is_err());
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[tokio::test]
async fn opens_a_database_in_a_new_directory() {
    let directory = directory("open");
    let (pool, _) = open(&directory.join("nested/mail.sqlite3"))
        .await
        .expect("open");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mails")
        .fetch_one(&pool)
        .await
        .expect("query");
    assert_eq!(count.0, 0);
    pool.close().await;
    fs::remove_dir_all(directory).expect("remove test directory");
}
