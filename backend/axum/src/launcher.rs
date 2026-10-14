//! The door to the launcher, `NAS_LAUNCHER_SOCKET`: the launcher knows this application by its
//! cgroup and lets it write only as itself. Through it the application says whether it has mails
//! to send without its window (a configured account and at least one notification on): while it
//! has, the launcher starts it with the NAS and leaves it running when its window closes.
//!
//! Plain HTTP/1.1 over the unix socket, one request per connection. Without the socket (in
//! development) nothing is sent, and nothing the launcher answers stops a mail.

use serde_json::json;
use std::{path::PathBuf, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Default)]
pub struct Launcher {
    socket: Option<PathBuf>,
}

impl Launcher {
    pub fn new(socket: Option<PathBuf>) -> Self {
        Self { socket }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var_os("NAS_LAUNCHER_SOCKET")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
        )
    }

    /// Says whether this application has work to do without its window.
    pub async fn background(&self, needed: bool) {
        let Some(socket) = &self.socket else {
            return;
        };
        let body = json!({ "needed": needed }).to_string();
        let exchange = async {
            let mut stream = UnixStream::connect(socket).await?;
            let request = format!(
                "PUT /api/apps/self/background HTTP/1.1\r\nHost: nas\r\nConnection: close\r\n\
                 Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(request.as_bytes()).await?;
            let mut answer = Vec::new();
            stream.read_to_end(&mut answer).await?;
            Ok::<_, std::io::Error>(status(&answer))
        };
        match tokio::time::timeout(TIMEOUT, exchange).await {
            Ok(Ok(Some(status))) if status < 300 => {}
            Ok(Ok(status)) => eprintln!("<4>the launcher refused the background: {status:?}"),
            Ok(Err(error)) => eprintln!("<4>the background was not declared: {error}"),
            Err(_) => eprintln!("<4>the launcher did not answer about the background"),
        }
    }
}

/// The status of an HTTP answer.
pub fn status(answer: &[u8]) -> Option<u16> {
    String::from_utf8_lossy(answer)
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[cfg(test)]
#[path = "../../tests/unit/launcher.rs"]
mod tests;
