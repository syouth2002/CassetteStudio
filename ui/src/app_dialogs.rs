//! Application settings and editing dialogs.
use super::*;
impl Studio {
    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context) {
        self.preview
            .sync(self.player.generation, self.player.finished());
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut open = true;
        let mut close = false;
        let ja = self.ja;
        let preview_dialog = matches!(
            dialog,
            Dialog::Gain { .. } | Dialog::Tone { .. } | Dialog::Properties(_)
        );
        let title = dialog.title(ja);
        let has_save = matches!(
            dialog,
            Dialog::Settings { .. }
                | Dialog::Rename(_)
                | Dialog::Gain { .. }
                | Dialog::Entry { .. }
        );
        let modal=egui::Modal::new(egui::Id::new("studio-dialog")).show(ctx,|ui|{
            ui.set_width(680.);
            let actions = appearance::dialog_header(ui, title, has_save.then_some(!self.io.busy()), ja);
            if actions.close {open=false;}
            let save_requested = actions.save;
            ui.separator();
            ui.add_enabled_ui(!self.io.busy() || preview_dialog || matches!(dialog,Dialog::Output(_) | Dialog::PlaybackPreparing{..}),|ui|{match &mut dialog{
                Dialog::PlaybackPreparing{name,loading,..}=>{
                    ui.strong(name.as_str());
                    ui.horizontal(|ui|{ui.spinner();ui.label(if *loading {tr(ja,"再生データを読み込んでいます…","Loading playback audio…")}else{tr(ja,"再生用のWAVを生成しています…","Generating WAV audio for playback…")});});
                    ui.label(tr(ja,"準備ができると、この画面を閉じて再生を開始します。閉じると取り消します。","Playback starts automatically when ready. Close this dialog to cancel."));
                },
                Dialog::Properties(p)=>{p.ui(ui,ja);},
                Dialog::Options=>{
                    if ui.button(tr(ja,"基準音","Reference tone")).clicked(){self.dialog=Some(Dialog::Tone{frequency:400.,db:-18.,seconds:30.});close=true;}
                    ui.small(tr(ja,"デッキの録音メーターを0dBに合わせるための音を出します。","Play a tone to align the deck recording meter to 0 dB."));
                    ui.separator();
                    ui.horizontal(|ui|{ui.label(tr(ja,"言語","Language"));egui::ComboBox::from_id_salt("language").selected_text(&self.language).show_ui(ui,|ui|{for (key,label)in[("auto","自動 / Automatic"),("ja","日本語"),("en","English")]{ui.selectable_value(&mut self.language,key.into(),label);}});});self.ja=self.language=="ja"||(self.language=="auto"&&platform::locale().starts_with("ja"));
                },
                Dialog::Tone{frequency,db,seconds}=>{
                    ui.add_enabled_ui(!self.io.busy(), |ui| {
                        ui.spacing_mut().slider_width = 300.;
                        ui.label(tr(ja,"デッキ録音レベルの基準音","Deck recording reference tone"));
                        ui.add(crate::appearance::slider(egui::Slider::new(frequency,100.0..=1000.).text("Hz")));
                        ui.add(crate::appearance::slider(egui::Slider::new(db,-36.0..=-6.).text("dBFS peak")));
                        ui.add(crate::appearance::slider(egui::Slider::new(seconds,1.0..=60.).text(tr(ja,"秒","Seconds"))));
                    });
                    ui.small(tr(ja,"出力経路の音量を固定し、デッキ側の録音レベルを0dBに合わせてください。","Keep the output-path volume fixed and set the deck recording meter to 0 dB."));
                    let active = self.preview.active();
                    if ui.add_enabled(active || !self.io.busy(), appearance::primary(if active {tr(ja,"停止","Stop")} else {tr(ja,"基準音を再生","Play tone")})).clicked() {
                        self.stop();
                        if !active {
                            self.player.title=tr(ja,"基準音","Reference tone").into();
                            match self.player.play(crate::playback::reference_tone(*frequency,*db,*seconds),0.,*seconds) {
                                Ok(()) => {self.preview.begin(self.player.generation);self.preview.ready(self.player.generation);}
                                Err(e) => self.notify_error(e),
                            }
                        }
                    }
                },
                Dialog::Url(text)=>{
                    ui.label(tr(ja,"音楽・動画のURL","Music / video URL"));
                    appearance::wide_text_input(ui,text,"url-input","https://…");
                    if ui.add_enabled(!text.trim().is_empty(), appearance::primary(tr(ja,"取り込む","Import"))).clicked(){
                        let url = text.trim().to_owned();
                        self.feedback.begin_url_import(url.clone(), ctx.input(|i|i.time));
                        self.io.json(Tag::Imported{target:self.active.clone().map(|id|(id,self.side))},"POST","/api/url",json!({"url":url,"settings":self.config()}));close=true;
                    }
                },
                Dialog::Settings{target,value:s}=>{settings::fields(ui,s,ja);if save_requested{match s.validate(){Ok(())=>{if let Some(id)=target{if let Some(mut c)=self.library.collections.iter().find(|c|&c.id==id).cloned(){c.settings=s.clone();self.save_collection(c);}}else{self.defaults=s.clone();}close=true;},Err(e)=>self.notify_error(e)}}},
                Dialog::Name{kind,name,optimize}=>{
                    ui.label(tr(ja,"名前","Name"));appearance::wide_text_input(ui,name,"create-name","");
                    if kind=="blank"{ui.label(tr(ja,"録音時間は作成後の「録音設定」で変更できます。","Change the recording duration in Recording settings after creation."));}
                    if ui.add(appearance::primary(tr(ja,"作成","Create"))).clicked()&&!name.trim().is_empty(){let name=numbered(name.trim(),self.library.collections.iter().map(|c|c.name.clone()));if kind=="cassette"{self.io.json(Tag::Plan{name,settings:self.config()},"POST","/api/plan",json!({"tracks":self.ids(),"settings":self.config(),"optimize":optimize}));}else{self.save_collection(Collection{id:String::new(),name,kind:if kind=="blank"{"cassette"}else{"playlist"}.into(),tracks:if kind=="blank"||self.selected.is_empty(){vec![]}else{self.ids()},sides:[vec![],vec![]],settings:self.config()});}close=true;}
                },
                Dialog::Rename(name)=>{appearance::wide_text_input(ui,name,"rename-input","");if save_requested&& let Some(mut c)=self.current().cloned(){c.name=name.trim().into();self.save_collection(c);close=true;}},
                Dialog::Add(ids)=>{egui::ScrollArea::vertical().max_height(450.).show(ui,|ui|{for t in &self.library.tracks{let mut checked=ids.contains(&t.id);if ui.checkbox(&mut checked,format!("{} — {}",t.title,t.artist)).changed(){if checked{ids.insert(t.id.clone());}else{ids.remove(&t.id);}}}});if ui.add(appearance::primary(tr(ja,"追加","Add"))).clicked(){self.append(self.library.tracks.iter().filter(|t|ids.contains(&t.id)).map(|t|t.id.clone()).collect());close=true;}},
                Dialog::Gain{track,gain,strength,..}=>{
                    ui.label(&track.title);
                    ui.add_enabled_ui(!self.io.busy(), |ui| {
                        ui.spacing_mut().slider_width = 320.;
                        ui.add(crate::appearance::slider(egui::Slider::new(gain,-24.0..=24.).text("dB")));
                        if track.mono==Some(true) {
                            egui::ComboBox::from_id_salt("pseudo-stereo").selected_text(["OFF","弱 / Weak","中 / Standard","強 / Strong"][*strength as usize]).show_ui(ui,|ui|{for(n,label)in["OFF","弱 / Weak","中 / Standard","強 / Strong"].iter().enumerate(){ui.selectable_value(strength,n as u8,*label);}});
                            ui.small(tr(ja,"OFF：元のモノラル音。弱：控えめな広がり。中：標準の広がり。強：左右の広がりを強め、音色の変化も感じやすくなります。","OFF: original mono. Weak: subtle width. Standard: moderate width. Strong: wider sound with a more noticeable tonal change."));
                        }
                    });
                    let active = self.preview.active();
                    if ui.add_enabled(active || !self.io.busy(), egui::Button::new(if active {tr(ja,"停止","Stop")} else {tr(ja,"試聴","Audition")})).clicked() {
                        self.stop();
                        if !active {
                            self.preview.begin(self.player.generation);
                            self.player.title=track.title.clone();
                            self.io.json(Tag::Preview{generation:self.player.generation,offset:0.,duration:track.audible()},"POST",&format!("/api/audition/{}",track.id),json!({"settings":self.config(),"gain_db":gain,"strength":strength}));
                        }
                    }
                    if self.preview.preparing() {ui.horizontal(|ui|{ui.spinner();ui.label(tr(ja,"試聴音声を準備中…","Preparing audition audio…"));});}
                    if save_requested{let tag=if track.mono==Some(true){Tag::GainThenStereo{id:track.id.clone(),strength:*strength}}else{Tag::Changed(None)};self.io.json(tag,"PUT",&format!("/api/tracks/{}/gain",track.id),json!({"gain_db":gain}));close=true;}
                },
                Dialog::Entry{index,value:e}=>{if let Some(t)=self.library.tracks.iter().find(|t|t.id==e.track_id).cloned(){ui.label(&t.title);ui.add(egui::DragValue::new(&mut e.offset).range(0.0..=t.audible()).speed(0.5).prefix(tr(ja,"開始（秒） ","Start (s) ")));ui.add(egui::DragValue::new(&mut e.duration).range(0.1..=t.audible()).speed(0.1).fixed_decimals(1).prefix(tr(ja,"再生する長さ（秒） ","Duration (s) ")));ui.add(egui::DragValue::new(&mut e.fade_in).range(0.0..=60.).speed(0.5).prefix("Fade in (s) "));ui.add(egui::DragValue::new(&mut e.fade_out).range(0.0..=60.).speed(0.5).prefix("Fade out (s) "));
                    if ui.button(tr(ja,"自動設定","Auto fit")).clicked()&& let Some(c)=self.current(){let others=c.sides[self.side].iter().enumerate().filter(|(i,_)|i!=index).map(|(_,e)|e.duration).sum::<f64>()+c.settings.gap*c.sides[self.side].len().saturating_sub(1)as f64+c.settings.lead_in;fit(e,c.settings.side_seconds-others,t.audible(),c.settings.fade);}
                    if save_requested&& let Some(mut c)=self.current().cloned(){c.sides[self.side][*index]=e.clone();self.save_collection(c);close=true;}
                }},
                Dialog::Batch(ids)=>{ui.label(tr(ja,"配置する空カセットを選択","Choose prepared blank tapes"));for c in self.library.collections.iter().filter(|c|c.kind=="cassette"&&c.sides.iter().all(Vec::is_empty)){let mut checked=ids.contains(&c.id);if ui.checkbox(&mut checked,format!("{} · {} × 2",c.name,time(c.settings.side_seconds))).changed(){if checked{ids.insert(c.id.clone());}else{ids.remove(&c.id);}}}if ui.add(appearance::primary(tr(ja,"自動配置","Arrange automatically"))).clicked(){self.io.json(Tag::Changed(None),"POST","/api/fill-cassettes",json!({"tracks":self.ids(),"targets":ids}));close=true;}},
                Dialog::Delete{collection}=>{ui.label(tr(ja,"削除しますか？ ライブラリの曲を削除すると、関連するカセットからも取り除きます。","Delete? Removing library tracks also removes them from related cassettes."));if ui.button(tr(ja,"削除する","Delete")).clicked(){if *collection{if let Some(id)=self.active.take(){self.io.json(Tag::Changed(None),"DELETE",&format!("/api/collections/{id}"),json!({}));}}else{self.remove_selection();}close=true;}},
                Dialog::Output(output)=>{
                    let ready=self.current().is_some_and(|c|c.id==output.id && render_key(c,&self.library.tracks)==output.snapshot)
                        && self.prepared_audio.as_ref().is_some_and(|(id,side,snapshot,_)|*id==output.id&&*side==output.side&&*snapshot==output.snapshot);
                    let now=ctx.input(|i|i.time);
                    if ready && output.countdown.is_some_and(|deadline|now>=deadline) {
                        output.start(self.prepared_audio.as_ref().unwrap().3.clone());
                    }
                    match output.ui(ui,&mut self.io,now,ready,ja) {
                        crate::output::OutputAction::Start=>{output.start(self.prepared_audio.as_ref().unwrap().3.clone());},
                        crate::output::OutputAction::Countdown=>{output.countdown=Some(now+5.);},
                        crate::output::OutputAction::Toggle=>{output.player.toggle();},
                        crate::output::OutputAction::Stop=>{output.stop();},
                        crate::output::OutputAction::Seek(at)=>output.player.seek(at),
                        crate::output::OutputAction::Volume(volume)=>output.player.set_volume(volume),
                        crate::output::OutputAction::Side(side)=>{
                            output.stop();output.side=side;output.error=None;
                            if let Some(urls)=&output.urls {
                                let cached=self.prepared_audio.as_ref().is_some_and(|(id,s,key,_)|*id==output.id && *s==side && *key==output.snapshot);
                                if !cached {self.io.json(Tag::DeckAudio{id:output.id.clone(),side,snapshot:output.snapshot.clone()},"GET",&urls[side],Value::Null);}
                            }
                        },
                        _=>{}
                    }
                },
            }
            });
            if self.io.busy() && !self.preview.preparing() && !matches!(dialog,Dialog::Output(_) | Dialog::PlaybackPreparing{..}){ui.horizontal(|ui|{ui.spinner();ui.label(tr(ja,"処理中…","Working…"));});}
        });
        if modal.should_close() {
            open = false;
        }
        if open && !close {
            self.dialog = Some(dialog);
        } else if matches!(
            dialog,
            Dialog::Tone { .. } | Dialog::Gain { .. } | Dialog::PlaybackPreparing { .. }
        ) {
            self.stop();
        }
    }
}
