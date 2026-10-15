mod events;
mod launcher;
mod mails;
mod message;
mod sender;
mod settings;
mod store;
#[cfg(test)]
#[path = "../../tests/unit/support.rs"]
mod support;

use aes_gcm::Aes256Gcm;
use axum::{
    extract::{Path as UrlPath, Query, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use mails::{Folder, Mail, Outgoing};
use serde::{Deserialize, Serialize};
use serde_json::json;
use settings::Settings;
use sqlx::SqlitePool;
use std::{
    env,
    os::unix::fs::FileTypeExt,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::{UnixListener, UnixStream};

#[derive(Clone)]
struct AppState {
    ui_dir: PathBuf,
    database: SqlitePool,
    cipher: Arc<Aes256Gcm>,
    msmtp: PathBuf,
    launcher: launcher::Launcher,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "No such mail.".to_owned(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        eprintln!("Mail client error: {error}");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Unable to access the mail data".to_owned(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

async fn health() -> impl IntoResponse {
    Json(json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}))
}

#[derive(Serialize)]
struct Status {
    msmtp: bool,
    configured: bool,
    /// A server and a sender: enough for the codes of the account, sent to their own address.
    sender: bool,
}

async fn status(State(state): State<AppState>) -> Result<Json<Status>, ApiError> {
    let settings = settings::load(&state.database, &state.cipher)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(Status {
        msmtp: sender::available(&state.msmtp),
        configured: settings.configured(),
        sender: settings.can_send(),
    }))
}

async fn read_settings(State(state): State<AppState>) -> Result<Json<settings::View>, ApiError> {
    settings::load(&state.database, &state.cipher)
        .await
        .map(|settings| Json(settings::view(settings)))
        .map_err(ApiError::internal)
}

async fn save_settings(
    State(state): State<AppState>,
    Json(wanted): Json<Settings>,
) -> Result<Json<settings::View>, ApiError> {
    wanted.validate().map_err(ApiError::bad_request)?;
    let settings = settings::save(&state.database, &state.cipher, wanted)
        .await
        .map_err(ApiError::internal)?;
    state.launcher.background(settings.in_background()).await;
    Ok(Json(settings::view(settings)))
}

/// Tells the launcher whether the saved settings have mails to send without the window.
async fn declare_background(state: &AppState) {
    if let Ok(settings) = settings::load(&state.database, &state.cipher).await {
        state.launcher.background(settings.in_background()).await;
    }
}

/// Sends a mail with the saved settings and keeps it with its outcome.
async fn send(
    state: &AppState,
    settings: &Settings,
    composed: &events::Composed,
) -> Result<Mail, ApiError> {
    let at_ms = now_ms();
    let outcome = sender::send(
        &state.msmtp,
        &settings.smtp,
        &message::Message {
            from: &settings.smtp.from,
            recipients: &settings.smtp.recipients,
            subject: &composed.subject,
            body: &composed.body,
            at_ms,
        },
    )
    .await;
    mails::record(
        &state.database,
        &Outgoing {
            category: composed.category,
            recipients: &settings.smtp.recipients,
            subject: &composed.subject,
            body: &composed.body,
            at_ms,
        },
        &outcome,
    )
    .await
    .map_err(ApiError::internal)
}

async fn check_connection(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    let settings = settings::load(&state.database, &state.cipher)
        .await
        .map_err(ApiError::internal)?;
    if !settings.configured() {
        return Err(ApiError::bad_request(
            "Configure the SMTP server, the sender and a recipient first.",
        ));
    }
    sender::check_connection(&state.msmtp, &settings.smtp)
        .await
        .map_err(|message| ApiError {
            status: StatusCode::BAD_GATEWAY,
            message,
        })?;
    Ok(StatusCode::NO_CONTENT)
}

/// An event of the launcher: answered at once, mailed in the background. A code of the account is
/// the exception: the user waits for it, so its outcome is the answer.
async fn receive_event(
    State(state): State<AppState>,
    Json(event): Json<events::Event>,
) -> Response {
    if event.key == "accountCode" {
        return match mail_code(&state, &event).await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => error.into_response(),
        };
    }
    tokio::spawn(async move {
        if let Err(error) = mail_event(&state, &event).await {
            eprintln!("Mail for {} not sent: {}", event.key, error.message);
        }
    });
    StatusCode::ACCEPTED.into_response()
}

/// Sends a code of the account to the address it names. It is not kept with the sent mail: whoever
/// opens this application must not read a code meant for the mailbox.
async fn mail_code(state: &AppState, event: &events::Event) -> Result<(), ApiError> {
    let param = |name: &str| event.params.get(name).map_or("", String::as_str);
    let (to, code) = (param("to"), param("code"));
    if !settings::is_address(to) || code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::bad_request(
            "The code or its address is not valid.",
        ));
    }
    let settings = settings::load(&state.database, &state.cipher)
        .await
        .map_err(ApiError::internal)?;
    if !settings.can_send() {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "No SMTP server is configured in the Email app.".to_owned(),
        });
    }
    let (subject, body) = events::code_mail(settings.language, param("purpose"), code)
        .ok_or_else(|| ApiError::bad_request("Unknown purpose of the code."))?;
    let recipients = [to.to_owned()];
    sender::send(
        &state.msmtp,
        &settings.smtp,
        &message::Message {
            from: &settings.smtp.from,
            recipients: &recipients,
            subject: &subject,
            body: &body,
            at_ms: now_ms(),
        },
    )
    .await
    .map_err(|error| ApiError {
        status: StatusCode::BAD_GATEWAY,
        message: error,
    })
}

