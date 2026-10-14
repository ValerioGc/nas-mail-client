use super::*;
use crate::settings::{AccessMails, AppMails, SystemMails, UiMails};

fn event(key: &str, params: &[(&str, &str)]) -> Event {
    Event {
        key: key.to_owned(),
        params: params
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

fn wanting_everything(language: Language, level: Level) -> Settings {
    Settings {
        language,
        access: AccessMails { refused: true },
        ui: UiMails {
            available: true,
            updated: true,
            failed: true,
        },
        apps: AppMails { available: true },
        system: SystemMails {
            available: true,
            level,
        },
        ..Settings::default()
    }
}

const UPGRADES: &str = "libc6 1 -> 2 (security)\nbash 1 -> 2 (stable)\ncockpit 1 -> 2 (other)";

#[test]
fn nothing_is_mailed_unless_its_section_asks_for_it() {
    let settings = Settings::default();
    for key in [
        "signInRefused",
        "launcherUpdateAvailable",
        "launcherUpdated",
        "launcherUpdateFailed",
        "appUpdatesAvailable",
        "systemUpdatesAvailable",
        "somethingElse",
    ] {
        assert_eq!(
            compose(&event(key, &[("packages", UPGRADES)]), &settings),
            None,
            "{key}"
        );
    }
}

#[test]
fn a_refused_sign_in_says_from_where_and_what_was_wrong() {
    let it = compose(
        &event(
            "signInRefused",
            &[("from", "10.0.0.9"), ("field", "username")],
        ),
        &wanting_everything(Language::It, Level::All),
    )
    .expect("a mail");
    assert_eq!(it.category, Category::Access);
    assert_eq!(it.subject, "Accesso al NAS rifiutato");
    assert!(it.body.contains("10.0.0.9") && it.body.contains("nome utente"));

    let en = compose(
        &event(
            "signInRefused",
            &[("from", "10.0.0.9"), ("field", "password")],
        ),
        &wanting_everything(Language::En, Level::All),
    )
    .expect("a mail");
    assert_eq!(
        en.body,
        "Sign-in attempt from 10.0.0.9 refused for a wrong password."
    );
}

#[test]
fn each_kind_of_launcher_update_has_its_own_checkbox() {
    let mut settings = wanting_everything(Language::En, Level::All);
    let available = event(
        "launcherUpdateAvailable",
        &[("version", "0.2.0"), ("current", "0.1.0")],
    );
    let mail = compose(&available, &settings).expect("available");
    assert_eq!(mail.category, Category::Ui);
    assert!(mail.body.contains("0.2.0") && mail.body.contains("0.1.0"));
    assert!(compose(
        &event("launcherUpdated", &[("version", "0.2.0")]),
        &settings
    )
    .is_some());
    let gateway = compose(
        &event(
            "componentUpdateAvailable",
            &[
                ("component", "gateway"),
                ("version", "0.1.15"),
                ("current", "-"),
            ],
        ),
        &settings,
    )
    .expect("gateway");
    assert!(gateway.body.contains("NAS gateway") && gateway.body.contains("0.1.15"));
    let together = compose(
        &event(
            "nasUpdated",
            &[
                ("components", "wizard, gateway, launcher"),
                ("version", "0.2.0"),
            ],
        ),
        &settings,
    )
    .expect("together");
    assert!(together.body.contains("wizard, gateway, launcher"));
    for key in [
        "launcherRolledBack",
        "launcherUpdateFailed",
        "launcherUpdateInterrupted",
    ] {
        let failed = compose(
            &event(key, &[("reason", "disk full"), ("version", "0.1.0")]),
            &settings,
        )
        .expect(key);
        assert!(failed.body.contains("disk full"));
    }

    settings.ui.available = false;
    settings.ui.failed = false;
    assert!(compose(&available, &settings).is_none());
    assert!(compose(&event("componentUpdateAvailable", &[]), &settings).is_none());
    assert!(compose(&event("launcherUpdateFailed", &[]), &settings).is_none());
    assert!(compose(&event("launcherUpdated", &[]), &settings).is_some());
}

#[test]
fn application_updates_name_the_applications_in_the_language_of_the_settings() {
    let found = event(
        "appUpdatesAvailable",
        &[("count", "2"), ("apps", "Backup, Explorer")],
    );
    let italian = compose(&found, &wanting_everything(Language::It, Level::Security))
        .expect("an Italian mail");
    assert_eq!(italian.category, Category::Apps);
    assert_eq!(
        italian.subject,
        "Aggiornamenti delle applicazioni disponibili"
    );
    assert!(italian.body.contains("2 applicazioni (Backup, Explorer)"));

    let english = compose(&found, &wanting_everything(Language::En, Level::Security))
        .expect("an English mail");
    assert!(english.body.contains("2 applications (Backup, Explorer)"));

    let mut without = wanting_everything(Language::En, Level::Security);
    without.apps.available = false;
    assert_eq!(compose(&found, &without), None);
}

#[test]
fn system_updates_are_filtered_by_the_level_chosen() {
    let upgrades = event("systemUpdatesAvailable", &[("packages", UPGRADES)]);
    let lines = |level| {
        compose(&upgrades, &wanting_everything(Language::It, level))
            .map(|mail| mail.body.lines().filter(|line| line.contains("->")).count())
    };
    assert_eq!(lines(Level::Security), Some(1));
    assert_eq!(lines(Level::Stable), Some(2));
    assert_eq!(lines(Level::All), Some(3));

    let only_other = event(
        "systemUpdatesAvailable",
        &[("packages", "cockpit 1 -> 2 (other)")],
    );
    assert!(compose(
        &only_other,
        &wanting_everything(Language::It, Level::Stable)
    )
    .is_none());
    let mail = compose(
        &upgrades,
        &wanting_everything(Language::En, Level::Security),
    )
    .expect("mail");
    assert_eq!(mail.category, Category::System);
    assert!(mail
        .body
        .starts_with("1 system updates are available.\n\nlibc6 1 -> 2 (security)"));
}

#[test]
fn the_test_mail_speaks_the_language_of_the_settings() {
    assert_eq!(test_mail(Language::It).subject, "Mail di prova dal NAS");
    assert_eq!(test_mail(Language::En).category, Category::Test);
}

#[test]
fn a_code_mail_names_its_purpose_and_its_code() {
    let (subject, body) = code_mail(Language::It, "verify", "042137").expect("verify");
    assert_eq!(subject, "Verifica dell’indirizzo di recupero del NAS");
    assert!(body.contains(" 042137.\n\n"));
    let (subject, body) = code_mail(Language::En, "reset", "999999").expect("reset");
    assert_eq!(subject, "Code to reset the password of the NAS");
    assert!(body.ends_with("this mail can be ignored."));
    assert!(code_mail(Language::En, "login", "999999").is_none());
}
