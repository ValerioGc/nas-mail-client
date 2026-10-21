//! Plain text mail rendering for msmtp.

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn base64(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0u32, |acc, (index, byte)| {
            acc | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= chunk.len() {
                let sextet = (triple >> (18 - 6 * index)) & 0x3f;
                text.push(char::from(BASE64[sextet as usize]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// Keeps an ASCII subject as is and encodes any other one as an RFC 2047 encoded word.
fn subject(text: &str) -> String {
    if text.is_ascii() {
        text.to_owned()
    } else {
        format!("=?UTF-8?B?{}?=", base64(text.as_bytes()))
    }
}

/// Formats Unix milliseconds as an RFC 5322 date in UTC.
pub fn date(at_ms: i64) -> String {
    let seconds = at_ms.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let time = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} +0000",
        DAYS[days.rem_euclid(7) as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

pub struct Message<'a> {
    pub from: &'a str,
    pub recipients: &'a [String],
    pub subject: &'a str,
    pub body: &'a str,
    pub at_ms: i64,
}

impl Message<'_> {
    /// Renders the mail for msmtp. Line breaks are removed from the subject to prevent header
    /// injection, and the addresses are already validated by the settings.
    pub fn render(&self) -> String {
        let subject_line = self.subject.replace(['\r', '\n'], " ");
        let body = self.body.replace("\r\n", "\n").replace('\n', "\r\n");
        format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
            self.from,
            self.recipients.join(", "),
            subject(&subject_line),
            date(self.at_ms),
            body
        )
    }
}

#[cfg(test)]
#[path = "../tests/unit/message.rs"]
mod tests;
