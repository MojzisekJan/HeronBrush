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
pub type Request = (String, mpsc::SyncSender<String>);
pub struct Server {
    pub port: u16,
    pub requests: mpsc::Receiver<Request>,
    stopped: Arc<AtomicBool>,
    clients: Arc<Mutex<Vec<std::sync::Weak<TcpStream>>>>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let (tx, requests) = mpsc::channel::<Request>();
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
                    let _ =
                        thread::Builder::new()
                            .name("mcp-live-client".into())
                            .spawn(move || {
                                let _control = control;
                                let mut write = stream;
                                for line in BufReader::new(read).lines() {
                                    let Ok(line) = line else { break };
                                    if stop.load(Ordering::Acquire) {
                                        break;
                                    }
                                    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                                    if tx.send((line, reply_tx)).is_err() {
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
    fn stopping_disconnects_clients_and_releases_port() {
        let server = Server::start().unwrap();
        let address = ("127.0.0.1", server.port);
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
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
