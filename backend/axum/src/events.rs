//! The events the launcher posts to `/internal/events`, and the mail each one becomes when the
//! user asked for it in its section.

use crate::{
    mails::Category,
    settings::{Language, Level, Settings},
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct Event {
    pub key: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Composed {
    pub category: Category,
    pub subject: String,
    pub body: String,
}

impl Event {
    fn param(&self, name: &str) -> &str {
        self.params.get(name).map_or("-", String::as_str)
    }
}

/// The upgrades of the list at or above the level asked for, one per line.
fn upgrades_from(list: &str, level: Level) -> Vec<&str> {
    list.lines()
        .filter(|line| match level {
            Level::All => !line.trim().is_empty(),
            Level::Stable => line.ends_with("(security)") || line.ends_with("(stable)"),
            Level::Security => line.ends_with("(security)"),
        })
        .collect()
}

fn field(language: Language, field: &str) -> &'static str {
    match (language, field) {
        (Language::It, "username") => "nome utente",
        (Language::It, _) => "password",
        (Language::En, "username") => "username",
        (Language::En, _) => "password",
    }
}

/// The Italian or the English of a text, after the language of the mails.
fn text<T>(it: bool, italian: T, english: T) -> T {
    if it {
        italian
    } else {
        english
    }
}

fn mail(category: Category, subject: &str, body: String) -> Option<Composed> {
    Some(Composed {
        category,
        subject: subject.to_owned(),
        body,
    })
}

/// The mail an event becomes, or `None` when its section does not ask for it.
pub fn compose(event: &Event, settings: &Settings) -> Option<Composed> {
    let it = settings.language == Language::It;
    match event.key.as_str() {
        "signInRefused" if settings.access.refused => mail(
            Category::Access,
            text(it, "Accesso al NAS rifiutato", "NAS sign-in refused"),
            text(it, format!(
                    "Tentativo di accesso da {} rifiutato per {} non corretto.",
                    event.param("from"),
                    field(settings.language, event.param("field"))
                ), format!(
                    "Sign-in attempt from {} refused for a wrong {}.",
                    event.param("from"),
                    field(settings.language, event.param("field"))
                )),
        ),
        "launcherUpdateAvailable" if settings.ui.available => mail(
            Category::Ui,
            text(it, "Aggiornamento del NAS disponibile", "NAS update available"),
            text(it, format!(
                    "È disponibile la versione {} dell’interfaccia del NAS, versione in uso {}. L’aggiornamento si esegue dalla sezione Aggiornamenti delle Impostazioni.",
                    event.param("version"),
                    event.param("current")
                ), format!(
                    "Version {} of the NAS interface is available, version in use {}. The update runs from the Updates section of the Settings.",
                    event.param("version"),
                    event.param("current")
                )),
        ),
        "componentUpdateAvailable" if settings.ui.available => mail(
            Category::Ui,
            text(it, "Aggiornamento del NAS disponibile", "NAS update available"),
            text(it, format!(
                    "È disponibile la versione {} del {} del NAS, versione in uso {}. L’aggiornamento si esegue dalla sezione Aggiornamenti delle Impostazioni.",
                    event.param("version"),
                    event.param("component"),
                    event.param("current")
                ), format!(
                    "Version {} of the NAS {} is available, version in use {}. The update runs from the Updates section of the Settings.",
                    event.param("version"),
                    event.param("component"),
                    event.param("current")
                )),
        ),
        "nasUpdated" if settings.ui.updated => mail(
            Category::Ui,
            text(it, "NAS aggiornato", "NAS updated"),
            text(it, format!(
                    "NAS aggiornato ({}). Versione del launcher in uso {}.",
                    event.param("components"),
                    event.param("version")
                ), format!(
                    "NAS updated ({}). Launcher version in use {}.",
                    event.param("components"),
                    event.param("version")
                )),
        ),
        "launcherUpdated" if settings.ui.updated => mail(
            Category::Ui,
            text(it, "Interfaccia del NAS aggiornata", "NAS interface updated"),
            text(it, format!(
                    "Interfaccia del NAS aggiornata alla versione {}.",
                    event.param("version")
                ), format!(
                    "NAS interface updated to version {}.",
                    event.param("version")
                )),
        ),
        "launcherRolledBack" | "launcherUpdateFailed" | "launcherUpdateInterrupted"
            if settings.ui.failed =>
        {
            mail(
                Category::Ui,
                text(it, "Aggiornamento del NAS non riuscito", "NAS update failed"),
                text(it, format!(
                        "Aggiornamento dell’interfaccia del NAS non riuscito ({}). Versione in uso {}.",
                        event.param("reason"),
                        event.param("version")
                    ), format!(
                        "Update of the NAS interface failed ({}). Version in use {}.",
                        event.param("reason"),
                        event.param("version")
                    )),
            )
        }
        "appUpdatesAvailable" if settings.apps.available => mail(
            Category::Apps,
            text(it, "Aggiornamenti delle applicazioni disponibili", "Application updates available"),
            text(it, format!(
                    "Aggiornamenti disponibili per {} applicazioni ({}). L’installazione si esegue dal Gestore applicazioni.",
                    event.param("count"),
                    event.param("apps")
                ), format!(
                    "Updates available for {} applications ({}). The installation runs from the Application manager.",
                    event.param("count"),
                    event.param("apps")
                )),
        ),
        "systemUpdatesAvailable" if settings.system.available => {
            let upgrades = upgrades_from(event.param("packages"), settings.system.level);
            if upgrades.is_empty() {
                return None;
            }
            mail(
                Category::System,
                text(it, "Aggiornamenti di sistema disponibili", "System updates available"),
                text(it, format!(
                        "Sono disponibili {} aggiornamenti di sistema.\n\n{}\n\nL’installazione si esegue dalla sezione Aggiornamenti delle Impostazioni.",
                        upgrades.len(),
                        upgrades.join("\n")
                    ), format!(
                        "{} system updates are available.\n\n{}\n\nThe installation runs from the Updates section of the Settings.",
                        upgrades.len(),
                        upgrades.join("\n")
                    )),
            )
        }
        _ => None,
    }
}

/// The mail with a code of the account: to verify the recovery address, or to reset the password.
pub fn code_mail(language: Language, purpose: &str, code: &str) -> Option<(String, String)> {
    let it = language == Language::It;
    let (subject, body) = match (purpose, it) {
        ("verify", true) => (
            "Verifica dell’indirizzo di recupero del NAS",
            "Il codice per verificare questo indirizzo come recupero della password del NAS è",
        ),
        ("verify", false) => (
            "Verification of the recovery address of the NAS",
            "The code to verify this address for the password recovery of the NAS is",
        ),
        ("reset", true) => (
            "Codice per reimpostare la password del NAS",
            "Il codice per reimpostare la password del NAS è",
        ),
        ("reset", false) => (
            "Code to reset the password of the NAS",
            "The code to reset the password of the NAS is",
        ),
        _ => return None,
    };
    let ignore = if it {
        "Il codice è valido 15 minuti. In assenza di una richiesta, la mail può essere ignorata."
    } else {
        "The code is valid for 15 minutes. Without a request, this mail can be ignored."
    };
    Some((subject.to_owned(), format!("{body} {code}.\n\n{ignore}")))
}

#[cfg(test)]
#[path = "../../tests/unit/events.rs"]
mod tests;
