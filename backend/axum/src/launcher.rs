//! Client for the launcher socket NAS_LAUNCHER_SOCKET. The app declares whether it needs to run in
//! background, so the launcher starts it with the NAS and keeps it running with its window closed.
//!
//! One HTTP/1.1 request per connection. Without the socket nothing is sent, and launcher errors
//! never block a mail.

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

    /// Declares whether the app has work to do with its window closed.
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

/// Parses the status code of an HTTP response.
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
#[path = "../tests/unit/launcher.rs"]
mod tests;
