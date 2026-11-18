//! Window-owned localhost server. Dropping it closes the listener and every client.
use std::{
    io::{BufRead, BufReader, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};
pub const MAX_REQUEST_BYTES: usize = 16 << 20;
pub const MAX_CLIENTS: usize = 8;
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(120);

fn read_line(reader: &mut impl BufRead, limit: usize) -> std::io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let deadline = std::time::Instant::now()
        + if limit == 128 {
            AUTH_TIMEOUT
        } else {
            READ_TIMEOUT
        };
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(std::io::ErrorKind::UnexpectedEof.into())
            };
        }
        let count = buffer
            .iter()
            .position(|b| *b == b'\n')
            .map_or(buffer.len(), |i| i + 1);
        if count > limit.saturating_sub(bytes.len()) {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        bytes.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            return String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| std::io::ErrorKind::InvalidData.into());
        }
    }
}

pub type Request = (String, mpsc::SyncSender<String>);
pub struct Server {
    pub port: u16,
    token: String,
    pub requests: mpsc::Receiver<Request>,
    stopped: Arc<AtomicBool>,
    clients: Arc<Mutex<Vec<std::sync::Weak<TcpStream>>>>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    /// Private invitation, copied explicitly by the user. Never log this value.
    pub fn connection_string(&self) -> String {
        format!("127.0.0.1:{}#{}", self.port, self.token)
    }
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let (tx, requests) = mpsc::sync_channel::<Request>(MAX_CLIENTS);
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|e| std::io::Error::other(e.to_string()))?;
        let token: String = secret.iter().map(|b| format!("{b:02x}")).collect();
        let credential = format!("HERON-MCP {token}");
        let stopped = Arc::new(AtomicBool::new(false));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let stop = stopped.clone();
        let connections = clients.clone();
        let worker = thread::Builder::new()
            .name("mcp-live".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    let stream = match listener.accept() {
                        Ok((stream, _)) => stream,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20));
                            continue;
                        }
                        Err(_) => break,
                    };
                    {
                        let mut clients = connections.lock().unwrap();
                        clients.retain(|client: &std::sync::Weak<TcpStream>| {
                            client.strong_count() > 0
                        });
                        if clients.len() >= MAX_CLIENTS {
                            continue;
                        }
                    }
                    if stream.set_read_timeout(Some(AUTH_TIMEOUT)).is_err()
                        || stream
                            .set_write_timeout(Some(Duration::from_secs(5)))
                            .is_err()
                    {
                        continue;
                    }
                    let Ok(read) = stream.try_clone() else {
                        continue;
                    };
                    let Ok(control) = stream.try_clone() else {
                        continue;
                    };
                    let control = Arc::new(control);
                    {
                        let mut clients = connections.lock().unwrap();
                        clients.retain(|client: &std::sync::Weak<TcpStream>| {
                            client.strong_count() > 0
                        });
                        clients.push(Arc::downgrade(&control));
                    }
                    let tx = tx.clone();
                    let stop = stop.clone();
                    let credential = credential.clone();
                    let _ =
                        thread::Builder::new()
                            .name("mcp-live-client".into())
                            .spawn(move || {
                                let _control = control;
                                let mut write = stream;
                                let mut reader = BufReader::new(read);
                                if read_line(&mut reader, 128).ok().flatten().as_deref()
                                    != Some(credential.as_str())
                                {
                                    return;
                                }
                                if write.set_read_timeout(Some(READ_TIMEOUT)).is_err() {
                                    return;
                                }
                                while let Ok(Some(line)) = read_line(&mut reader, MAX_REQUEST_BYTES)
                                {
                                    if stop.load(Ordering::Acquire) {
                                        break;
                                    }
                                    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                                    if tx.try_send((line, reply_tx)).is_err() {
                                        break;
                                    }
                                    let reply = loop {
                                        if stop.load(Ordering::Acquire) {
                                            return;
                                        }
                                        match reply_rx.recv_timeout(Duration::from_millis(20)) {
                                            Ok(reply) => break reply,
                                            Err(mpsc::RecvTimeoutError::Timeout) => continue,
                                            Err(_) => return,
                                        }
                                    };
                                    if !reply.is_empty()
                                        && (writeln!(write, "{reply}").is_err()
                                            || write.flush().is_err())
                                    {
                                        break;
                                    }
                                }
                            });
                }
            })?;
        Ok(Self {
            port,
            token,
            requests,
            stopped,
            clients,
            worker: Some(worker),
        })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        for client in self
            .clients
            .lock()
            .unwrap()
            .drain(..)
            .filter_map(|client| client.upgrade())
        {
            let _ = client.shutdown(Shutdown::Both);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn client_limit_rejects_excess_connections() {
        let server = Server::start().unwrap();
        let mut clients = Vec::new();
        let mut replies = Vec::new();
        for _ in 0..MAX_CLIENTS {
            let mut client = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
            writeln!(client, "HERON-MCP {}\nrequest", server.token).unwrap();
            replies.push(
                server
                    .requests
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap(),
            );
            clients.push(client);
        }
        let mut excess = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
        excess
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut byte = [0];
        match excess.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) => {}
            other => panic!("excess client was not disconnected: {other:?}"),
        }
        assert!(server.requests.try_recv().is_err());
    }

    #[test]
    fn rejects_unterminated_oversized_and_invalid_utf8_lines() {
        assert!(read_line(&mut &b"123456789"[..], 8).is_err());
        assert!(read_line(&mut &b"abc"[..], 8).is_err());
        assert!(read_line(&mut &b"\xff\n"[..], 8).is_err());
        assert_eq!(
            read_line(&mut &b"abc\r\n"[..], 8).unwrap(),
            Some("abc".into())
        );
    }
    #[test]
    fn unauthenticated_requests_never_reach_dispatch_and_valid_client_still_works() {
        let server = Server::start().unwrap();
        let mut rejected = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
        writeln!(rejected, "{{}}\nrequest").unwrap();
        assert!(server
            .requests
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        let mut valid = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
        writeln!(valid, "HERON-MCP {}\nrequest", server.token).unwrap();
        let (line, _) = server
            .requests
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        assert_eq!(line, "request");
        assert_ne!(server.token, Server::start().unwrap().token);
    }
    #[test]
    fn stopping_disconnects_clients_and_releases_port() {
        let server = Server::start().unwrap();
        let address = ("127.0.0.1", server.port);
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        writeln!(client, "HERON-MCP {}", server.token).unwrap();
        writeln!(client, "request").unwrap();
        let (_, _pending_reply) = server
            .requests
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        drop(server);
        let mut byte = [0];
        match client.read(&mut byte) {
            Ok(0) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::NotConnected
                ) => {}
            other => panic!("client was not disconnected: {other:?}"),
        }
        assert!(TcpStream::connect(address).is_err());
        let restarted = Server::start().unwrap();
        assert!(TcpStream::connect(("127.0.0.1", restarted.port)).is_ok());
    }
}
