use super::*;
use crate::support::memory;
use aes_gcm::KeyInit;

fn cipher() -> Aes256Gcm {
    Aes256Gcm::new_from_slice(&[3; 32]).expect("key")
}

fn configured() -> Settings {
    Settings {
        smtp: Smtp {
            host: "smtp.example.com".to_owned(),
            port: 587,
            security: Security::Starttls,
            username: "nas@example.com".to_owned(),
            password: Some("secret".to_owned()),
            from: "nas@example.com".to_owned(),
            recipients: vec!["me@example.com".to_owned()],
        },
        ..Settings::default()
    }
}

#[test]
fn the_defaults_send_nothing() {
    let settings = Settings::default();
    assert!(!settings.configured());
    assert_eq!(settings.smtp.port, 465);
    assert_eq!(settings.system.level, Level::Security);
    assert!(!settings.access.refused && !settings.ui.available && !settings.system.available);
    assert!(settings.validate().is_ok());
}

#[test]
fn only_plain_addresses_are_accepted() {
    for address in ["me@example.com", "a.b+c@mail.example.org"] {
        assert!(is_address(address), "{address}");
    }
    for address in [
        "",
        "me",
        "me@local",
        "@example.com",
        "me@.com",
        "Me <me@example.com>",
        "a@b@c.com",
        "me@example.com\r\nBcc: x@y.z",
    ] {
        assert!(!is_address(address), "{address}");
    }
}

#[test]
fn every_field_is_checked() {
    assert!(configured().validate().is_ok());
    let broken = |change: fn(&mut Settings)| {
        let mut settings = configured();
        change(&mut settings);
        settings.validate()
    };
    assert!(broken(|s| s.smtp.host = "smtp example".to_owned()).is_err());
    assert!(broken(|s| s.smtp.port = 0).is_err());
    assert!(broken(|s| s.smtp.username = "a\nb".to_owned()).is_err());
    assert!(broken(|s| s.smtp.password = Some("a\rb".to_owned())).is_err());
    assert!(broken(|s| s.smtp.from = "nas".to_owned()).is_err());
    assert!(broken(|s| s.smtp.recipients = vec!["x".to_owned()]).is_err());
    assert!(broken(|s| s.smtp.recipients = vec!["me@example.com".to_owned(); 11]).is_err());
}

#[tokio::test]
async fn the_password_is_stored_encrypted_kept_on_a_save_without_it_and_never_shown() {
    let db = memory().await;
    let cipher = cipher();
    assert_eq!(
        load(&db, &cipher).await.expect("defaults"),
        Settings::default()
    );

    save(&db, &cipher, configured()).await.expect("save");
    let (stored,): (String,) = sqlx::query_as("SELECT data FROM settings")
        .fetch_one(&db)
        .await
        .expect("row");
    assert!(!stored.contains("\"secret\""));

    let mut changed = configured();
    changed.smtp.password = None;
    changed.access.refused = true;
    save(&db, &cipher, changed).await.expect("save again");
    let loaded = load(&db, &cipher).await.expect("load");
    assert_eq!(loaded.smtp.password.as_deref(), Some("secret"));
    assert!(loaded.access.refused);

    let shown = serde_json::to_value(view(loaded)).expect("json");
    assert_eq!(shown["passwordSet"], true);
    assert_eq!(shown["configured"], true);
    assert!(shown["smtp"].get("password").is_none());

    let mut cleared = configured();
    cleared.smtp.password = Some(String::new());
    save(&db, &cipher, cleared).await.expect("clear");
    let shown = view(load(&db, &cipher).await.expect("load"));
    assert!(!shown.password_set);
}

#[test]
fn stays_in_background_only_with_an_account_and_a_notification_on() {
    let mut settings = configured();
    assert!(!settings.notifies());
    assert!(!settings.in_background());
    for switch in [
        |settings: &mut Settings| settings.access.refused = true,
        |settings: &mut Settings| settings.ui.available = true,
        |settings: &mut Settings| settings.ui.updated = true,
        |settings: &mut Settings| settings.ui.failed = true,
        |settings: &mut Settings| settings.apps.available = true,
        |settings: &mut Settings| settings.system.available = true,
    ] {
        let mut on = configured();
        switch(&mut on);
        assert!(on.in_background());
    }
    settings.system.available = true;
    settings.smtp.recipients.clear();
    assert!(settings.notifies() && !settings.in_background());
}
