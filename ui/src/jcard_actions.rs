//! Context-menu editing operations; every document mutation uses the undo stack.
use super::*;
impl Card {
    pub(super) fn hit(&self, x: f64, y: f64) -> Option<usize> {
        self.elements()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| {
                let (px, py) = unrotate(x, y, e);
                px >= num(e, "x")
                    && px <= num(e, "x") + num(e, "w")
                    && py >= num(e, "y")
                    && py <= num(e, "y") + num(e, "h")
            })
            .map(|(i, _)| i)
    }
    // Keep an explicitly selected object accessible beneath other artwork.
    pub(super) fn selection_hit(&self, x: f64, y: f64) -> Option<usize> {
        self.selected
            .filter(|i| {
                self.elements().get(*i).is_some_and(|e| {
                    let (px, py) = unrotate(x, y, e);
                    px >= num(e, "x")
                        && px <= num(e, "x") + num(e, "w")
                        && py >= num(e, "y")
                        && py <= num(e, "y") + num(e, "h")
                })
            })
            .or_else(|| self.hit(x, y))
    }
    pub(super) fn layer(&mut self, target: usize) {
        if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
            let target = target.min(self.elements().len() - 1);
            if target == i {
                return;
            }
            let key = self.key();
            self.mutate(|d| {
                let list = d[key].as_array_mut().unwrap();
                let e = list.remove(i);
                list.insert(target, e);
            });
            self.selected = Some(target);
        }
    }
    pub(super) fn center(&mut self, horizontal: bool) {
        if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
            if self.elements()[i]["locked"] == true {
                return;
            }
            let key = self.key();
            let (field, v) = if horizontal {
                ("x", {
                    let e = &self.elements()[i];
                    let cx = num(e, "x") + num(e, "w") / 2.;
                    let regions = panels(&self.doc, self.back);
                    let (_, start, span) = regions
                        .iter()
                        .min_by(|a, b| {
                            let distance = |p: &&(String, f64, f64)| {
                                (p.1 - cx).max(0.) + (cx - p.1 - p.2).max(0.)
                            };
                            distance(a).total_cmp(&distance(b))
                        })
                        .unwrap();
                    start + (span - num(e, "w")) / 2.
                })
            } else {
                (
                    "y",
                    (num(&self.doc, "height") - num(&self.elements()[i], "h")) / 2.,
                )
            };
            self.mutate(|d| d[key][i][field] = json!(v));
        }
    }
    pub(super) fn duplicate_selected(&mut self) {
        if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
            let mut e = self.elements()[i].clone();
            e["id"] = json!(uid());
            e["locked"] = json!(false);
            e["x"] = json!((num(&e, "x") + 2.).clamp(-300., 300.));
            e["y"] = json!((num(&e, "y") + 2.).clamp(-300., 300.));
            self.add(e);
        }
    }
    pub(super) fn delete_selected(&mut self) {
        if let Some(i) = self.selected.filter(|i| *i < self.elements().len()) {
            let key = self.key();
            self.mutate(|d| {
                d[key].as_array_mut().unwrap().remove(i);
            });
            self.selected = None;
        }
    }
    pub(super) fn fit_view(&mut self, view: egui::Rect) {
        (self.zoom, self.pan) = crate::jcard_view::fit(
            crate::jcard_view::rotated_size(
                Vec2::new(width(&self.doc) as f32, num(&self.doc, "height") as f32),
                self.view_turns,
            ),
            view.size(),
        );
    }
    pub(super) fn normal_view(&mut self, view: egui::Rect) {
        self.zoom = crate::jcard_view::NORMAL;
        self.pan = (view.size()
            - crate::jcard_view::rotated_size(
                Vec2::new(width(&self.doc) as f32, num(&self.doc, "height") as f32),
                self.view_turns,
            ) * self.zoom)
            * 0.5;
    }
    pub(super) fn canvas_menu(
        &mut self,
        ui: &mut egui::Ui,
        view: egui::Rect,
        io: &mut Transport,
        ctx: &egui::Context,
        ja: bool,
    ) {
        let selected = self.selected.filter(|i| *i < self.elements().len());
        if selected.is_some() {
            if ui.button(tr(ja, "切り取り", "Cut")).clicked() {
                self.copy(true);
                ui.close();
                return;
            }
            if ui.button(tr(ja, "コピー", "Copy")).clicked() {
                self.copy(false);
                ui.close();
                return;
            }
        }
        if ui
            .add_enabled(
                self.clipboard.is_some(),
                egui::Button::new(tr(ja, "貼り付け", "Paste")),
            )
            .clicked()
        {
            self.paste();
            ui.close();
            return;
        }
        if let Some(i) = selected {
            if ui.button(tr(ja, "複製", "Duplicate")).clicked() {
                self.duplicate_selected();
                ui.close();
                return;
            }
            if ui.button(tr(ja, "削除", "Delete")).clicked() {
                self.delete_selected();
                ui.close();
                return;
            }
            ui.separator();
            ui.menu_button(tr(ja, "重なり順", "Arrange"), |ui| {
                for (label, target) in [
                    (
                        tr(ja, "最前面へ", "Bring to front"),
                        self.elements().len() - 1,
                    ),
                    (
                        tr(ja, "前面へ", "Bring forward"),
                        (i + 1).min(self.elements().len() - 1),
                    ),
                    (tr(ja, "背面へ", "Send backward"), i.saturating_sub(1)),
                    (tr(ja, "最背面へ", "Send to back"), 0),
                ] {
                    if ui
                        .add_enabled(i != target, egui::Button::new(label))
                        .clicked()
                    {
                        self.layer(target);
                        ui.close();
                        return;
                    }
                }
            });
            let locked = self.elements().get(i).is_some_and(|e| e["locked"] == true);
            ui.add_enabled_ui(!locked, |ui| {
                ui.menu_button(tr(ja, "中央揃え", "Center"), |ui| {
                    if ui.button(tr(ja, "左右中央", "Horizontally")).clicked() {
                        self.center(true);
                        ui.close();
                        return;
                    }
                    if ui.button(tr(ja, "上下中央", "Vertically")).clicked() {
                        self.center(false);
                        ui.close();
                    }
                });
                if ui.button(tr(ja, "回転を解除", "Reset rotation")).clicked() {
                    let key = self.key();
                    self.mutate(|d| d[key][i]["rotation"] = json!(0));
                    ui.close();
                }
            });
            if ui
                .button(if locked {
                    tr(ja, "固定解除", "Unlock position / size")
                } else {
                    tr(ja, "位置・サイズを固定", "Lock position / size")
                })
                .clicked()
            {
                let key = self.key();
                self.mutate(|d| d[key][i]["locked"] = json!(!locked));
                self.drag = None;
                ui.close();
                return;
            }
        } else {
            ui.separator();
            let x = (self.menu_point.x as f64).clamp(0., (width(&self.doc) - 40.).max(0.));
            let y = (self.menu_point.y as f64).clamp(0., (num(&self.doc, "height") - 20.).max(0.));
            if ui.button(tr(ja, "文字を追加", "Add text")).clicked() {
                self.add(box_text(
                    &uid(),
                    x,
                    y,
                    40.,
                    20.,
                    tr(ja, "文字", "Text").into(),
                    12.,
                    "manual",
                ));
                ui.close();
                return;
            }
            if ui.button(tr(ja, "画像を追加", "Add image")).clicked() {
                platform::pick(platform::Pick::Artwork, io.tx.clone(), ctx.clone());
                ui.close();
                return;
            }
            if ui.button(tr(ja, "四角形を追加", "Add rectangle")).clicked() {
                self.add(json!({"id":uid(),"kind":"rect","x":x,"y":y,"w":30,"h":20,"rotation":0,"color":"#bdcfc6"}));
                ui.close();
                return;
            }
        }
        ui.separator();
        if ui
            .add_enabled(
                !self.undo.is_empty(),
                egui::Button::new(tr(ja, "元に戻す", "Undo")),
            )
            .clicked()
        {
            self.undo();
            ui.close();
            return;
        }
        if ui
            .add_enabled(
                !self.redo.is_empty(),
                egui::Button::new(tr(ja, "やり直す", "Redo")),
            )
            .clicked()
        {
            self.redo();
            ui.close();
            return;
        }
        ui.separator();
        ui.menu_button(tr(ja, "表示", "View"), |ui| {
            if ui.button(tr(ja, "全体を表示", "Fit card")).clicked() {
                self.fit_view(view);
                ui.close();
                return;
            }
            if ui.button(tr(ja, "100%表示", "100% zoom")).clicked() {
                self.normal_view(view);
                ui.close();
                return;
            }
            if ui
                .checkbox(&mut self.snap, tr(ja, "0.5mmに吸着", "Snap to 0.5 mm"))
                .changed()
            {
                ui.close();
                return;
            }
            if ui
                .button(if self.back {
                    tr(ja, "外側へ切り替え", "Switch to outside")
                } else {
                    tr(ja, "内側へ切り替え", "Switch to inside")
                })
                .clicked()
            {
                self.back = !self.back;
                self.selected = None;
                self.drag = None;
                ui.close();
            }
        });
    }
}
