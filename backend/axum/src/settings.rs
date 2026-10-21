//! SMTP account and notification settings. The password is stored encrypted and never returned
//! to the UI, which only learns whether one is set.

use crate::store::{decrypt_secret, encrypt_secret};
use aes_gcm::Aes256Gcm;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    #[default]
    Tls,
    Starttls,
    None,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    It,
    En,
}

/// Minimum level of system updates that triggers a mail.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    All,
    Stable,
    #[default]
    Security,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Smtp {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub username: String,
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
    pub from: String,
    pub recipients: Vec<String>,
}

impl Default for Smtp {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 465,
            security: Security::Tls,
            username: String::new(),
            password: None,
            from: String::new(),
            recipients: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessMails {
    pub refused: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiMails {
    pub available: bool,
    pub updated: bool,
    pub failed: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppMails {
    pub available: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemMails {
    pub available: bool,
    pub level: Level,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub smtp: Smtp,
    #[serde(default)]
    pub access: AccessMails,
    #[serde(default)]
    pub ui: UiMails,
    #[serde(default)]
    pub apps: AppMails,
    #[serde(default)]
    pub system: SystemMails,
}

/// Settings returned to the UI with the password state and the configured flag.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    #[serde(flatten)]
    pub settings: Settings,
    pub password_set: bool,
    pub configured: bool,
}

fn is_header_safe(value: &str) -> bool {
    !value.chars().any(char::is_control)
}

/// Accepts only a plain local@domain address.
pub fn is_address(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    value.len() <= 254
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && value
            .chars()
            .all(|c| c.is_ascii_graphic() && !"<>()[],;:\"\\".contains(c))
        && !domain.contains('@')
}

fn is_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
}

impl Settings {
    /// Validates every field. An empty host is allowed and means not configured.
    pub fn validate(&self) -> Result<(), String> {
        let smtp = &self.smtp;
        if !smtp.host.is_empty() && !is_host(&smtp.host) {
            return Err("The SMTP server name is not valid.".to_owned());
        }
        if smtp.port == 0 {
            return Err("The SMTP port is not valid.".to_owned());
        }
        if smtp.username.len() > 254 || !is_header_safe(&smtp.username) {
            return Err("The SMTP username is not valid.".to_owned());
        }
        if smtp
            .password
            .as_deref()
            .is_some_and(|password| password.len() > 1024 || !is_header_safe(password))
        {
            return Err("The SMTP password is not valid.".to_owned());
        }
        if !smtp.from.is_empty() && !is_address(&smtp.from) {
            return Err("The sender address is not valid.".to_owned());
        }
        if smtp.recipients.len() > 10 || !smtp.recipients.iter().all(|to| is_address(to)) {
            return Err("The recipients are not valid addresses (at most 10).".to_owned());
        }
        Ok(())
    }

    /// True when server, sender and at least one recipient are set.
    pub fn configured(&self) -> bool {
        self.can_send() && !self.smtp.recipients.is_empty()
    }

    /// True when at least one notification is enabled.
    pub fn notifies(&self) -> bool {
        self.access.refused
            || self.ui.available
            || self.ui.updated
            || self.ui.failed
            || self.apps.available
            || self.system.available
    }

    /// True when the app has to keep running in background to send notifications.
    pub fn in_background(&self) -> bool {
        self.configured() && self.notifies()
    }

    /// True when server and sender are set, enough for account codes sent to a given address.
    pub fn can_send(&self) -> bool {
        !self.smtp.host.is_empty() && !self.smtp.from.is_empty()
    }
}

/// Loads the stored settings with the password decrypted, or the defaults.
pub async fn load(db: &SqlitePool, cipher: &Aes256Gcm) -> Result<Settings, String> {
    let stored: Option<(String,)> = sqlx::query_as("SELECT data FROM settings WHERE id = 1")
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())?;
    let Some((data,)) = stored else {
        return Ok(Settings::default());
    };
    let mut settings: Settings = serde_json::from_str(&data).map_err(|error| error.to_string())?;
    settings.smtp.password = settings
        .smtp
        .password
        .as_deref()
        .map(|secret| decrypt_secret(secret, cipher))
        .transpose()?;
    Ok(settings)
}

#[derive(Serialize)]
struct Stored<'a> {
    #[serde(flatten)]
    settings: &'a Settings,
    password: Option<String>,
}

/// Saves the settings. A missing password keeps the stored one.
pub async fn save(
    db: &SqlitePool,
    cipher: &Aes256Gcm,
    mut wanted: Settings,
) -> Result<Settings, String> {
    if wanted.smtp.password.is_none() {
        wanted.smtp.password = load(db, cipher).await?.smtp.password;
    }
    let password = wanted
        .smtp
        .password
        .as_deref()
        .filter(|password| !password.is_empty())
        .map(|password| encrypt_secret(password, cipher))
        .transpose()?;
    let mut data = serde_json::to_value(Stored {
        settings: &wanted,
        password,
    })
    .map_err(|error| error.to_string())?;
    // Move the encrypted password inside smtp, where load reads it.
    if let Some(object) = data.as_object_mut() {
        let password = object.remove("password").unwrap_or_default();
        if let Some(smtp) = object.get_mut("smtp").and_then(|smtp| smtp.as_object_mut()) {
            smtp.insert("password".to_owned(), password);
        }
    }
    sqlx::query(
        "INSERT INTO settings (id, data) VALUES (1, ?)
         ON CONFLICT(id) DO UPDATE SET data = excluded.data",
    )
    .bind(data.to_string())
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(wanted)
}

pub fn view(settings: Settings) -> View {
    View {
        password_set: settings
            .smtp
            .password
            .as_deref()
            .is_some_and(|password| !password.is_empty()),
        configured: settings.configured(),
        settings,
    }
}

#[cfg(test)]
#[path = "../tests/unit/settings.rs"]
mod tests;
