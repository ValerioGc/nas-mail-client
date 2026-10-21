//! Sent and trashed mails. Failed mails are stored too, together with their error.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

const LIST_LIMIT: i64 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Access,
    Ui,
    Apps,
    System,
}

impl Category {
    pub fn id(self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Ui => "ui",
            Self::Apps => "apps",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Folder {
    Sent,
    Trash,
}

#[derive(Debug, Serialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Mail {
    pub id: i64,
    pub category: String,
    pub recipients: String,
    pub subject: String,
    pub body: String,
    pub sent_at: i64,
    pub status: String,
    pub error: Option<String>,
    pub trashed_at: Option<i64>,
}

pub struct Outgoing<'a> {
    pub category: Category,
    pub recipients: &'a [String],
    pub subject: &'a str,
    pub body: &'a str,
    pub at_ms: i64,
}

/// Stores a mail with the outcome of its delivery.
pub async fn record(
    db: &SqlitePool,
    mail: &Outgoing<'_>,
    outcome: &Result<(), String>,
) -> Result<Mail, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO mails (category, recipients, subject, body, sent_at, status, error)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(mail.category.id())
    .bind(mail.recipients.join(", "))
    .bind(mail.subject)
    .bind(mail.body)
    .bind(mail.at_ms)
    .bind(if outcome.is_ok() { "sent" } else { "failed" })
    .bind(outcome.as_ref().err())
    .fetch_one(db)
    .await
}

/// Lists the mails of a folder, newest first.
pub async fn list(db: &SqlitePool, folder: Folder) -> Result<Vec<Mail>, sqlx::Error> {
    let query = match folder {
        Folder::Sent => {
            "SELECT * FROM mails WHERE trashed_at IS NULL ORDER BY sent_at DESC, id DESC LIMIT ?"
        }
        Folder::Trash => {
            "SELECT * FROM mails WHERE trashed_at IS NOT NULL ORDER BY sent_at DESC, id DESC LIMIT ?"
        }
    };
    sqlx::query_as(query).bind(LIST_LIMIT).fetch_all(db).await
}

/// Moves a mail to the trash, or restores it with None. Returns false when the mail does not exist.
pub async fn set_trashed(db: &SqlitePool, id: i64, at: Option<i64>) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("UPDATE mails SET trashed_at = ? WHERE id = ?")
        .bind(at)
        .bind(id)
        .execute(db)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Permanently deletes a trashed mail. Returns false when the trash has no such mail.
pub async fn delete(db: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("DELETE FROM mails WHERE id = ? AND trashed_at IS NOT NULL")
        .bind(id)
        .execute(db)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Empties the trash and returns the number of deleted mails.
pub async fn empty_trash(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM mails WHERE trashed_at IS NOT NULL")
            .execute(db)
            .await?
            .rows_affected(),
    )
}

#[cfg(test)]
#[path = "../tests/unit/mails.rs"]
mod tests;
