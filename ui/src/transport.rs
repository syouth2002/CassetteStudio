use crate::model::Settings;
use eframe::egui;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone)]
pub enum Tag {
    Library,
    Properties(String),
    Mono,
    EntryAudio {
        generation: u64,
        duration: f64,
    },
    Changed(Option<String>),
    Reordered,
    Imported {
        target: Option<(String, usize)>,
    },
    ImportSaved {
        message: String,
        warning: bool,
    },
    Message,
    Plan {
        name: String,
        settings: Settings,
    },
    DeckAudio {
        id: String,
        side: usize,
        snapshot: Value,
    },
    Render {
        snapshot: Value,
        id: String,
        generation: u64,
        play: bool,
        prepare: bool,
        side: usize,
    },
    Preview {
        generation: u64,
        offset: f64,
        duration: f64,
    },
    Audio {
        generation: u64,
        offset: f64,
        duration: f64,
    },
    Download(String),
    Graph(u64),
    JCard(String),
    JCardRefresh(String),
    JCardSaved(u64),
    GainThenStereo {
        id: String,
        strength: u8,
    },
}
impl Tag {
    pub fn playback_generation(&self) -> Option<u64> {
        match self {
            Self::Preview { generation, .. }
            | Self::Audio { generation, .. }
            | Self::EntryAudio { generation, .. }
            | Self::Render {
                generation,
                play: true,
                ..
            } => Some(*generation),
            _ => None,
        }
    }
}
pub enum Event {
    Response(Tag, Result<Vec<u8>, String>),
    ImportReading(Vec<String>),
    Files {
        files: Vec<(String, Vec<u8>)>,
        errors: Vec<String>,
    },
    Artwork(Vec<u8>),
    Design(Vec<u8>),
    Notice {
        message: String,
        error: bool,
    },
}
pub struct Transport {
    pub base: String,
    pub tx: Sender<Event>,
    pub rx: Receiver<Event>,
    queue: std::collections::VecDeque<(Tag, ehttp::Request)>,
    pub pending: bool,
}
impl Transport {
    pub fn new(base: String) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            base,
            tx,
            rx,
            queue: Default::default(),
            pending: false,
        }
    }
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base.trim_end_matches('/'), path)
    }
    pub fn json(&mut self, tag: Tag, method: &str, path: &str, body: Value) {
        let mut req = ehttp::Request::get(self.url(path));
        req.method = ehttp::Method::parse(method).expect("valid HTTP method");
        if method != "GET" {
            req.body = serde_json::to_vec(&body).expect("JSON request");
            req.headers.insert("Content-Type", "application/json");
        }
        req.timeout = None;
        self.queue.push_back((tag, req));
    }
    pub fn upload(
        &mut self,
        files: Vec<(String, Vec<u8>)>,
        settings: &Settings,
        target: Option<(String, usize)>,
    ) {
        let boundary = "CassetteStudioRustUpload_ea4a092197b64acd";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"settings\"\r\n\r\n{}\r\n",
            serde_json::to_string(settings).unwrap()
        )
        .into_bytes();
        for (name, bytes) in files {
            let name = name.replace(['"', '\r', '\n', '\\', '/'], "_");
            body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
            body.extend(bytes);
            body.extend(b"\r\n");
        }
        body.extend(format!("--{boundary}--\r\n").as_bytes());
        let mut req = ehttp::Request::post(self.url("/api/import"), body);
        req.headers = ehttp::Headers::new(&[
            ("Accept", "*/*"),
            (
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            ),
        ]);
        req.timeout = None;
        self.queue.push_back((Tag::Imported { target }, req));
    }
    pub fn tick(&mut self, ctx: &egui::Context) {
        if self.pending {
            return;
        }
        if let Some((tag, request)) = self.queue.pop_front() {
            self.pending = true;
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            ehttp::fetch(request, move |response| {
                let result = response.and_then(|r| {
                    if r.ok {
                        Ok(r.bytes)
                    } else {
                        Err(serde_json::from_slice::<Value>(&r.bytes)
                            .ok()
                            .and_then(|v| v["error"].as_str().map(str::to_owned))
                            .unwrap_or_else(|| format!("HTTP {}: {}", r.status, r.status_text)))
                    }
                });
                let _ = tx.send(Event::Response(tag, result));
                ctx.request_repaint();
            });
        }
    }
    pub fn busy(&self) -> bool {
        self.pending || !self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upload_sends_one_multipart_content_type() {
        let mut transport = Transport::new("http://127.0.0.1:1/".into());
        transport.upload(
            vec![("追加テスト☆.wav".into(), vec![0, 1, 2, 255])],
            &Settings::default(),
            None,
        );
        let (_, request) = transport.queue.pop_front().unwrap();
        let types = request.headers.get_all("Content-Type").collect::<Vec<_>>();
        assert_eq!(types.len(), 1);
        assert!(types[0].starts_with("multipart/form-data; boundary="));
        assert!(
            request
                .body
                .windows("追加テスト☆.wav".len())
                .any(|w| w == "追加テスト☆.wav".as_bytes())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires CASSETTE_TEST_BASE pointing at a disposable running engine"]
    fn native_unicode_media_import() {
        let base = std::env::var("CASSETTE_TEST_BASE").expect("disposable engine URL");
        let mut transport = Transport::new(base);
        for duplicate in [false, true] {
            transport.upload(
                vec![(
                    "追加テスト☆.wav".into(),
                    crate::playback::reference_tone(440., -18., 1.),
                )],
                &Settings::default(),
                None,
            );
            let (_, request) = transport.queue.pop_front().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            ehttp::fetch(request, move |r| {
                let _ = tx.send(r);
            });
            let response = rx
                .recv_timeout(std::time::Duration::from_secs(90))
                .unwrap()
                .unwrap();
            assert!(
                response.ok,
                "{}: {}",
                response.status,
                String::from_utf8_lossy(&response.bytes)
            );
            let result: Value = serde_json::from_slice(&response.bytes).unwrap();
            assert_eq!(result["errors"].as_array().unwrap().len(), 0, "{result}");
            assert_eq!(
                result["tracks"].as_array().unwrap().len(),
                usize::from(!duplicate)
            );
            assert_eq!(
                result["duplicates"].as_array().unwrap().len(),
                usize::from(duplicate)
            );
            let track = if duplicate {
                &result["duplicates"][0]["track"]
            } else {
                &result["tracks"][0]
            };
            assert_eq!(track["filename"], "追加テスト☆.wav");
        }
    }
}
