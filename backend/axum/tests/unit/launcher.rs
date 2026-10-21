use super::*;
use std::sync::{Arc, Mutex};
use tokio::net::UnixListener;

fn launcher(socket: &std::path::Path, status: u16) -> Arc<Mutex<Vec<String>>> {
    let listener = UnixListener::bind(socket).expect("socket");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let read = stream.read(&mut buffer).await.unwrap_or(0);
                request.extend_from_slice(&buffer[..read]);
                if read == 0 || request.ends_with(b"}") {
                    break;
                }
            }
            kept.lock()
                .expect("seen")
                .push(String::from_utf8_lossy(&request).into_owned());
            let answer = format!("HTTP/1.1 {status} X\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.write_all(answer.as_bytes()).await;
        }
    });
    seen
}

fn socket(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("nas-mail-launcher-{name}-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

#[tokio::test]
async fn says_on_the_socket_of_the_launcher_whether_it_works_in_background() {
    let path = socket("ok");
    let seen = launcher(&path, 204);
    let door = Launcher::new(Some(path.clone()));
    door.background(true).await;
    door.background(false).await;
    let seen = seen.lock().expect("seen").clone();
    assert!(seen[0].starts_with("PUT /api/apps/self/background HTTP/1.1"));
    assert!(seen[0].ends_with(r#"{"needed":true}"#));
    assert!(seen[1].ends_with(r#"{"needed":false}"#));
    std::fs::remove_file(path).expect("clean up");
}

#[tokio::test]
async fn a_refusal_or_no_launcher_stops_nothing() {
    let path = socket("refused");
    launcher(&path, 403);
    Launcher::new(Some(path.clone())).background(true).await;
    std::fs::remove_file(path).expect("clean up");
    Launcher::new(Some(socket("missing")))
        .background(true)
        .await;
    Launcher::new(None).background(true).await;
    Launcher::from_env().background(true).await;
    assert_eq!(status(b"HTTP/1.1 204 No Content\r\n\r\n"), Some(204));
    assert_eq!(status(b"garbage"), None);
}
