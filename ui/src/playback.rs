//! Shared playback and cancellable dialog previews.
/// Tracks cancellable dialog previews independently of normal playback.
#[derive(Default)]
pub struct PreviewControl {
    generation: Option<u64>,
    preparing: bool,
}
impl PreviewControl {
    pub fn begin(&mut self, generation: u64) {
        self.generation = Some(generation);
        self.preparing = true;
    }
    pub fn owns(&self, generation: u64) -> bool {
        self.generation == Some(generation)
    }
    pub fn ready(&mut self, generation: u64) {
        if self.owns(generation) {
            self.preparing = false;
        }
    }
    pub fn active(&self) -> bool {
        self.generation.is_some()
    }
    pub fn preparing(&self) -> bool {
        self.preparing
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn sync(&mut self, generation: u64, finished: bool) {
        if self.generation.is_some_and(|g| g != generation) || (!self.preparing && finished) {
            self.clear();
        }
    }
}

#[derive(Default)]
pub struct Player {
    #[cfg(not(target_arch = "wasm32"))]
    stream: Option<rodio::OutputStream>,
    #[cfg(not(target_arch = "wasm32"))]
    sink: Option<rodio::Sink>,
    #[cfg(target_arch = "wasm32")]
    audio: Option<web_sys::HtmlAudioElement>,
    #[cfg(target_arch = "wasm32")]
    object_url: Option<String>,
    pub length: f64,
    pub volume: f32,
    pub title: String,
    pub generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    pub device: Option<String>,
    #[cfg(target_arch = "wasm32")]
    offset: f64,
}
impl Player {
    pub fn new() -> Self {
        Self {
            volume: 1.,
            ..Default::default()
        }
    }
    pub fn stop(&mut self) {
        self.generation += 1;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = self.sink.take() {
            s.stop();
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(a) = self.audio.take() {
                let _ = a.pause();
                a.set_src("");
            }
            if let Some(url) = self.object_url.take() {
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        }
        self.length = 0.0;
    }
    pub fn play(&mut self, bytes: Vec<u8>, offset: f64, length: f64) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        let length = if length > 0.0 {
            length
        } else {
            wav_duration(&bytes)
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::Source;
            if self.stream.is_none() {
                self.stream = Some(self.open_output()?);
            }
            if let Some(s) = self.sink.take() {
                s.stop();
            }
            let source =
                rodio::Decoder::try_from(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
            self.length = if length > 0.0 {
                length
            } else {
                source
                    .total_duration()
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0)
            };
            let sink = rodio::Sink::connect_new(self.stream.as_ref().unwrap().mixer());
            sink.set_volume(self.volume);
            sink.append(
                source
                    .skip_duration(std::time::Duration::from_secs_f64(offset.max(0.0)))
                    .take_duration(std::time::Duration::from_secs_f64(self.length.max(0.001))),
            );
            self.sink = Some(sink);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let a = js_sys::Array::new();
            a.push(&js_sys::Uint8Array::from(bytes.as_slice()));
            let props = web_sys::BlobPropertyBag::new();
            props.set_type("audio/wav");
            let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&a, &props)
                .map_err(|e| format!("{e:?}"))?;
            if let Some(url) = self.object_url.take() {
                let _ = web_sys::Url::revoke_object_url(&url);
            }
            let url =
                web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("{e:?}"))?;
            let audio =
                web_sys::HtmlAudioElement::new_with_src(&url).map_err(|e| format!("{e:?}"))?;
            audio.set_volume(self.volume as f64);
            audio.set_current_time(offset);
            let _ = audio.play().map_err(|e| format!("{e:?}"))?;
            self.audio = Some(audio);
            self.object_url = Some(url);
            self.length = length;
            self.offset = offset;
        }
        Ok(())
    }
    pub fn position(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.sink
                .as_ref()
                .map(|s| s.get_pos().as_secs_f64())
                .unwrap_or(0.0)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.audio
                .as_ref()
                .map(|a| (a.current_time() - self.offset).max(0.0))
                .unwrap_or(0.0)
        }
    }
    pub fn finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.sink.as_ref().is_none_or(|s| s.empty())
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.audio.as_ref().is_none_or(|a| {
                a.ended() || (self.length > 0. && self.position() >= self.length - 0.01)
            })
        }
    }
    pub fn paused(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.sink
                .as_ref()
                .is_none_or(|s| s.is_paused() || s.empty())
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.audio.as_ref().is_none_or(|a| a.paused())
        }
    }
    pub fn pause(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = &self.sink {
            s.pause();
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(a) = &self.audio {
            let _ = a.pause();
        }
    }
    pub fn toggle(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = &self.sink {
            if s.is_paused() { s.play() } else { s.pause() }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(a) = &self.audio {
            if a.paused() {
                let _ = a.play();
            } else {
                let _ = a.pause();
            }
        }
    }
    pub fn seek(&self, seconds: f64) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = &self.sink {
            let _ = s.try_seek(std::time::Duration::from_secs_f64(seconds.max(0.0)));
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(a) = &self.audio {
            a.set_current_time(seconds + self.offset);
        }
    }
    pub fn set_volume(&mut self, v: f32) {
        self.volume = v;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = &self.sink {
            s.set_volume(v);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(a) = &self.audio {
            a.set_volume(v as f64);
        }
    }
}
pub fn reference_tone(frequency: f64, db: f64, seconds: f64) -> Vec<u8> {
    let frames = (seconds * 44100.0) as u32;
    let size = frames * 4;
    let mut b = Vec::with_capacity(size as usize + 44);
    b.extend(b"RIFF");
    b.extend((size + 36).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(44100u32.to_le_bytes());
    b.extend(176400u32.to_le_bytes());
    b.extend(4u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(size.to_le_bytes());
    for i in 0..frames {
        let v = (32767.0
            * 10f64.powf(db / 20.0)
            * (std::f64::consts::TAU * frequency * i as f64 / 44100.0).sin())
        .round() as i16;
        b.extend(v.to_le_bytes());
        b.extend(v.to_le_bytes());
    }
    b
}

#[cfg(not(target_arch = "wasm32"))]
impl Player {
    pub fn devices() -> Vec<String> {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        rodio::cpal::default_host()
            .output_devices()
            .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }
    fn open_output(&self) -> Result<rodio::OutputStream, String> {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        if let Some(name) = &self.device {
            let d = rodio::cpal::default_host()
                .output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().ok().as_ref() == Some(name))
                .ok_or("Output device is unavailable")?;
            return rodio::OutputStreamBuilder::from_device(d)
                .map_err(|e| e.to_string())?
                .open_stream()
                .map_err(|e| e.to_string());
        }
        rodio::OutputStreamBuilder::open_default_stream().map_err(|e| e.to_string())
    }
    pub fn change_output(&mut self, device: Option<String>) {
        self.stop();
        self.stream = None;
        self.device = device;
    }
}

#[cfg(target_arch = "wasm32")]
fn wav_duration(b: &[u8]) -> f64 {
    let mut pos = 12;
    let mut rate = 176400u32;
    while pos + 8 <= b.len() {
        let size = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if &b[pos..pos + 4] == b"fmt " && pos + 20 <= b.len() {
            rate = u32::from_le_bytes(b[pos + 16..pos + 20].try_into().unwrap());
        }
        if &b[pos..pos + 4] == b"data" {
            return size.min(b.len() - pos - 8) as f64 / rate.max(1) as f64;
        }
        pos = pos
            .saturating_add(8)
            .saturating_add(size)
            .saturating_add(size % 2);
    }
    0.0
}

#[cfg(test)]
mod preview_tests {
    use super::PreviewControl;
    #[test]
    fn cancellation_and_late_responses_cannot_reactivate_a_preview() {
        let mut p = PreviewControl::default();
        p.begin(4);
        p.sync(4, true);
        assert!(p.active() && p.preparing());
        p.clear();
        p.ready(4);
        assert!(!p.active());
        p.begin(5);
        p.ready(4);
        assert!(p.preparing());
        p.ready(5);
        assert!(!p.preparing());
        p.sync(5, false);
        assert!(p.active());
        p.sync(5, true);
        assert!(!p.active());
        p.begin(6);
        p.sync(7, false);
        assert!(!p.active());
    }
}
