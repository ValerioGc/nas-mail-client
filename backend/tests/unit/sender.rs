use super::*;
use crate::settings::Settings;
use std::os::unix::fs::PermissionsExt;

/// A msmtp that copies its account and the mail beside itself, and fails when told to.
fn fake_msmtp(name: &str, fail: bool) -> PathBuf {
    let directory = env::temp_dir().join(format!("nas-mail-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create directory");
    let program = directory.join("msmtp");
    let outcome = if fail {
        "echo 'msmtp: server message: 535 authentication failed' >&2; exit 77"
    } else {
        "exit 0"
    };
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\ncp \"$2\" \"{dir}/account\"\necho \"$2\" > \"{dir}/path\"\nstat -c %a \"$2\" > \"{dir}/mode\"\necho \"$3\" > \"{dir}/args\"\ncat > \"{dir}/mail\"\n{outcome}\n",
            dir = directory.display()
        ),
    )
    .expect("write program");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    program
}

fn smtp(username: &str) -> Smtp {
    Smtp {
        host: "smtp.example.com".to_owned(),
        port: 465,
        security: Security::Tls,
        username: username.to_owned(),
        password: Some("p\"a\\ss".to_owned()),
        from: "nas@example.com".to_owned(),
        ..Settings::default().smtp
    }
}

fn message() -> Message<'static> {
    Message {
        from: "nas@example.com",
        recipients: &[],
        subject: "Prova",
        body: "Corpo",
        at_ms: 0,
    }
}

#[test]
fn the_account_follows_the_security_and_quotes_the_password() {
    let account = configuration(&smtp("nas@example.com"));
    assert!(account.contains("host \"smtp.example.com\"\nport 465\n"));
    assert!(account.contains("tls on\ntls_starttls off\n"));
    assert!(account.contains("auth on\nuser \"nas@example.com\"\npassword \"p\\\"a\\\\ss\"\n"));
    assert!(account.ends_with("account default : nas\n"));

    let mut starttls = smtp("");
    starttls.security = Security::Starttls;
    let account = configuration(&starttls);
    assert!(account.contains("tls on\ntls_starttls on\n"));
    assert!(account.contains("auth off\n"));
    assert!(!account.contains("password"));

    let mut plain = smtp("");
    plain.security = Security::None;
    assert!(configuration(&plain).contains("tls off\ntls_starttls off\n"));
}

#[test]
fn finds_a_program_by_path_or_in_the_path() {
    assert!(available(Path::new("sh")));
    assert!(!available(Path::new("no-such-msmtp-anywhere")));
    assert!(!available(Path::new("/no/such/msmtp")));
}

#[tokio::test]
async fn hands_the_mail_to_msmtp_with_a_private_account_removed_afterwards() {
    let program = fake_msmtp("sent", false);
    let directory = program.parent().expect("directory").to_owned();
    send(&program, &smtp("nas@example.com"), &message())
        .await
        .expect("sent");
    let mail = std::fs::read_to_string(directory.join("mail")).expect("mail");
    assert!(mail.contains("Subject: Prova\r\n"));
    assert_eq!(
        std::fs::read_to_string(directory.join("args"))
            .expect("args")
            .trim(),
        "-t"
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("mode"))
            .expect("mode")
            .trim(),
        "600"
    );
    assert!(std::fs::read_to_string(directory.join("account"))
        .expect("account")
        .contains("host \"smtp.example.com\""));
    let account = std::fs::read_to_string(directory.join("path")).expect("path");
    assert!(
        !Path::new(account.trim()).exists(),
        "the account is removed"
    );
    std::fs::remove_dir_all(directory).expect("clean");
}

#[tokio::test]
async fn reports_why_msmtp_failed_or_that_it_is_missing() {
    let program = fake_msmtp("failed", true);
    let error = send(&program, &smtp(""), &message())
        .await
        .expect_err("failed");
    assert_eq!(error, "msmtp: server message: 535 authentication failed");
    std::fs::remove_dir_all(program.parent().expect("directory")).expect("clean");

    let missing = send(Path::new("/no/such/msmtp"), &smtp(""), &message()).await;
    assert_eq!(
        missing,
        Err("msmtp is not installed on the NAS.".to_owned())
    );
}
