use crate::{Error, Result};
use std::collections::VecDeque;
/// Shared, bounded SSE framing for async and blocking transports.
#[derive(Default)]
pub(crate) struct Decoder {
    data: String,
    line_bytes: Vec<u8>,
    pub pending: VecDeque<Result<serde_json::Value>>,
    pub done: bool,
}
impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.done {
                break;
            }
            if byte == b'\n' {
                let bytes = std::mem::take(&mut self.line_bytes);
                match std::str::from_utf8(&bytes) {
                    Ok(line) => {
                        if let Some(event) = self.line(line) {
                            self.pending.push_back(event);
                        }
                    }
                    Err(_) => {
                        self.done = true;
                        self.pending
                            .push_back(Err(Error::Stream("SSE contains invalid UTF-8".into())));
                    }
                }
            } else {
                self.line_bytes.push(byte);
                if self.line_bytes.len() + self.data.len() > 8 * 1024 * 1024 {
                    self.done = true;
                    self.line_bytes.clear();
                    self.data.clear();
                    self.pending
                        .push_back(Err(Error::Stream("SSE event exceeds 8 MiB".into())));
                }
            }
        }
    }
    pub fn finish(&mut self) {
        if !self.line_bytes.is_empty() && !self.done {
            self.feed(b"\n");
        }
        if let Some(event) = self.dispatch() {
            self.pending.push_back(event);
        }
        if !self.done {
            self.pending.push_back(Err(Error::Stream(
                "connection ended before the terminal SSE event".into(),
            )));
            self.done = true;
        }
    }
    pub fn read_error(&mut self) {
        self.done = true;
        self.pending
            .push_back(Err(Error::Stream("failed to read SSE body".into())));
    }
    pub fn line(&mut self, line: &str) -> Option<Result<serde_json::Value>> {
        if self.done {
            return None;
        }
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            return self.dispatch();
        }
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.strip_prefix(' ').unwrap_or(data);
            if self.data.len() + data.len() > 8 * 1024 * 1024 {
                self.done = true;
                return Some(Err(Error::Stream("SSE event exceeds 8 MiB".into())));
            }
            if !self.data.is_empty() {
                self.data.push('\n');
            }
            self.data.push_str(data);
        }
        // Comments and event/id/retry/unknown fields do not carry JSON payloads.
        None
    }
    pub fn dispatch(&mut self) -> Option<Result<serde_json::Value>> {
        if self.data.is_empty() || self.done {
            return None;
        }
        let data = std::mem::take(&mut self.data);
        if data.trim() == "[DONE]" {
            self.done = true;
            return None;
        }
        Some(
            serde_json::from_str::<serde_json::Value>(&data)
                .map_err(Error::from)
                .and_then(|value| {
                    if value.get("error").is_some() && value.get("type").is_none() {
                        self.done = true;
                        return Err(Error::Stream("provider returned an error event".into()));
                    }
                    if matches!(
                        value.get("type").and_then(|v| v.as_str()),
                        Some("response.completed" | "response.incomplete" | "response.failed")
                    ) {
                        self.done = true;
                    }
                    Ok(value)
                }),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiline_comments_and_terminal() {
        let mut d = Decoder::default();
        for line in [
            ": ping",
            "event: test",
            "id: 5",
            "data:{",
            "data:  \"value\": 42}",
        ] {
            assert!(d.line(line).is_none());
        }
        assert_eq!(d.line("").unwrap().unwrap()["value"], 42);
        d.line("data:[DONE]");
        assert!(d.line("").is_none());
        assert!(d.done);
        d.finish();
        assert!(d.pending.is_empty());
    }
    #[test]
    fn malformed_event_recovers_and_eof_is_reported() {
        let mut d = Decoder::default();
        d.line("data: invalid");
        assert!(d.line("").unwrap().is_err());
        d.line("data: {}");
        assert!(d.line("").unwrap().is_ok());
        d.finish();
        assert!(d.pending.pop_front().unwrap().is_err());
        d.finish();
        assert!(d.pending.is_empty());
    }
    #[test]
    fn oversized_lines_invalid_utf8_and_unterminated_done() {
        let mut d = Decoder::default();
        d.feed(&vec![b'x'; 8 * 1024 * 1024 + 1]);
        assert!(d.done);
        assert!(d.pending.pop_front().unwrap().is_err());
        let mut d = Decoder::default();
        d.feed(b"data: \xff\n");
        assert!(d.done);
        assert!(d.pending.pop_front().unwrap().is_err());
        let mut d = Decoder::default();
        d.feed(b"data:[DONE]");
        d.finish();
        assert!(d.done);
        assert!(d.pending.is_empty());
    }
}
