use crate::transport::Event;
use eframe::egui;
use std::sync::mpsc::Sender;

#[derive(Clone, Copy)]
pub enum Pick {
    Media,
    Artwork,
    Design,
}
pub fn locale() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        sys_locale::get_locale().unwrap_or_default()
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.navigator().language())
            .unwrap_or_default()
    }
}
pub fn pick(kind: Pick, tx: Sender<Event>, ctx: egui::Context) {
    let task = async move {
        let dialog = rfd::AsyncFileDialog::new();
        let files = match kind {
            Pick::Media => dialog.pick_files().await.unwrap_or_default(),
            Pick::Artwork => dialog
                .add_filter("PNG / JPEG", &["png", "jpg", "jpeg"])
                .pick_file()
                .await
                .into_iter()
                .collect(),
            Pick::Design => dialog
                .add_filter("J-CARD", &["json"])
                .pick_file()
                .await
                .into_iter()
                .collect(),
        };
        let mut bytes = Vec::new();
        if matches!(kind, Pick::Media) && !files.is_empty() {
            let _ = tx.send(Event::ImportReading(
                files.iter().map(|f| f.file_name()).collect(),
            ));
            ctx.request_repaint();
        }
        for f in files {
            bytes.push((f.file_name(), f.read().await));
        }
        let event = match kind {
            Pick::Media => Event::Files {
                files: bytes,
                errors: vec![],
            },
            Pick::Artwork => match bytes.pop() {
                Some((_, b)) => Event::Artwork(b),
                None => return,
            },
            Pick::Design => match bytes.pop() {
                Some((_, b)) => Event::Design(b),
                None => return,
            },
        };
        let _ = tx.send(event);
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || pollster::block_on(task));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(task);
}

pub fn read_dropped(files: Vec<egui::DroppedFile>, tx: Sender<Event>, ctx: egui::Context) {
    let names = files
        .iter()
        .map(|f| {
            if !f.name.is_empty() {
                f.name.clone()
            } else {
                f.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "File".into())
            }
        })
        .collect::<Vec<_>>();
    let _ = tx.send(Event::ImportReading(names.clone()));
    ctx.request_repaint();
    let read = move || {
        let mut ready = vec![];
        let mut errors = vec![];
        for (file, name) in files.into_iter().zip(names) {
            if let Some(bytes) = file.bytes {
                ready.push((name, bytes.to_vec()));
                continue;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = file.path {
                match std::fs::read(path) {
                    Ok(bytes) => ready.push((name, bytes)),
                    Err(error) => errors.push(format!("{name}: {error}")),
                }
                continue;
            }
            errors.push(format!("{name}: no file data available"));
        }
        let _ = tx.send(Event::Files {
            files: ready,
            errors,
        });
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(read);
    #[cfg(target_arch = "wasm32")]
    read();
}
pub fn save(name: String, bytes: Vec<u8>, tx: Sender<Event>, ctx: egui::Context) {
    let name = name.replace(['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'], "_");
    let task = async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_file_name(&name)
            .save_file()
            .await
        else {
            return;
        };
        let result = file.write(&bytes).await;
        let error = result.is_err();
        let _ = tx.send(Event::Notice {
            error,
            message: match result {
                Ok(()) => format!("Saved / 保存しました: {name}"),
                Err(e) => e.to_string(),
            },
        });
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || pollster::block_on(task));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(task);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn dropped_files_report_reading_then_preserve_partial_read_failures() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("cassette-drop-{}-{unique}.wav", std::process::id()));
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        read_dropped(
            vec![
                egui::DroppedFile {
                    path: Some(path.clone()),
                    ..Default::default()
                },
                egui::DroppedFile {
                    path: Some(path.with_extension("missing")),
                    ..Default::default()
                },
            ],
            tx,
            egui::Context::default(),
        );
        assert!(
            matches!(rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap(),Event::ImportReading(names) if names.len()==2)
        );
        match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
            Event::Files { files, errors } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].1, [1, 2, 3]);
                assert_eq!(errors.len(), 1);
            }
            _ => panic!("Expected file read results"),
        }
        std::fs::remove_file(path).unwrap();
    }
}

/// Logical canvas bounds for preventing browser middle-button autoscroll only here.
pub fn editor_viewport(rect: Option<egui::Rect>, pixels_per_point: f32) {
    #[cfg(target_arch = "wasm32")]
    if let Some(canvas) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("cassette-canvas"))
    {
        if let Some(r) = rect {
            let value = format!(
                "{},{},{},{},{}",
                r.min.x, r.min.y, r.max.x, r.max.y, pixels_per_point
            );
            let _ = canvas.set_attribute("data-editor-viewport", &value);
        } else {
            let _ = canvas.remove_attribute("data-editor-viewport");
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (rect, pixels_per_point);
}
