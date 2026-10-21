-- SMTP server and the mails the user wants, as JSON with the password encrypted.
CREATE TABLE settings (
    id   INTEGER PRIMARY KEY CHECK (id = 1),
    data TEXT    NOT NULL
) STRICT;

-- The mails sent, and those in the trash since `trashed_at`.
CREATE TABLE mails (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    category   TEXT    NOT NULL CHECK (category IN ('access', 'ui', 'apps', 'system', 'test')),
    recipients TEXT    NOT NULL,
    subject    TEXT    NOT NULL,
    body       TEXT    NOT NULL,
    sent_at    INTEGER NOT NULL,
    status     TEXT    NOT NULL CHECK (status IN ('sent', 'failed')),
    error      TEXT,
    trashed_at INTEGER
) STRICT;

-- The sent folder in its order, newest first, read straight from the index.
CREATE INDEX mails_by_folder ON mails (trashed_at, sent_at, id);