async fn mail_event(state: &AppState, event: &events::Event) -> Result<(), ApiError> {
    let settings = settings::load(&state.database, &state.cipher)
        .await
        .map_err(ApiError::internal)?;
    if !settings.configured() {
        return Ok(());
    }
    if let Some(composed) = events::compose(event, &settings) {
        send(state, &settings, &composed).await?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct FolderQuery {
    folder: Folder,
}

async fn list_mails(
    State(state): State<AppState>,
    Query(query): Query<FolderQuery>,
) -> Result<Json<Vec<Mail>>, ApiError> {
    mails::list(&state.database, query.folder)
        .await
        .map(Json)
        .map_err(ApiError::internal)
}

fn found(done: bool) -> Result<StatusCode, ApiError> {
    if done {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found())
    }
}

async fn trash_mail(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<i64>,
) -> Result<StatusCode, ApiError> {
    found(
        mails::set_trashed(&state.database, id, Some(now_ms()))
            .await
            .map_err(ApiError::internal)?,
    )
}

async fn restore_mail(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<i64>,
) -> Result<StatusCode, ApiError> {
    found(
        mails::set_trashed(&state.database, id, None)
            .await
            .map_err(ApiError::internal)?,
    )
}

async fn delete_mail(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<i64>,
) -> Result<StatusCode, ApiError> {
    found(
        mails::delete(&state.database, id)
            .await
            .map_err(ApiError::internal)?,
    )
}

async fn empty_trash(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let removed = mails::empty_trash(&state.database)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "removed": removed })))
}

fn ui_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("NAS_UI_DIR") {
        return Ok(PathBuf::from(path));
    }
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let binary_dir = executable.parent().ok_or("Invalid executable path")?;
    let packaged = binary_dir
        .parent()
        .ok_or("Invalid executable path")?
        .join("dist");
    if packaged.join("index.html").is_file() {
        Ok(packaged)
    } else {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/axum"))
    }
}

async fn static_file(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") || path == "/api" || path.starts_with("/internal") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let requested = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };
    let relative = Path::new(requested);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let file = state.ui_dir.join(relative);
    let mime = match file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    // The files of `assets/` carry a hash of their content in the name: the browser keeps them for
    // good. The page that names them is asked for again every time.
    let cache = if requested.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    match tokio::fs::read(file).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache)],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn prepare_socket(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = tokio::fs::symlink_metadata(path).await {
        if !metadata.file_type().is_socket() {
            return Err(format!("Socket path is occupied: {}", path.display()));
        }
        match UnixStream::connect(path).await {
            Ok(_) => return Err(format!("Socket is already listening: {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
            Err(error) => return Err(format!("Unable to check socket: {error}")),
        }
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/settings", get(read_settings).put(save_settings))
        .route("/api/test", post(check_connection))
        .route("/api/mails", get(list_mails))
        .route("/api/mails/{id}", delete(delete_mail))
        .route("/api/mails/{id}/trash", post(trash_mail))
        .route("/api/mails/{id}/restore", post(restore_mail))
        .route("/api/trash", delete(empty_trash))
        .route("/internal/events", post(receive_event))
        .fallback(get(static_file))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let mut listen = env::var_os("NAS_APP_SOCKET");
    while let Some(arg) = args.next() {
        if arg == "--listen" {
            listen = Some(args.next().ok_or("--listen requires a socket path")?);
        } else {
            return Err(format!("Unrecognized argument: {}", arg.to_string_lossy()).into());
        }
    }
    let path = PathBuf::from(listen.ok_or("Provide --listen or NAS_APP_SOCKET")?);
    if path.as_os_str().is_empty() {
        return Err("Empty socket path".into());
    }
    let (database, cipher) = store::open(&store::database_path()?).await?;
    let state = AppState {
        ui_dir: ui_dir()?,
        database,
        cipher: Arc::new(cipher),
        msmtp: sender::program(),
        launcher: launcher::Launcher::from_env(),
    };
    if !state.ui_dir.join("index.html").is_file() {
        return Err(format!("Frontend not found: {}", state.ui_dir.display()).into());
    }

    prepare_socket(&path).await?;
    let listener = UnixListener::bind(&path)?;
    println!("nas-mail-client listening on {}", path.display());
    tokio::spawn({
        let state = state.clone();
        async move { declare_background(&state).await }
    });
    let result = axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown())
        .await;
    if let Err(error) = tokio::fs::remove_file(&path).await {
        eprintln!("Unable to remove socket: {error}");
    }
    result?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/main.rs"]
mod tests;
