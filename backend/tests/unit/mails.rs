use super::*;
use crate::support::memory;

async fn sent(db: &SqlitePool, subject: &str, at_ms: i64, outcome: Result<(), String>) -> Mail {
    let recipients = ["me@example.com".to_owned(), "you@example.com".to_owned()];
    record(
        db,
        &Outgoing {
            category: Category::Access,
            recipients: &recipients,
            subject,
            body: "body",
            at_ms,
        },
        &outcome,
    )
    .await
    .expect("record")
}

#[tokio::test]
async fn keeps_each_mail_with_its_outcome_newest_first() {
    let db = memory().await;
    let first = sent(&db, "first", 1, Ok(())).await;
    assert_eq!(first.status, "sent");
    assert_eq!(first.recipients, "me@example.com, you@example.com");
    assert_eq!(first.category, "access");
    let second = sent(&db, "second", 2, Err("refused".to_owned())).await;
    assert_eq!(second.status, "failed");
    assert_eq!(second.error.as_deref(), Some("refused"));

    let subjects: Vec<_> = list(&db, Folder::Sent)
        .await
        .expect("list")
        .into_iter()
        .map(|mail| mail.subject)
        .collect();
    assert_eq!(subjects, ["second", "first"]);
}

#[tokio::test]
async fn moves_mails_to_the_trash_and_back_and_deletes_only_from_it() {
    let db = memory().await;
    let mail = sent(&db, "one", 1, Ok(())).await;
    assert!(
        !delete(&db, mail.id).await.expect("delete"),
        "not in the trash yet"
    );

    assert!(set_trashed(&db, mail.id, Some(5)).await.expect("trash"));
    assert!(list(&db, Folder::Sent).await.expect("sent").is_empty());
    let trash = list(&db, Folder::Trash).await.expect("trash");
    assert_eq!(trash[0].trashed_at, Some(5));

    assert!(set_trashed(&db, mail.id, None).await.expect("restore"));
    assert_eq!(list(&db, Folder::Sent).await.expect("sent").len(), 1);

    assert!(set_trashed(&db, mail.id, Some(6)).await.expect("trash"));
    assert!(delete(&db, mail.id).await.expect("delete"));
    assert!(!set_trashed(&db, mail.id, None).await.expect("gone"));
}

#[tokio::test]
async fn empties_the_trash_only() {
    let db = memory().await;
    let kept = sent(&db, "kept", 1, Ok(())).await;
    for at in [2, 3] {
        let mail = sent(&db, "old", at, Ok(())).await;
        set_trashed(&db, mail.id, Some(at)).await.expect("trash");
    }
    assert_eq!(empty_trash(&db).await.expect("empty"), 2);
    let left = list(&db, Folder::Sent).await.expect("sent");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, kept.id);
}
