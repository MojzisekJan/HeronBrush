//! One-shot localhost acknowledgement of a managed Defold level replacement.
use std::{
    hash::{BuildHasher, Hash, Hasher},
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::Path,
    time::{Duration, Instant},
};

pub(crate) struct Receipt {
    listener: TcpListener,
    path: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultMessage {
    status: String,
    generation: u64,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    kept_previous: bool,
}
impl Receipt {
    pub(crate) fn prepare(root: &Path) -> Result<Self, String> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let mut hash = std::collections::hash_map::RandomState::new().build_hasher();
        listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .hash(&mut hash);
        std::process::id().hash(&mut hash);
        let nonce = format!("{:016x}", hash.finish());
        let path = format!("/heron/{nonce}");
        let url = format!(
            "http://127.0.0.1:{}{path}",
            listener.local_addr().map_err(|e| e.to_string())?.port()
        );
        let folder = root.join(crate::defold_live::FOLDER);
        let collection = std::fs::read_to_string(folder.join("preview.collection"))
            .map_err(|e| e.to_string())?;
        // A .go prefab can spawn children too, so this is a lower bound, not an exact count.
        let count = collection
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("instances {") || line.starts_with("embedded_instances {")
            })
            .count();
        let revision = folder.join("revision.script");
        if std::fs::symlink_metadata(&revision).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Defold revision script must not be a symbolic link".into());
        }
        crate::files::write_atomic(
            &revision,
            crate::defold_addon::revision_with_receipt(&nonce, &url, count),
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { listener, path })
    }
    pub(crate) fn wait(self) -> Result<String, String> {
        self.wait_for(Duration::from_secs(15))
    }
    fn wait_for(self, timeout: Duration) -> Result<String, String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    match read_result(&mut stream, &self.path) {
                        Ok(result) => {
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
                            return match result.status.as_str() {
                                "loaded" if result.generation > 0 => Ok(format!(
                                    "Defold updated — runtime confirmed level generation {}.",
                                    result.generation
                                )),
                                "failed" => Err(format!(
                                    "Defold runtime rejected the replacement: {}. {}",
                                    result.reason.chars().take(300).collect::<String>(),
                                    if result.kept_previous {
                                        "Previous map objects were preserved."
                                    } else {
                                        "No previous map was loaded."
                                    }
                                )),
                                _ => {
                                    Err("Defold returned an invalid runtime acknowledgement".into())
                                }
                            };
                        }
                        Err(_) => {
                            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("Defold did not confirm replacing the map. Update the addon, add controller.go to the running game, and check its console.".into())
    }
}
fn read_result(stream: &mut TcpStream, path: &str) -> Result<ResultMessage, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let n = stream.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Truncated runtime acknowledgement".into());
        }
        bytes.extend_from_slice(&buffer[..n]);
        if bytes.len() > 8192 {
            return Err("Runtime acknowledgement is too large".into());
        }
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())?;
            if header.lines().next() != Some(&format!("POST {path} HTTP/1.1"))
                && header.lines().next() != Some(&format!("POST {path} HTTP/1.0"))
            {
                return Err("Unexpected runtime acknowledgement endpoint".into());
            }
            let mut length = None;
            for line in header.lines().skip(1) {
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("Content-Length") {
                        if length.is_some() {
                            return Err("Duplicate content length".into());
                        }
                        length = Some(value.trim().parse::<usize>().map_err(|e| e.to_string())?);
                    }
                }
            }
            let length = length.ok_or("Missing content length")?;
            if length > 4096 {
                return Err("Runtime acknowledgement is too large".into());
            }
            if bytes.len() >= end + 4 + length {
                return serde_json::from_slice(&bytes[end + 4..end + 4 + length])
                    .map_err(|e| e.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn receive(body: &str) -> Result<String, String> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_owned();
        let sender = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            write!(
                stream,
                "POST /heron/test HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"));
        });
        let result = Receipt {
            listener,
            path: "/heron/test".into(),
        }
        .wait_for(Duration::from_secs(2));
        sender.join().unwrap();
        result
    }
    #[test]
    fn runtime_success_requires_a_loaded_generation() {
        assert!(receive(r#"{"status":"loaded","generation":3}"#)
            .unwrap()
            .contains("generation 3"));
        assert!(receive(r#"{"status":"loaded","generation":0}"#).is_err());
    }
    #[test]
    fn runtime_failures_report_whether_the_map_was_preserved() {
        let error=receive(r#"{"status":"failed","generation":2,"reason":"missing resource","kept_previous":true}"#).unwrap_err();
        assert!(error.contains("missing resource") && error.contains("preserved"));
    }
    #[test]
    #[ignore = "requires the native engine fixture"]
    fn native_runtime_receipt() {
        let root = std::path::PathBuf::from(std::env::var("HERON_DEFOLD_ADDON_PROJECT").unwrap());
        let receipt = Receipt::prepare(&root).unwrap();
        println!("HERON_RECEIPT_READY");
        std::io::stdout().flush().unwrap();
        let response = receipt.wait_for(Duration::from_secs(90)).unwrap();
        assert!(response.contains("runtime confirmed level generation"));
        println!("HERON_NATIVE_RECEIPT_OK {response}");
    }
    #[test]
    fn no_acknowledgement_is_not_success() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        assert!(Receipt {
            listener,
            path: "/heron/test".into()
        }
        .wait_for(Duration::from_millis(30))
        .is_err());
    }
}
