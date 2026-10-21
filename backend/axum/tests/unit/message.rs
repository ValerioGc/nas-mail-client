use super::*;

#[test]
fn encodes_base64_with_padding() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64("è".as_bytes()), "w6g=");
}

#[test]
fn writes_the_date_in_utc() {
    assert_eq!(date(0), "Thu, 01 Jan 1970 00:00:00 +0000");
    assert_eq!(date(1_772_000_000_000), "Wed, 25 Feb 2026 06:13:20 +0000");
    assert_eq!(date(951_782_400_000), "Tue, 29 Feb 2000 00:00:00 +0000");
}

#[test]
fn renders_the_headers_and_keeps_the_subject_on_one_line() {
    let recipients = ["a@example.com".to_owned(), "b@example.com".to_owned()];
    let text = Message {
        from: "nas@example.com",
        recipients: &recipients,
        subject: "Accesso rifiutato\r\nBcc: x@y.z",
        body: "riga 1\nriga 2",
        at_ms: 0,
    }
    .render();
    assert!(text.starts_with("From: nas@example.com\r\nTo: a@example.com, b@example.com\r\n"));
    assert!(text.contains("Subject: Accesso rifiutato  Bcc: x@y.z\r\n"));
    assert!(!text.contains("\r\nBcc:"));
    assert!(text.contains("Content-Type: text/plain; charset=UTF-8\r\n"));
    assert!(text.ends_with("\r\n\r\nriga 1\r\nriga 2\r\n"));

    let accented = Message {
        from: "nas@example.com",
        recipients: &recipients,
        subject: "Disponibilità",
        body: "",
        at_ms: 0,
    }
    .render();
    assert!(accented.contains(&format!(
        "Subject: =?UTF-8?B?{}?=",
        base64("Disponibilità".as_bytes())
    )));
}
