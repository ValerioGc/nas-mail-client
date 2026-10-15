use super::*;
use crate::{settings::Smtp, support::memory};
use aes_gcm::KeyInit;
use std::os::unix::fs::PermissionsExt;

async fn state(msmtp: PathBuf) -> AppState {
    AppState {
        ui_dir: PathBuf::new(),
        database: memory().await,
        cipher: Arc::new(Aes256Gcm::new_from_slice(&[9; 32]).expect("key")),
        msmtp,
        launcher: launcher::Launcher::new(None),
    }
}

fn configured() -> Settings {
    Settings {
        smtp: Smtp {
            host: "smtp.example.com".to_owned(),
            from: "nas@example.com".to_owned(),
            recipients: vec!["me@example.com".to_owned()],
            ..Smtp::default()
        },
        ..Settings::default()
    }
}

/// A msmtp that accepts every mail. Written once: tests run in parallel, and rewriting a program
/// while another test runs it fails with `Text file busy`.
fn accepting() -> PathBuf {
    static PROGRAM: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PROGRAM
        .get_or_init(|| {
            let directory = env::temp_dir().join(format!("nas-mail-main-{}", std::process::id()));
            std::fs::create_dir_all(&directory).expect("directory");
            let program = directory.join("msmtp");
            std::fs::write(&program, "#!/bin/sh\ncat > /dev/null\n").expect("program");
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
            program
        })
        .clone()
}

#[tokio::test]
async fn settings_are_validated_saved_and_shown_without_the_password() {
    let state = state(PathBuf::from("/no/msmtp")).await;
    let mut wrong = configured();
    wrong.smtp.from = "nobody".to_owned();
    let refused = save_settings(State(state.clone()), Json(wrong))
        .await
        .expect_err("invalid");
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);

    let mut wanted = configured();
    wanted.smtp.password = Some("secret".to_owned());
    let Json(saved) = save_settings(State(state.clone()), Json(wanted))
        .await
        .expect("saved");
    assert!(saved.password_set && saved.configured);
    let Json(read) = read_settings(State(state.clone())).await.expect("read");
    assert_eq!(read.settings.smtp.host, "smtp.example.com");

    let Json(status) = status(State(state)).await.expect("status");
    assert!(!status.msmtp && status.configured && status.sender);
}

#[tokio::test]
async fn a_connection_check_needs_settings_and_never_records_a_mail() {
    let state = state(PathBuf::from("/no/msmtp")).await;
    let refused = check_connection(State(state.clone()))
        .await
        .expect_err("not configured");
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);

    settings::save(&state.database, &state.cipher, configured())
        .await
        .expect("save");
    let failed = check_connection(State(state.clone()))
        .await
        .expect_err("missing msmtp");
    assert_eq!(failed.status, StatusCode::BAD_GATEWAY);
    assert_eq!(failed.message, "msmtp is not installed on the NAS.");
    assert!(mails::list(&state.database, Folder::Sent)
        .await
        .expect("list")
        .is_empty());

    let working = AppState {
        msmtp: accepting(),
        ..state
    };
    assert_eq!(
        check_connection(State(working.clone()))
            .await
            .expect("connected"),
        StatusCode::NO_CONTENT
    );
    assert!(mails::list(&working.database, Folder::Sent)
        .await
        .expect("list")
        .is_empty());
}

#[tokio::test]
async fn an_event_is_mailed_only_when_wanted_and_configured() {
    let state = state(accepting()).await;
    let event = || events::Event {
        key: "signInRefused".to_owned(),
        params: Default::default(),
    };
    mail_event(&state, &event())
        .await
        .expect("nothing configured");
    let mut wanted = configured();
    settings::save(&state.database, &state.cipher, wanted.clone())
        .await
        .expect("save");
    mail_event(&state, &event()).await.expect("not wanted");
    assert!(mails::list(&state.database, Folder::Sent)
        .await
        .expect("list")
        .is_empty());

    wanted.access.refused = true;
    settings::save(&state.database, &state.cipher, wanted)
        .await
        .expect("save");
    mail_event(&state, &event()).await.expect("mailed");
    let sent = mails::list(&state.database, Folder::Sent)
        .await
        .expect("list");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].category, "access");
    assert_eq!(
        receive_event(State(state), Json(event())).await.status(),
        StatusCode::ACCEPTED
    );
}

