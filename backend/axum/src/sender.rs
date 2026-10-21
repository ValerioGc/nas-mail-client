//! Mail delivery and SMTP connection checks through msmtp. The temporary account file is private
//! to the process and removed afterwards.

use crate::{
    message::Message,
    settings::{Security, Smtp},
};
use std::{
    env,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};

const SEND_TIMEOUT: Duration = Duration::from_secs(60);
static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns NAS_MAIL_MSMTP, or msmtp from the PATH.
pub fn program() -> PathBuf {
    env::var_os("NAS_MAIL_MSMTP")
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from("msmtp"), PathBuf::from)
}

/// Checks that the program exists, as a path or as a name in the PATH.
pub fn available(program: &Path) -> bool {
    if program.components().count() > 1 {
        return program.is_file();
    }
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

/// Quotes a msmtp configuration value, escaping quotes and backslashes.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// Builds the msmtp account for the settings.
pub fn configuration(smtp: &Smtp) -> String {
    let mut text = format!(
        "account nas\nhost {}\nport {}\nfrom {}\ntls {}\ntls_starttls {}\ntls_trust_file system\n",
        quoted(&smtp.host),
        smtp.port,
        quoted(&smtp.from),
        on_off(smtp.security != Security::None),
        on_off(smtp.security == Security::Starttls),
    );
    if smtp.username.is_empty() {
        text.push_str("auth off\n");
    } else {
        text.push_str(&format!(
            "auth on\nuser {}\npassword {}\n",
            quoted(&smtp.username),
            quoted(smtp.password.as_deref().unwrap_or_default())
        ));
    }
    text.push_str("account default : nas\n");
    text
}

async fn write_private(path: &Path, text: &str) -> Result<(), String> {
    let path = path.to_owned();
    let text = text.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(text.as_bytes())
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| format!("Unable to write the msmtp account: {error}"))
}

/// Hands the mail to msmtp and waits for the result.
pub async fn send(program: &Path, smtp: &Smtp, message: &Message<'_>) -> Result<(), String> {
    execute(program, smtp, Some(message)).await
}

/// Checks the SMTP and TLS connection without sending a message.
pub async fn check_connection(program: &Path, smtp: &Smtp) -> Result<(), String> {
    execute(program, smtp, None).await
}

async fn execute(program: &Path, smtp: &Smtp, message: Option<&Message<'_>>) -> Result<(), String> {
    if !available(program) {
        return Err("msmtp is not installed on the NAS.".to_owned());
    }
    let account = env::temp_dir().join(format!(
        "nas-mail-{}-{}.msmtprc",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    write_private(&account, &configuration(smtp)).await?;
    let result = run(program, &account, message).await;
    let _ = tokio::fs::remove_file(&account).await;
    result
}

async fn run(program: &Path, account: &Path, message: Option<&Message<'_>>) -> Result<(), String> {
    let mut child = Command::new(program)
        .arg("-C")
        .arg(account)
        .arg(if message.is_some() {
            "-t"
        } else {
            "--serverinfo"
        })
        .stdin(if message.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Unable to start msmtp: {error}"))?;
    if let (Some(message), Some(mut input)) = (message, child.stdin.take()) {
        input
            .write_all(message.render().as_bytes())
            .await
            .map_err(|error| format!("Unable to hand the mail to msmtp: {error}"))?;
    }
    let output = tokio::time::timeout(SEND_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "The SMTP server did not answer in time.".to_owned())?
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let reason = String::from_utf8_lossy(&output.stderr);
    Err(reason
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map_or_else(
            || "msmtp could not complete the SMTP request.".to_owned(),
            |line| line.trim().to_owned(),
        ))
}

#[cfg(test)]
#[path = "../tests/unit/sender.rs"]
mod tests;
