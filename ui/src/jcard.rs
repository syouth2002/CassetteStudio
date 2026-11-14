//! Millimetre artwork shared by the native/wasm editor, SVG preview and PDF export.
use crate::{
    model::{Collection, Library, Track},
    platform,
    settings::{time, tr},
    transport::{Tag, Transport},
};
use base64::Engine;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use serde_json::{Value, json};
#[path = "jcard_actions.rs"]
mod actions;

#[derive(Clone)]
pub struct Clipboard {
    item: Value,
    panels: Vec<(String, f64, f64)>,
    back: bool,
    cassette: String,
    cut: bool,
    pasted: u64,
}
fn panels(d: &Value, back: bool) -> Vec<(String, f64, f64)> {
    let mut list = vec![
        ("flap".into(), 0., num(d, "flap")),
        ("spine".into(), num(d, "flap"), num(d, "spine")),
        (
            "front".into(),
            num(d, "flap") + num(d, "spine"),
            num(d, "front"),
        ),
    ];
    let mut start = num(d, "flap") + num(d, "spine") + num(d, "front");
    for (i, p) in d["extensions"].as_array().into_iter().flatten().enumerate() {
        let w = p.as_f64().unwrap_or(0.);
        list.push((format!("extra-{i}"), start, w));
        start += w;
    }
    if back {
        let total = width(d);
        for (_, x, w) in &mut list {
            *x = total - *x - *w;
        }
    }
    list
}
pub struct Card {
    id: String,
    name: String,
    pub doc: Value,
    back: bool,
    selected: Option<usize>,
    undo: Vec<Value>,
    redo: Vec<Value>,
    pub clipboard: Option<Clipboard>,
    version: u64,
    ack: u64,
    sent: Option<u64>,
    changed_at: f64,
    closing: bool,
    zoom: f32,
    pan: Vec2,
    view_turns: u8,
    pan_drag: bool,
    menu_point: Vec2,
    snap: bool,
    custom_spine: bool,
    texture: Option<egui::TextureHandle>,
    preview_key: String,
    preview_revision: u64,
    preview_back: bool,
    drag: Option<(Value, Pos2, String)>,
    error: String,
}
fn num(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or(0.)
}
fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn choice_label(ja: bool, key: &str) -> &str {
    match key {
        "left" => tr(ja, "左揃え", "Left"),
        "center" => tr(ja, "中央揃え", "Center"),
        "right" => tr(ja, "右揃え", "Right"),
        "sans" => tr(ja, "ゴシック体", "Sans serif"),
        "serif" => tr(ja, "明朝体", "Serif"),
        "mono" => tr(ja, "等幅", "Monospace"),
        "manual" | "" => tr(ja, "自由入力（更新しない）", "Free text (no refresh)"),
        "title" => tr(ja, "カセット名", "Cassette title"),
        "spine" => tr(ja, "背表紙の名前", "Spine title"),
        "sideA" => tr(ja, "A面の曲一覧", "Side A tracks"),
        "sideB" => tr(ja, "B面の曲一覧", "Side B tracks"),
        _ => key,
    }
}
fn uid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    format!("element-{}", N.fetch_add(1, Ordering::Relaxed))
}
pub fn width(d: &Value) -> f64 {
    num(d, "front")
        + num(d, "spine")
        + num(d, "flap")
        + d["extensions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_f64)
            .sum::<f64>()
}
pub fn paper(d: &Value) -> Option<(f64, f64)> {
    let (w, h) = if text(d, "paper") == "letter" {
        (215.9, 279.4)
    } else {
        (210., 297.)
    };
    let pad = 10. + 2. * num(d, "margin");
    let (cw, ch) = (width(d) + pad, num(d, "height") + pad);
    if cw <= w && ch <= h {
        Some((w, h))
    } else if cw <= h && ch <= w {
        Some((h, w))
    } else {
        None
    }
}
#[allow(clippy::too_many_arguments)]
fn box_text(
    id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    s: String,
    size: f64,
    binding: &str,
) -> Value {
    json!({"id":id,"kind":"text","x":x,"y":y,"w":w,"h":h,"rotation":0,"text":s,"size":size,"lineHeight":1.3,"font":"sans","align":"left","bold":false,"vertical":false,"binding":binding,"color":"#233b36"})
}
fn values(c: &Collection, tracks: &[Track]) -> Value {
    let side = |i: usize| {
        let mut s = format!("SIDE {}", if i == 0 { "A" } else { "B" });
        for (n, e) in c.sides[i].iter().enumerate() {
            if let Some(t) = tracks.iter().find(|t| t.id == e.track_id) {
                s += &format!(
                    "\n{:02}  {}{}  {}",
                    n + 1,
                    t.title,
                    if t.artist.is_empty() {
                        String::new()
                    } else {
                        format!(" — {}", t.artist)
                    },
                    time(e.duration)
                );
            }
        }
        s
    };
    json!({"title":c.name,"spine":c.name,"sideA":side(0),"sideB":side(1)})
}
fn create(c: &Collection, tracks: &[Track]) -> Value {
    let v = values(c, tracks);
    let mut d = json!({"version":2,"front":65,"spine":12,"flap":25,"height":100,"extensions":[],"paper":"a4","margin":5,"background":"#fffdf7","backBackground":"#ffffff","backElements":[],"elements":[box_text("title",41.,6.,57.,16.,text(&v,"title").into(),17.,"title"),box_text("spine",28.,5.,7.,90.,text(&v,"spine").into(),11.,"spine"),box_text("side-a",41.,27.,27.,68.,text(&v,"sideA").into(),8.,"sideA"),box_text("side-b",72.,27.,27.,68.,text(&v,"sideB").into(),8.,"sideB"),box_text("notes",3.,6.,19.,86.,"CASSETTE\nSTUDIO".into(),10.,"manual")]});
    d["elements"][1]["vertical"] = json!(true);
    d
}
fn upgrade(mut d: Value) -> Result<Value, String> {
    if !matches!(d["version"].as_u64(), Some(1 | 2)) {
        return Err("Invalid J-CARD version".into());
    }
    if d["version"] == 1 {
        d["version"] = json!(2);
        d["extensions"] = json!([]);
        d["paper"] = json!("a4");
        d["margin"] = json!(5);
        d["backElements"] = json!([]);
        d["backBackground"] = json!("#ffffff");
        if paper(&d).is_none() {
            d["margin"] = json!(0);
        }
    }
    for (k, lo, hi) in [
        ("front", 30., 100.),
        ("spine", 5., 30.),
        ("flap", 0., 100.),
        ("height", 50., 200.),
    ] {
        let n = num(&d, k);
        if !n.is_finite() || n < lo || n > hi {
            return Err(format!("Invalid {k}"));
        }
    }
    for key in ["elements", "backElements"] {
        let es = d[key].as_array().ok_or("Invalid artwork")?;
        if es.len() > 150 {
            return Err("Maximum 150 elements".into());
        }
        for e in es {
            if text(e, "kind") == "image" {
                let src = text(e, "src");
                let encoded = src
                    .strip_prefix("data:image/png;base64,")
                    .or_else(|| src.strip_prefix("data:image/jpeg;base64,"))
                    .ok_or("Use embedded PNG / JPEG images")?;
                if encoded.len() > 8_000_000
                    || encoded.is_empty()
                    || !encoded
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
                {
                    return Err("Invalid embedded image".into());
                }
            }
            if !matches!(text(e, "kind"), "text" | "rect" | "image")
                || num(e, "w") < 1.
                || num(e, "h") < 1.
            {
                return Err("Invalid element".into());
            }
        }
    }
    if paper(&d).is_none() {
        return Err("Artwork does not fit paper".into());
    }
    Ok(d)
}
impl Card {
    pub fn refresh_latest(&mut self, id: &str, lib: &Library) {
        if self.id == id {
            self.refresh(lib);
        }
    }
    pub fn request_close(&mut self) {
        self.closing = true;
    }
    pub fn new(c: &Collection, tracks: &[Track], stored: Option<Value>) -> Self {
        let (doc, error) = match stored {
            Some(v) => match upgrade(v) {
                Ok(v) => (v, String::new()),
                Err(e) => (create(c, tracks), e),
            },
            None => (create(c, tracks), String::new()),
        };
        let custom_spine = num(&doc, "spine") != 12.;
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            doc,
            back: false,
            selected: None,
            undo: vec![],
            redo: vec![],
            clipboard: None,
            version: 1,
            ack: 0,
            sent: None,
            changed_at: 0.,
            closing: false,
            zoom: 4.,
            pan: Vec2::ZERO,
            view_turns: 0,
            pan_drag: false,
            menu_point: Vec2::ZERO,
            snap: true,
            custom_spine,
            texture: None,
            preview_key: String::new(),
            preview_revision: 0,
            preview_back: false,
            drag: None,
            error,
        }
    }
    fn key(&self) -> &'static str {
        if self.back {
            "backElements"
        } else {
            "elements"
        }
    }
    fn background(&self) -> &'static str {
        if self.back {
            "backBackground"
        } else {
            "background"
        }
    }
    fn elements(&self) -> &Vec<Value> {
        self.doc[self.key()].as_array().unwrap()
    }
    fn mutate(&mut self, f: impl FnOnce(&mut Value)) {
        let before = self.doc.clone();
        f(&mut self.doc);
        if before != self.doc {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.version += 1;
            self.changed_at = 0.;
        }
    }
    pub fn saved(&mut self, version: u64) {
        self.ack = self.ack.max(version);
        self.sent = None;
        self.error.clear();
    }
    pub fn failed(&mut self, _version: u64) {
        self.sent = None;
        self.closing = false;
        self.error = "Save failed / 保存に失敗しました".into();
    }
    pub fn load(&mut self, b: &[u8]) -> Result<(), String> {
        if b.len() > 12 * 1024 * 1024 {
            return Err("File too large".into());
        }
        let d = upgrade(serde_json::from_slice(b).map_err(|e| e.to_string())?)?;
        self.mutate(|doc| *doc = d);
        self.custom_spine = num(&self.doc, "spine") != 12.;
        self.selected = None;
        Ok(())
    }
    pub fn add_image(&mut self, b: Vec<u8>) -> Result<(), String> {
        if b.len() > 6_000_000 {
            return Err("Image too large".into());
        }
        let image = image::load_from_memory(&b).map_err(|e| e.to_string())?;
        let mime = if b.starts_with(b"\x89PNG") {
            "png"
        } else if b.starts_with(b"\xff\xd8") {
            "jpeg"
        } else {
            return Err("Use PNG / JPEG".into());
        };
        let e = json!({"id":uid(),"kind":"image","x":5,"y":5,"w":40,"h":(40.*image.height()as f64/image.width()as f64).clamp(1.,200.),"rotation":0,"src":format!("data:image/{mime};base64,{}",base64::engine::general_purpose::STANDARD.encode(b))});
        self.add(e);
        Ok(())
    }
    fn add(&mut self, mut e: Value) {
        while self.doc["elements"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(self.doc["backElements"].as_array().into_iter().flatten())
            .any(|old| old["id"] == e["id"])
        {
            e["id"] = json!(uid());
        }
        let key = self.key();
        if self.elements().len() >= 150 {
            return;
        }
        self.selected = Some(self.elements().len());
        self.mutate(|d| d[key].as_array_mut().unwrap().push(e));
    }
    fn refresh(&mut self, lib: &Library) {
        if let Some(c) = lib.collections.iter().find(|c| c.id == self.id) {
            let v = values(c, &lib.tracks);
            self.mutate(|d| {
                for key in ["elements", "backElements"] {
                    for e in d[key].as_array_mut().unwrap() {
                        let mut binding = text(e, "binding").to_owned();
                        if binding.is_empty() {
                            binding = match text(e, "id") {
                                "side-a" | "back-a" => "sideA",
                                "side-b" | "back-b" => "sideB",
                                _ => "",
                            }
                            .into();
                        }
                        if let Some(s) = v[&binding].as_str() {
                            e["text"] = json!(s);
                            e["binding"] = json!(binding);
                        }
                    }
                }
            });
        }
    }
    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
            self.version += 1;
            self.selected = None;
        }
    }
    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
            self.version += 1;
            self.selected = None;
        }
    }
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ja: bool,
        _lib: &Library,
        io: &mut Transport,
    ) -> bool {
        let now = ctx.input(|i| i.time);
        let previous_zoom = self.zoom;
        let previous_turns = self.view_turns;
        egui::TopBottomPanel::top("card-toolbar").show(ctx,|ui|{
            if crate::appearance::dialog_header(ui, &format!("J-CARD · {}", self.name), None, ja).close { self.closing = true; }
            ui.horizontal_wrapped(|ui| {
            ui.label(if self.ack==self.version{tr(ja,"保存済み","Saved")}else{tr(ja,"保存中…","Saving…")});
            if ui.button(tr(ja,"元に戻す","Undo")).clicked(){self.undo();}
if ui.button(tr(ja,"やり直す","Redo")).clicked(){self.redo();}
            if ui.button(tr(ja,"曲一覧を更新","Refresh track lists")).clicked(){io.json(Tag::JCardRefresh(self.id.clone()),"GET","/api/library",Value::Null);}
            if ui.button(tr(ja,"文字を追加","Add text")).clicked(){self.add(box_text(&uid(),5.,5.,40.,20.,tr(ja,"文字","Text").into(),12.,"manual"));}
            if ui.button(tr(ja,"画像を追加","Add image")).clicked(){platform::pick(platform::Pick::Artwork,io.tx.clone(),ctx.clone());}
            if ui.button(tr(ja,"四角形を追加","Add rectangle")).clicked(){self.add(json!({"id":uid(),"kind":"rect","x":5,"y":5,"w":30,"h":20,"rotation":0,"color":"#bdcfc6"}));}
        });});
        egui::TopBottomPanel::bottom("card-export").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(tr(ja, "編集データを開く", "Open design"))
                    .clicked()
                {
                    platform::pick(platform::Pick::Design, io.tx.clone(), ctx.clone());
                }
                if ui
                    .button(tr(ja, "編集データを書き出す", "Export editable file"))
                    .clicked()
                {
                    platform::save(
                        format!("{}.jcard.json", self.name),
                        serde_json::to_vec_pretty(&self.doc).unwrap(),
                        io.tx.clone(),
                        ctx.clone(),
                    );
                }
                if ui.button("SVG").clicked() {
                    platform::save(
                        format!(
                            "{}-{}.svg",
                            self.name,
                            if self.back { "inside" } else { "outside" }
                        ),
                        svg(&self.doc, self.back, false).into_bytes(),
                        io.tx.clone(),
                        ctx.clone(),
                    );
                }
                if ui.button("PDF (A4 / Letter)").clicked() {
                    match pdf(&self.doc) {
                        Ok(b) => platform::save(
                            format!("{}.pdf", self.name),
                            b,
                            io.tx.clone(),
                            ctx.clone(),
                        ),
                        Err(e) => self.error = e,
                    }
                }
            });
            ui.small(tr(
                ja,
                "PDFは両面・実寸です。切断／折り位置のガイドを含みます。印刷は100%で。",
                "Two-sided actual-size PDF with cutting/folding guides. Print at 100%.",
            ));
            if io.busy() {
                ui.spinner();
            }
            if !self.error.is_empty() {
                ui.colored_label(Color32::DARK_RED, &self.error);
            }
        });
        egui::SidePanel::left("card-dimensions")
            .default_width(190.)
            .min_width(190.)
            .resizable(true)
            .show(ctx, |ui| {
                let previous_face = self.back;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.back, false, tr(ja, "外側", "Outside"));
                    ui.selectable_value(&mut self.back, true, tr(ja, "内側", "Inside"));
                });
                if self.back != previous_face {
                    self.selected = None;
                    self.drag = None;
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(tr(ja, "表示の向き", "View rotation"));
                    ui.horizontal(|ui| {
                        if ui
                            .button(tr(ja, "左90°", "Left 90°"))
                            .on_hover_text(tr(ja, "左へ90度回転", "Rotate view left 90°"))
                            .clicked()
                        {
                            self.view_turns = (self.view_turns + 3) % 4;
                            self.drag = None;
                        }
                        if ui
                            .button(tr(ja, "右90°", "Right 90°"))
                            .on_hover_text(tr(ja, "右へ90度回転", "Rotate view right 90°"))
                            .clicked()
                        {
                            self.view_turns = (self.view_turns + 1) % 4;
                            self.drag = None;
                        }
                    });
                    ui.label(format!("{}°", self.view_turns as u16 * 90));
                    ui.small(tr(
                        ja,
                        "編集表示だけを回転します。印刷の向きは変わりません。",
                        "Rotates the editing view only; print orientation stays the same.",
                    ));
                    ui.label(tr(ja, "ズーム", "Zoom"));
                    ui.add(crate::appearance::slider(
                        egui::Slider::new(
                            &mut self.zoom,
                            crate::jcard_view::MIN..=crate::jcard_view::MAX,
                        )
                        .show_value(false),
                    ));
                    ui.label(format!(
                        "{:.0}%",
                        self.zoom / crate::jcard_view::NORMAL * 100.
                    ));
                    ui.small(tr(
                        ja,
                        "編集領域：ホイールでズーム、中央ドラッグ／Space＋ドラッグで移動。",
                        "Canvas: wheel to zoom; middle-drag / Space-drag to pan.",
                    ));
                    ui.checkbox(&mut self.snap, tr(ja, "0.5mmに吸着", "Snap to 0.5 mm"));
                    let mut next = self.doc.clone();
                    ui.label(tr(ja, "用紙", "Paper"));
                    let mut p = text(&next, "paper").to_owned();
                    egui::ComboBox::from_id_salt("card-paper")
                        .selected_text(&p)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut p, "a4".into(), "A4");
                            ui.selectable_value(&mut p, "letter".into(), "Letter");
                        });
                    next["paper"] = json!(p);
                    for (k, label, lo, hi) in [
                        (
                            "margin",
                            tr(ja, "用紙余白（mm）", "Paper margin (mm)"),
                            0.,
                            20.,
                        ),
                        (
                            "front",
                            tr(ja, "表紙幅（mm）", "Front width (mm)"),
                            30.,
                            100.,
                        ),
                        (
                            "flap",
                            tr(ja, "折り返し幅（mm）", "Flap width (mm)"),
                            0.,
                            100.,
                        ),
                        ("height", tr(ja, "高さ（mm）", "Height (mm)"), 50., 200.),
                    ] {
                        let mut n = num(&next, k);
                        ui.label(label);
                        ui.add(egui::DragValue::new(&mut n).range(lo..=hi).speed(0.5));
                        next[k] = json!(n);
                    }
                    ui.label(tr(ja, "背幅", "Spine width"));
                    let before = self.custom_spine;
                    egui::ComboBox::from_id_salt("card-spine-mode")
                        .selected_text(if self.custom_spine {
                            tr(ja, "カスタム", "Custom")
                        } else {
                            tr(ja, "標準（12mm）", "Standard (12 mm)")
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.custom_spine,
                                false,
                                tr(ja, "標準（12mm）", "Standard (12 mm)"),
                            );
                            ui.selectable_value(
                                &mut self.custom_spine,
                                true,
                                tr(ja, "カスタム", "Custom"),
                            );
                        });
                    if before && !self.custom_spine {
                        next["spine"] = json!(12.);
                    }
                    let mut spine = num(&next, "spine");
                    ui.add_enabled(
                        self.custom_spine,
                        egui::DragValue::new(&mut spine)
                            .range(5.0..=30.)
                            .speed(0.5)
                            .suffix(" mm"),
                    );
                    next["spine"] = json!(spine);
                    ui.separator();
                    ui.label(tr(ja, "追加の折り返し", "Extra fold-out panels"));
                    let panels = next["extensions"].as_array_mut().unwrap();
                    let mut remove = None;
                    for (i, w) in panels.iter_mut().enumerate() {
                        let mut n = w.as_f64().unwrap_or(65.);
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i + 1));
                            ui.add_sized(
                                [76., 26.],
                                egui::DragValue::new(&mut n)
                                    .range(10.0..=100.)
                                    .speed(0.5)
                                    .suffix(" mm"),
                            );
                            if ui
                                .button(tr(ja, "削除", "Remove"))
                                .on_hover_text(tr(
                                    ja,
                                    "この折り返しを削除",
                                    "Remove this fold-out panel",
                                ))
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                        *w = json!(n);
                    }
                    if let Some(i) = remove {
                        panels.remove(i);
                    }
                    if ui
                        .button(tr(ja, "＋ 折り返しを追加", "+ Add fold-out panel"))
                        .clicked()
                    {
                        panels.push(json!(65.));
                    }
                    if next != self.doc {
                        if paper(&next).is_some() {
                            let delta = width(&next)
                                - num(&next, "flap")
                                - num(&next, "spine")
                                - num(&next, "front")
                                - (width(&self.doc)
                                    - num(&self.doc, "flap")
                                    - num(&self.doc, "spine")
                                    - num(&self.doc, "front"));
                            if delta != 0.0 {
                                for e in next["backElements"].as_array_mut().unwrap() {
                                    e["x"] = json!(num(e, "x") + delta);
                                }
                            }
                            self.mutate(|d| *d = next);
                        } else {
                            self.error = tr(
                                ja,
                                "選択した用紙に実寸で収まりません",
                                "Does not fit paper at actual size",
                            )
                            .into();
                        }
                    }
                    ui.separator();
                    let layers = self.elements().clone();
                    for (i, e) in layers.iter().enumerate() {
                        if ui
                            .selectable_label(
                                self.selected == Some(i),
                                format!(
                                    "{} {}",
                                    i + 1,
                                    if text(e, "kind") == "text" {
                                        text(e, "text").lines().next().unwrap_or("Text")
                                    } else {
                                        text(e, "kind")
                                    }
                                ),
                            )
                            .clicked()
                        {
                            self.selected = Some(i);
                        }
                    }
                });
            });
        egui::SidePanel::right("card-properties")
            .default_width(230.)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(tr(ja, "切り取り", "Cut")).clicked() {
                            self.copy(true);
                        }
                        if ui.button(tr(ja, "コピー", "Copy")).clicked() {
                            self.copy(false);
                        }
                        if ui.button(tr(ja, "貼り付け", "Paste")).clicked() {
                            self.paste();
                        }
                    });
                    if let Some(index) = self.selected.filter(|i| *i < self.elements().len()) {
                        let mut e = self.elements()[index].clone();
                        let key = self.key();
                        let mut locked = e["locked"].as_bool().unwrap_or(false);
                        if ui
                            .checkbox(
                                &mut locked,
                                tr(ja, "位置・サイズを固定", "Lock position / size"),
                            )
                            .changed()
                        {
                            e["locked"] = json!(locked);
                        }
                        ui.add_enabled_ui(!locked, |ui| {
                            for (k, label, lo, hi, step) in [
                                ("x", "X (mm)", -300., 300., 0.5),
                                ("y", "Y (mm)", -300., 300., 0.5),
                                ("w", tr(ja, "幅（mm）", "Width (mm)"), 1., 300., 0.5),
                                ("h", tr(ja, "高さ（mm）", "Height (mm)"), 1., 300., 0.5),
                                (
                                    "rotation",
                                    tr(ja, "回転（°）", "Rotation (°)"),
                                    -360.,
                                    360.,
                                    1.,
                                ),
                            ] {
                                ui.label(label);
                                let mut n = num(&e, k);
                                if ui
                                    .add(egui::DragValue::new(&mut n).range(lo..=hi).speed(step))
                                    .changed()
                                {
                                    e[k] = json!(n);
                                }
                            }
                        });
                        ui.small(tr(
                            ja,
                            "枠をドラッグして移動。四隅・辺でサイズ変更、上の丸で回転。",
                            "Drag frame to move, corners/edges to resize, top circle to rotate.",
                        ));
                        if text(&e, "kind") != "image" {
                            let mut color = parse_color(text(&e, "color"));
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                e["color"] = json!(hex(color));
                            }
                        }
                        if text(&e, "kind") == "text" {
                            let mut s = text(&e, "text").to_owned();
                            if ui
                                .add(egui::TextEdit::multiline(&mut s).desired_rows(7))
                                .changed()
                            {
                                e["text"] = json!(s);
                            }
                            for (k, label, lo, hi) in [
                                (
                                    "size",
                                    tr(ja, "文字サイズ（pt）", "Font size (pt)"),
                                    4.,
                                    100.,
                                ),
                                ("lineHeight", tr(ja, "行間", "Line height"), 0.8, 3.),
                            ] {
                                ui.label(label);
                                let mut n = num(&e, k);
                                if ui
                                    .add(egui::DragValue::new(&mut n).range(lo..=hi).speed(0.1))
                                    .changed()
                                {
                                    e[k] = json!(n);
                                }
                            }
                            for (k, label) in [
                                ("bold", tr(ja, "太字", "Bold")),
                                ("vertical", tr(ja, "縦書き", "Vertical")),
                            ] {
                                let mut b = e[k].as_bool().unwrap_or(false);
                                if ui.checkbox(&mut b, label).changed() {
                                    e[k] = json!(b);
                                }
                            }
                            for (k, choices) in [
                                ("align", vec!["left", "center", "right"]),
                                ("font", vec!["sans", "serif", "mono"]),
                                (
                                    "binding",
                                    vec!["manual", "title", "spine", "sideA", "sideB"],
                                ),
                            ] {
                                let mut s = text(&e, k).to_owned();
                                ui.label(match k {
                                    "align" => tr(ja, "文字揃え", "Alignment"),
                                    "font" => tr(ja, "書体", "Font"),
                                    _ => tr(ja, "連動する内容", "Linked content"),
                                });
                                egui::ComboBox::from_id_salt(k)
                                    .selected_text(choice_label(ja, &s))
                                    .show_ui(ui, |ui| {
                                        for c in choices {
                                            ui.selectable_value(
                                                &mut s,
                                                c.into(),
                                                choice_label(ja, c),
                                            );
                                        }
                                    });
                                if text(&e, k) != s {
                                    e[k] = json!(s);
                                }
                            }
                        }
                        if e != self.elements()[index] {
                            self.mutate(|d| d[key][index] = e);
                        }
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_enabled(
                                    !locked,
                                    egui::Button::new(tr(ja, "左右中央", "Center horizontally")),
                                )
                                .clicked()
                            {
                                self.center(true);
                            }
                            if ui
                                .add_enabled(
                                    !locked,
                                    egui::Button::new(tr(ja, "上下中央", "Center vertically")),
                                )
                                .clicked()
                            {
                                self.center(false);
                            }
                            if ui.button(tr(ja, "複製", "Duplicate")).clicked() {
                                self.duplicate_selected();
                            }
                            if ui.button(tr(ja, "削除", "Delete")).clicked() {
                                self.mutate(|d| {
                                    d[key].as_array_mut().unwrap().remove(index);
                                });
                                self.selected = None;
                            }
                            if ui.button("↑").clicked() && index + 1 < self.elements().len() {
                                self.mutate(|d| {
                                    d[key].as_array_mut().unwrap().swap(index, index + 1)
                                });
                                self.selected = Some(index + 1);
                            }
                            if ui.button("↓").clicked() && index > 0 {
                                self.mutate(|d| {
                                    d[key].as_array_mut().unwrap().swap(index, index - 1)
                                });
                                self.selected = Some(index - 1);
                            }
                        });
                    } else {
                        ui.label(tr(ja, "要素を選択してください", "Select an element"));
                    }
                    ui.separator();
                    ui.label(tr(ja, "背景色", "Background"));
                    let key = self.background();
                    let mut color = parse_color(text(&self.doc, key));
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        self.mutate(|d| d[key] = json!(hex(color)));
                    }
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            let view = ui.available_rect_before_wrap();
            ui.set_clip_rect(ui.clip_rect().intersect(view));
            let response = ui.allocate_rect(view, egui::Sense::CLICK | egui::Sense::DRAG);
            if response.contains_pointer()
                && ctx.input(|i| i.pointer.any_pressed())
                && !egui::Popup::is_any_open(ctx)
            {
                ctx.memory_mut(|memory| {
                    if let Some(id) = memory.focused() {
                        memory.surrender_focus(id);
                    }
                });
            }
            platform::editor_viewport(Some(view), ctx.pixels_per_point());
            let page = Vec2::new(width(&self.doc) as f32, num(&self.doc, "height") as f32);
            let turns = self.view_turns;
            if previous_zoom != self.zoom || previous_turns != turns {
                let anchor = crate::jcard_view::from_view(
                    (view.size() * 0.5 - self.pan) / previous_zoom,
                    page,
                    previous_turns,
                );
                self.pan =
                    view.size() * 0.5 - crate::jcard_view::to_view(anchor, page, turns) * self.zoom;
            }
            let space = ctx.input(|i| i.key_down(egui::Key::Space)) && !ctx.wants_keyboard_input();
            if response.hovered()
                && !egui::Popup::is_any_open(ctx)
                && self.drag.is_none()
                && !ctx.input(|i| i.pointer.any_down())
            {
                let (wheel, pinch, pos) =
                    ctx.input(|i| (i.raw_scroll_delta.y, i.zoom_delta(), i.pointer.hover_pos()));
                if let Some(pos) = pos
                    && (wheel != 0. || pinch != 1.)
                {
                    let factor = if pinch != 1. {
                        pinch
                    } else {
                        (wheel.clamp(-240., 240.) * 0.0025).exp()
                    };
                    let zoom =
                        (self.zoom * factor).clamp(crate::jcard_view::MIN, crate::jcard_view::MAX);
                    self.pan =
                        crate::jcard_view::zoom_at(self.pan, self.zoom, zoom, pos - view.min);
                    self.zoom = zoom;
                    ctx.input_mut(|i| {
                        i.raw_scroll_delta = Vec2::ZERO;
                        i.smooth_scroll_delta = Vec2::ZERO;
                    });
                }
            }
            if response.double_clicked_by(egui::PointerButton::Middle) {
                self.fit_view(view);
            }
            if response.drag_started_by(egui::PointerButton::Middle)
                || (space && response.drag_started_by(egui::PointerButton::Primary))
            {
                self.pan_drag = true;
                self.drag = None;
            }
            if self.pan_drag {
                if ctx.input(|i| {
                    i.pointer.button_down(egui::PointerButton::Middle) || i.pointer.primary_down()
                }) {
                    self.pan += ctx.input(|i| i.pointer.delta());
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                } else {
                    self.pan_drag = false;
                }
            } else if response.hovered() && space {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
            let size = crate::jcard_view::rotated_size(page, turns) * self.zoom;
            self.pan = crate::jcard_view::constrain_pan(self.pan, size, view.size());
            if self.preview_revision != self.version
                || self.preview_back != self.back
                || self.texture.is_none()
            {
                let svg = svg(&self.doc, self.back, false);
                if self.preview_key != svg {
                    match raster(&svg) {
                        Ok((w, h, pixels)) => {
                            self.texture = Some(ctx.load_texture(
                                "jcard-preview",
                                egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels),
                                egui::TextureOptions::LINEAR,
                            ));
                            self.preview_key = svg;
                        }
                        Err(e) => self.error = e,
                    }
                }
                self.preview_revision = self.version;
                self.preview_back = self.back;
            }
            let rect = egui::Rect::from_min_size(view.min + self.pan, size);
            let zoom = self.zoom;
            let to_pos = |x: f64, y: f64| {
                rect.min
                    + crate::jcard_view::to_view(Vec2::new(x as f32, y as f32), page, turns) * zoom
            };
            let from_pos =
                |p: Pos2| crate::jcard_view::from_view((p - rect.min) / zoom, page, turns);
            if let Some(t) = &self.texture {
                let mut mesh = egui::Mesh::with_texture(t.id());
                for uv in [
                    Pos2::new(0., 0.),
                    Pos2::new(1., 0.),
                    Pos2::new(1., 1.),
                    Pos2::new(0., 1.),
                ] {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: to_pos((uv.x * page.x) as f64, (uv.y * page.y) as f64),
                        uv,
                        color: Color32::WHITE,
                    });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                ui.painter().add(egui::Shape::mesh(mesh));
            }
            if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
                let e = &self.elements()[i];
                let corners = corners(e);
                for n in 0..4 {
                    ui.painter().line_segment(
                        [
                            to_pos(corners[n].0, corners[n].1),
                            to_pos(corners[(n + 1) % 4].0, corners[(n + 1) % 4].1),
                        ],
                        Stroke::new(1.0_f32, Color32::from_rgb(40, 125, 145)),
                    );
                }
                if e["locked"] != true {
                    for (x, y) in handles(e) {
                        ui.painter().rect_filled(
                            egui::Rect::from_center_size(to_pos(x, y), Vec2::splat(7.)),
                            0.,
                            Color32::from_rgb(40, 125, 145),
                        );
                    }
                    let r = rotate(num(e, "x") + num(e, "w") / 2., num(e, "y") - 6., e);
                    ui.painter().circle_filled(
                        to_pos(r.0, r.1),
                        4.,
                        Color32::from_rgb(40, 125, 145),
                    );
                } else {
                    ui.painter().text(
                        to_pos(num(e, "x"), num(e, "y")) - Vec2::new(0., 16.),
                        egui::Align2::LEFT_TOP,
                        tr(ja, "固定", "Locked"),
                        egui::FontId::proportional(12.),
                        Color32::from_rgb(40, 125, 145),
                    );
                }
            }
            if response.clicked()
                && !space
                && !self.pan_drag
                && let Some(p) = response.interact_pointer_pos()
            {
                let point = from_pos(p);
                let (x, y) = (point.x as f64, point.y as f64);
                self.selected = self.selection_hit(x, y);
            }
            if response.drag_started_by(egui::PointerButton::Primary)
                && self.drag.is_none()
                && !space
                && !self.pan_drag
                && let Some(p) = ctx
                    .input(|i| i.pointer.press_origin())
                    .or(response.interact_pointer_pos())
            {
                let point = from_pos(p);
                let (x, y) = (point.x as f64, point.y as f64);
                let mut hit = None;
                let mut mode = "move".to_owned();
                if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
                    let e = &self.elements()[i];
                    let radius = 8.0 / self.zoom as f64;
                    if let Some((n, _)) = handles(e)
                        .into_iter()
                        .enumerate()
                        .map(|(n, (cx, cy))| (n, (cx - x).hypot(cy - y)))
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .filter(|(_, distance)| *distance <= radius)
                    {
                        hit = Some(i);
                        mode = format!("resize{n}");
                    }
                    let r = rotate(num(e, "x") + num(e, "w") / 2., num(e, "y") - 6., e);
                    if (r.0 - x).hypot(r.1 - y) <= radius {
                        hit = Some(i);
                        mode = "rotate".into();
                    }
                }
                if hit.is_none() {
                    hit = self.selection_hit(x, y);
                }
                self.selected = hit;
                if let Some(i) = hit.filter(|i| self.elements()[*i]["locked"] != true) {
                    self.undo.push(self.doc.clone());
                    self.redo.clear();
                    self.drag = Some((self.elements()[i].clone(), p, mode));
                }
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && !self.pan_drag
                && let (Some((before, start, mode)), Some(i), Some(p)) = (
                    self.drag.clone(),
                    self.selected,
                    response.interact_pointer_pos(),
                )
            {
                let delta = crate::jcard_view::delta_from_view((p - start) / zoom, turns);
                let mut e = before.clone();
                let snap = |v: f64| if self.snap { (v * 2.).round() / 2. } else { v };
                if mode == "move" {
                    e["x"] = json!(snap(num(&before, "x") + delta.x as f64).clamp(-300., 300.));
                    e["y"] = json!(snap(num(&before, "y") + delta.y as f64).clamp(-300., 300.));
                } else if mode == "rotate" {
                    let center = to_pos(
                        num(&before, "x") + num(&before, "w") / 2.,
                        num(&before, "y") + num(&before, "h") / 2.,
                    );
                    let mut angle =
                        (p - center).angle().to_degrees() as f64 + 90. - turns as f64 * 90.;
                    if ctx.input(|i| i.modifiers.shift) {
                        angle = (angle / 15.).round() * 15.;
                    }
                    e["rotation"] = json!(angle);
                } else {
                    let angle = num(&before, "rotation").to_radians();
                    let dx = delta.x as f64 * angle.cos() + delta.y as f64 * angle.sin();
                    let dy = -delta.x as f64 * angle.sin() + delta.y as f64 * angle.cos();
                    let corner = mode.chars().last().unwrap_or('2');
                    let sx = if matches!(corner, '0' | '6' | '7') {
                        -1.
                    } else if matches!(corner, '2' | '3' | '4') {
                        1.
                    } else {
                        0.
                    };
                    let sy = if matches!(corner, '0' | '1' | '2') {
                        -1.
                    } else if matches!(corner, '4' | '5' | '6') {
                        1.
                    } else {
                        0.
                    };
                    let w = snap(num(&before, "w") + sx * dx).clamp(1., 300.);
                    let h = snap(num(&before, "h") + sy * dy).clamp(1., 300.);
                    let shift_x = sx * (w - num(&before, "w")) / 2.;
                    let shift_y = sy * (h - num(&before, "h")) / 2.;
                    let cx = num(&before, "x") + num(&before, "w") / 2. + shift_x * angle.cos()
                        - shift_y * angle.sin();
                    let cy = num(&before, "y")
                        + num(&before, "h") / 2.
                        + shift_x * angle.sin()
                        + shift_y * angle.cos();
                    e["x"] = json!(cx - w / 2.);
                    e["y"] = json!(cy - h / 2.);
                    e["w"] = json!(w);
                    e["h"] = json!(h);
                }
                let key = self.key();
                if self.doc[key][i] != e {
                    self.doc[key][i] = e;
                    self.version += 1;
                    self.changed_at = 0.;
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                self.drag = None;
            }
            if response.secondary_clicked()
                && let Some(p) = response.interact_pointer_pos()
            {
                self.menu_point = from_pos(p);
                self.selected =
                    self.selection_hit(self.menu_point.x as f64, self.menu_point.y as f64);
            }
            response.context_menu(|ui| self.canvas_menu(ui, view, io, ctx, ja));
        });
        if !ctx.wants_keyboard_input() {
            let (copy, cut, paste, undo, redo) = ctx.input_mut(|i| {
                let command = egui::Modifiers::COMMAND;
                let redo = i.consume_shortcut(&egui::KeyboardShortcut::new(
                    command | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )) || i
                    .consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::Y));
                let undo = i.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::Z));
                let copy = i.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::C))
                    || i.events.iter().any(|e| matches!(e, egui::Event::Copy));
                let cut = i.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::X))
                    || i.events.iter().any(|e| matches!(e, egui::Event::Cut));
                let paste = i.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::V))
                    || i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
                (copy, cut, paste, undo, redo)
            });
            if copy {
                self.copy(false);
            }
            if cut {
                self.copy(true);
            }
            if paste {
                self.paste();
            }
            if undo {
                self.undo();
            }
            if redo {
                self.redo();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape))
                && let (Some((before, _, _)), Some(index)) = (self.drag.take(), self.selected)
            {
                let key = self.key();
                self.doc[key][index] = before;
                self.undo.pop();
                self.version += 1;
            }
        }
        if self.version > self.ack && self.sent.is_none() {
            if self.changed_at == 0. {
                self.changed_at = now;
            }
            if self.closing || now - self.changed_at > 0.6 {
                self.sent = Some(self.version);
                io.json(
                    Tag::JCardSaved(self.version),
                    "PUT",
                    &format!("/api/jcards/{}", self.id),
                    self.doc.clone(),
                );
            }
        }
        self.closing && self.ack == self.version
    }
    fn copy(&mut self, cut: bool) {
        if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
            self.clipboard = Some(Clipboard {
                item: self.elements()[i].clone(),
                panels: panels(&self.doc, self.back),
                back: self.back,
                cassette: self.id.clone(),
                cut,
                pasted: 0,
            });
            if cut {
                let key = self.key();
                self.mutate(|d| {
                    d[key].as_array_mut().unwrap().remove(i);
                });
                self.selected = None;
            }
        }
    }
    fn paste(&mut self) {
        if let Some(c) = self.clipboard.clone() {
            let mut e = c.item.clone();
            e["id"] = json!(uid());
            e["locked"] = json!(false);
            let center = num(&e, "x") + num(&e, "w") / 2.;
            if let Some((key, x, _)) = c
                .panels
                .iter()
                .find(|(_, x, w)| *w > 0. && center >= *x && center <= *x + *w)
                && let Some((_, target, _)) = panels(&self.doc, self.back)
                    .iter()
                    .find(|(k, _, _)| k == key)
            {
                e["x"] = json!(target + num(&e, "x") - x);
            }
            let offset = 2. * c.pasted as f64
                + if c.cassette == self.id && c.back == self.back && !c.cut {
                    2.
                } else {
                    0.
                };
            e["x"] =
                json!((num(&e, "x") + offset).clamp(0., (width(&self.doc) - num(&e, "w")).max(0.)));
            e["y"] = json!(
                (num(&e, "y") + offset)
                    .clamp(0., (num(&self.doc, "height") - num(&e, "h")).max(0.))
            );
            self.add(e);
            if let Some(c) = &mut self.clipboard {
                c.pasted += 1;
            }
        }
    }
}
fn rotate(x: f64, y: f64, e: &Value) -> (f64, f64) {
    let a = num(e, "rotation").to_radians();
    let (cx, cy) = (
        num(e, "x") + num(e, "w") / 2.,
        num(e, "y") + num(e, "h") / 2.,
    );
    (
        cx + (x - cx) * a.cos() - (y - cy) * a.sin(),
        cy + (x - cx) * a.sin() + (y - cy) * a.cos(),
    )
}
fn unrotate(x: f64, y: f64, e: &Value) -> (f64, f64) {
    let mut e = e.clone();
    e["rotation"] = json!(-num(&e, "rotation"));
    rotate(x, y, &e)
}
fn corners(e: &Value) -> [(f64, f64); 4] {
    let (x, y, w, h) = (num(e, "x"), num(e, "y"), num(e, "w"), num(e, "h"));
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|(x, y)| rotate(x, y, e))
}
fn handles(e: &Value) -> [(f64, f64); 8] {
    let [a, b, c, d] = corners(e);
    let mid = |a: (f64, f64), b: (f64, f64)| ((a.0 + b.0) / 2., (a.1 + b.1) / 2.);
    [a, mid(a, b), b, mid(b, c), c, mid(c, d), d, mid(d, a)]
}
fn parse_color(s: &str) -> Color32 {
    if s.len() == 7
        && s.starts_with('#')
        && let Ok(n) = u32::from_str_radix(&s[1..], 16)
    {
        return Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8);
    }
    Color32::WHITE
}
fn hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}
pub fn svg(d: &Value, back: bool, print: bool) -> String {
    let (w, h) = (width(d), num(d, "height"));
    let (pw, ph) = if print {
        paper(d).unwrap_or((210., 297.))
    } else {
        (w, h)
    };
    let (ox, oy) = ((pw - w) / 2., (ph - h) / 2.);
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{pw}mm\" height=\"{ph}mm\" viewBox=\"0 0 {pw} {ph}\"><defs><clipPath id=\"card\"><rect width=\"{w}\" height=\"{h}\"/></clipPath></defs><g transform=\"translate({ox} {oy})\"><g clip-path=\"url(#card)\"><rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>",
        xml(text(d, if back { "backBackground" } else { "background" }))
    );
    for (i, e) in d[if back { "backElements" } else { "elements" }]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let (x, y, ew, eh) = (num(e, "x"), num(e, "y"), num(e, "w"), num(e, "h"));
        s += &format!(
            "<g transform=\"rotate({} {} {})\"><defs><clipPath id=\"e{i}\"><rect x=\"{x}\" y=\"{y}\" width=\"{ew}\" height=\"{eh}\"/></clipPath></defs><g clip-path=\"url(#e{i})\">",
            num(e, "rotation"),
            x + ew / 2.,
            y + eh / 2.
        );
        match text(e, "kind") {
            "rect" => {
                s += &format!(
                    "<rect x=\"{x}\" y=\"{y}\" width=\"{ew}\" height=\"{eh}\" fill=\"{}\"/>",
                    xml(text(e, "color"))
                )
            }
            "image" => {
                s += &format!(
                    "<image x=\"{x}\" y=\"{y}\" width=\"{ew}\" height=\"{eh}\" xlink:href=\"{}\" preserveAspectRatio=\"xMidYMid meet\"/>",
                    xml(text(e, "src"))
                )
            }
            "text" => {
                let size = num(e, "size") * 25.4 / 72.;
                let line = size * num(e, "lineHeight");
                let weight = if e["bold"] == true { 700 } else { 400 };
                let stroke = if e["bold"] == true {
                    xml(text(e, "color"))
                } else {
                    "none".into()
                };
                let stroke_width = if e["bold"] == true { size * 0.025 } else { 0. };
                let family = match text(e, "font") {
                    "serif" => "Noto Serif JP",
                    "mono" => "Hack",
                    _ => "Noto Sans JP",
                };
                let vertical = e["vertical"] == true;
                let limit = if vertical { eh } else { ew } / size.max(0.1);
                let mut lines = vec![];
                for para in text(e, "text").split('\n') {
                    let mut l = String::new();
                    let mut n = 0.;
                    for c in para.chars() {
                        let a = if c.is_ascii() { 0.58 } else { 1. };
                        if n + a > limit && !l.is_empty() {
                            lines.push(l);
                            l = String::new();
                            n = 0.;
                        }
                        l.push(c);
                        n += a;
                    }
                    lines.push(l);
                }
                for (n, l) in lines.iter().enumerate() {
                    let align = text(e, "align");
                    let (tx, anchor) = match align {
                        "center" => (x + ew / 2., "middle"),
                        "right" => (x + ew, "end"),
                        _ => (x, "start"),
                    };
                    if vertical {
                        let tx = x + ew - size / 2. - n as f64 * line;
                        for (j, c) in l.chars().enumerate() {
                            s += &format!(
                                "<text x=\"{tx}\" y=\"{}\" text-anchor=\"middle\" font-family=\"{family}\" font-size=\"{size}\" font-weight=\"{weight}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" fill=\"{}\">{}</text>",
                                y + size + j as f64 * size,
                                xml(text(e, "color")),
                                xml(&c.to_string())
                            );
                        }
                    } else {
                        s += &format!(
                            "<text x=\"{tx}\" y=\"{}\" text-anchor=\"{anchor}\" font-family=\"{family}\" font-size=\"{size}\" font-weight=\"{weight}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" fill=\"{}\">{}</text>",
                            y + size + n as f64 * line,
                            xml(text(e, "color")),
                            xml(l)
                        );
                    }
                }
            }
            _ => {}
        }
        s += "</g></g>";
    }
    s += "</g>";
    let mut folds = vec![num(d, "flap"), num(d, "flap") + num(d, "spine")];
    let mut end = num(d, "flap") + num(d, "spine") + num(d, "front");
    for p in d["extensions"].as_array().into_iter().flatten() {
        folds.push(end);
        end += p.as_f64().unwrap_or(0.);
    }
    for f in folds {
        let f = if back { w - f } else { f };
        if f > 0. && f < w {
            s += &format!(
                "<path d=\"M{f} 0 V{h}\" fill=\"none\" stroke=\"#89928d\" stroke-width=\"0.12\" stroke-dasharray=\"1 1\"/>"
            );
        }
    }
    if print {
        s += &format!(
            "<rect width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"#444444\" stroke-width=\"0.15\" stroke-dasharray=\"2 1\"/>"
        );
    }
    s += "</g></svg>";
    s
}
fn options() -> svg2pdf::usvg::Options<'static> {
    let mut o = svg2pdf::usvg::Options {
        font_family: "Noto Sans JP".into(),
        ..Default::default()
    };
    o.fontdb_mut()
        .load_font_data(include_bytes!("../../assets/fonts/NotoSansJP.ttf").to_vec());
    o.fontdb_mut()
        .load_font_data(include_bytes!("../../assets/fonts/NotoSerifJP.ttf").to_vec());
    o.fontdb_mut()
        .load_font_data(epaint_default_fonts::HACK_REGULAR.to_vec());
    o.fontdb_mut()
        .load_font_data(epaint_default_fonts::NOTO_EMOJI_REGULAR.to_vec());
    o
}
fn raster(s: &str) -> Result<(usize, usize, Vec<u8>), String> {
    let tree = svg2pdf::usvg::Tree::from_str(s, &options()).map_err(|e| e.to_string())?;
    let scale = 1.6;
    let size = tree.size();
    let mut p = resvg::tiny_skia::Pixmap::new(
        (size.width() * scale).ceil() as u32,
        (size.height() * scale).ceil() as u32,
    )
    .ok_or("Image too large")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut p.as_mut(),
    );
    Ok((p.width() as usize, p.height() as usize, p.take()))
}
pub fn pdf(d: &Value) -> Result<Vec<u8>, String> {
    let mut merged = lopdf::Document::with_version("1.7");
    let pages_id = merged.new_object_id();
    let mut kids = vec![];
    for back in [false, true] {
        let tree = svg2pdf::usvg::Tree::from_str(&svg(d, back, true), &options())
            .map_err(|e| e.to_string())?;
        let bytes = svg2pdf::to_pdf(
            &tree,
            Default::default(),
            svg2pdf::PageOptions { dpi: 96.0 },
        )
        .map_err(|e| e.to_string())?;
        let mut page = lopdf::Document::load_mem(&bytes).map_err(|e| e.to_string())?;
        page.renumber_objects_with(merged.max_id + 1);
        let id = *page.get_pages().values().next().ok_or("No PDF page")?;
        page.get_object_mut(id)
            .and_then(lopdf::Object::as_dict_mut)
            .map_err(|e| e.to_string())?
            .set("Parent", pages_id);
        kids.push(lopdf::Object::Reference(id));
        merged.max_id = page.max_id;
        merged.objects.extend(page.objects);
    }
    merged.objects.insert(
        pages_id,
        lopdf::Object::Dictionary(lopdf::dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>2}),
    );
    let root = merged.add_object(lopdf::dictionary! {"Type"=>"Catalog","Pages"=>pages_id});
    merged.trailer.set("Root", root);
    let mut out = vec![];
    merged.save_to(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stacking_order_is_undoable_and_matches_svg_paint_order() {
        let c = Collection {
            id: "layers".into(),
            name: "Layers".into(),
            kind: "cassette".into(),
            tracks: vec![],
            sides: [vec![], vec![]],
            settings: Default::default(),
        };
        let mut card = Card::new(&c, &[], None);
        card.doc["elements"] = json!([
            {"id":"red","kind":"rect","x":1,"y":1,"w":10,"h":10,"rotation":0,"color":"#ff0000"},
            {"id":"blue","kind":"rect","x":1,"y":1,"w":10,"h":10,"rotation":0,"color":"#0000ff"}
        ]);
        card.selected = Some(0);
        assert_eq!(card.hit(5., 5.), Some(1));
        assert_eq!(card.selection_hit(5., 5.), Some(0));
        assert_eq!(card.selection_hit(50., 50.), None);
        // Each face has mirrored fold regions. Center within the current region.
        card.doc["flap"] = json!(20.);
        card.doc["spine"] = json!(10.);
        card.doc["front"] = json!(60.);
        card.doc["extensions"] = json!([40.]);
        for (back, x, expected) in [
            (false, 35., 55.),
            (false, 95., 105.),
            (true, 45., 65.),
            (true, 105., 115.),
        ] {
            card.back = back;
            let key = card.key();
            card.doc[key] = json!([{"x":x,"y":1,"w":10,"h":10}]);
            card.selected = Some(0);
            card.center(true);
            assert_eq!(num(&card.elements()[0], "x"), expected);
            card.undo();
            assert_eq!(num(&card.elements()[0], "x"), x);
        }
        card.back = false;
        card.doc["elements"] = json!([
            {"id":"red","kind":"rect","x":1,"y":1,"w":10,"h":10,"rotation":0,"color":"#ff0000"},
            {"id":"blue","kind":"rect","x":1,"y":1,"w":10,"h":10,"rotation":0,"color":"#0000ff"}
        ]);
        let initial = svg(&card.doc, false, false);
        card.pan = Vec2::new(120., -70.);
        card.zoom = 9.;
        assert_eq!(
            svg(&card.doc, false, false),
            initial,
            "Navigation must not change exported geometry"
        );
        card.selected = Some(0);
        card.layer(1);
        assert_eq!(card.doc["elements"][1]["id"], "red");
        let output = svg(&card.doc, false, false);
        assert!(output.find("#0000ff").unwrap() < output.find("#ff0000").unwrap());
        card.undo();
        assert_eq!(svg(&card.doc, false, false), initial);
        card.redo();
        assert_eq!(svg(&card.doc, false, false), output);
        card.doc["elements"][1]["locked"] = json!(true);
        card.selected = Some(1);
        let before = card.doc.clone();
        card.center(true);
        assert_eq!(card.doc, before);
        card.duplicate_selected();
        assert_eq!(card.doc["elements"][2]["locked"], false);
    }
    #[test]
    fn blank_inside_and_custom_export() {
        let c = Collection {
            id: "test".into(),
            name: "テスト".into(),
            kind: "cassette".into(),
            tracks: vec![],
            sides: [vec![], vec![]],
            settings: Default::default(),
        };
        let mut d = create(&c, &[]);
        assert!(d["backElements"].as_array().unwrap().is_empty());
        d["spine"] = json!(8.5);
        assert_eq!(width(&d), 98.5);
        let b = pdf(&d).unwrap();
        let p = lopdf::Document::load_mem(&b).unwrap();
        assert_eq!(p.get_pages().len(), 2);
        for id in p.get_pages().values() {
            let page = p.get_object(*id).unwrap().as_dict().unwrap();
            let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
            let width = media[2].as_float().unwrap();
            let height = media[3].as_float().unwrap();
            assert!((width - 595.2756).abs() < 0.05);
            assert!((height - 841.8898).abs() < 0.05);
        }

        assert!(svg(&d, false, true).contains("stroke-dasharray"));
        assert!(
            raster(&svg(&d, false, false))
                .unwrap()
                .2
                .iter()
                .any(|b| *b != 0)
        );
    }
}
