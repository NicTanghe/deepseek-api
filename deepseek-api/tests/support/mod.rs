#![allow(dead_code)]
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
#[derive(Debug)]
pub struct Recorded {
    pub head: String,
    pub body: Vec<u8>,
}
impl Recorded {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}
pub struct Reply {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub chunked: bool,
    pub delay: Duration,
}
impl Reply {
    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: vec![],
            chunked: false,
            delay: Duration::ZERO,
        }
    }
    pub fn sse(body: impl Into<String>) -> Self {
        let mut r = Self::json(200, body);
        r.chunked = true;
        r
    }
}
pub struct Server {
    pub url: String,
    receiver: mpsc::Receiver<Recorded>,
    worker: thread::JoinHandle<()>,
}
impl Server {
    pub fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            for reply in replies {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(2))
                        }
                        Err(e) => panic!("expected HTTP request: {e}"),
                    }
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buf = [0; 4096];
                let header_end = loop {
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let head = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
                let len: usize = head
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(str::trim)
                            .and_then(|s| s.parse().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + len {
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                sender
                    .send(Recorded {
                        head,
                        body: bytes[header_end..header_end + len].to_vec(),
                    })
                    .unwrap();
                thread::sleep(reply.delay);
                let mut head = format!("HTTP/1.1 {} Test\r\nConnection: close\r\n", reply.status);
                for (key, value) in reply.headers {
                    head.push_str(&format!("{key}: {value}\r\n"));
                }
                if reply.chunked {
                    head.push_str(
                        "Content-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
                    );
                } else {
                    head.push_str(&format!(
                        "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                        reply.body.len()
                    ));
                }
                if socket.write_all(head.as_bytes()).is_err() {
                    continue;
                }
                if reply.chunked {
                    // Split at every byte, including UTF-8 and CRLF boundaries.
                    for byte in reply.body.as_bytes() {
                        if socket
                            .write_all(&[b'1', b'\r', b'\n', *byte, b'\r', b'\n'])
                            .is_err()
                        {
                            break;
                        }
                    }
                    let _ = socket.write_all(b"0\r\n\r\n");
                } else {
                    let _ = socket.write_all(reply.body.as_bytes());
                }
            }
        });
        Self {
            url,
            receiver,
            worker,
        }
    }
    pub fn finish(self) -> Vec<Recorded> {
        self.worker.join().unwrap();
        self.receiver.try_iter().collect()
    }
}