#[tokio::test]
async fn the_folders_answer_for_their_mails_only() {
    let state = state(accepting()).await;
    let recipients = ["me@example.com".to_owned()];
    let mail = mails::record(
        &state.database,
        &Outgoing {
            category: mails::Category::Test,
            recipients: &recipients,
            subject: "s",
            body: "b",
            at_ms: 1,
        },
        &Ok(()),
    )
    .await
    .expect("record");
    let folder = |folder| Query(FolderQuery { folder });

    assert_eq!(
        trash_mail(State(state.clone()), UrlPath(mail.id))
            .await
            .expect("trash"),
        StatusCode::NO_CONTENT
    );
    let Json(trash) = list_mails(State(state.clone()), folder(Folder::Trash))
        .await
        .expect("trash");
    assert_eq!(trash.len(), 1);
    restore_mail(State(state.clone()), UrlPath(mail.id))
        .await
        .expect("restore");
    assert_eq!(
        delete_mail(State(state.clone()), UrlPath(mail.id))
            .await
            .expect_err("not in trash")
            .status,
        StatusCode::NOT_FOUND
    );
    trash_mail(State(state.clone()), UrlPath(mail.id))
        .await
        .expect("trash");
    let Json(removed) = empty_trash(State(state.clone())).await.expect("empty");
    assert_eq!(removed["removed"], 1);
    assert_eq!(
        restore_mail(State(state), UrlPath(mail.id))
            .await
            .expect_err("gone")
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn the_interface_is_served_but_never_the_api_or_the_launcher_side() {
    let directory = env::temp_dir().join(format!("nas-mail-ui-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("directory");
    std::fs::write(directory.join("index.html"), "<html></html>").expect("index");
    let state = AppState {
        ui_dir: directory.clone(),
        ..state(PathBuf::new()).await
    };
    let answer = |path: &'static str| static_file(State(state.clone()), Uri::from_static(path));
    assert_eq!(answer("/").await.status(), StatusCode::OK);
    std::fs::create_dir_all(directory.join("assets")).expect("assets");
    std::fs::write(directory.join("assets/index-abc.js"), "script").expect("asset");
    assert_eq!(
        answer("/assets/index-abc.js").await.headers()[axum::http::header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(
        answer("/").await.headers()[axum::http::header::CACHE_CONTROL],
        "no-cache"
    );
    assert_eq!(answer("/api/unknown").await.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        answer("/internal/events").await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(answer("/missing.js").await.status(), StatusCode::NOT_FOUND);
    std::fs::remove_dir_all(directory).expect("clean");
}

#[tokio::test]
async fn the_socket_path_is_checked_before_binding() {
    let free = env::temp_dir().join(format!("nas-mail-socket-{}.sock", std::process::id()));
    assert!(prepare_socket(&free).await.is_ok());
    let occupied = env::temp_dir().join(format!("nas-mail-file-{}", std::process::id()));
    std::fs::write(&occupied, "").expect("file");
    assert!(prepare_socket(&occupied).await.is_err());
    std::fs::remove_file(occupied).expect("clean");
}

fn code_event(to: &str, purpose: &str, code: &str) -> events::Event {
    events::Event {
        key: "accountCode".to_owned(),
        params: [("to", to), ("purpose", purpose), ("code", code)]
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
    }
}

#[tokio::test]
async fn a_code_of_the_account_goes_to_its_own_address_at_once_and_is_not_kept() {
    let state = state(accepting()).await;
    let answer = |event| receive_event(State(state.clone()), Json(event));

    let unconfigured = answer(code_event("me@example.com", "verify", "123456")).await;
    assert_eq!(unconfigured.status(), StatusCode::SERVICE_UNAVAILABLE);

    let mut sender_only = configured();
    sender_only.smtp.recipients.clear();
    settings::save(&state.database, &state.cipher, sender_only)
        .await
        .expect("save");
    for invalid in [
        code_event("not an address", "verify", "123456"),
        code_event("me@example.com", "verify", "12345"),
        code_event("me@example.com", "verify", "12345a"),
        code_event("me@example.com", "other", "123456"),
    ] {
        assert_eq!(answer(invalid).await.status(), StatusCode::BAD_REQUEST);
    }
    let sent = answer(code_event("me@example.com", "reset", "123456")).await;
    assert_eq!(sent.status(), StatusCode::NO_CONTENT);
    assert!(mails::list(&state.database, Folder::Sent)
        .await
        .expect("list")
        .is_empty());

    let broken = AppState {
        msmtp: PathBuf::from("/no/msmtp"),
        ..state.clone()
    };
    let failed = receive_event(
        State(broken),
        Json(code_event("me@example.com", "reset", "123456")),
    )
    .await;
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);
}
