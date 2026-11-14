//! Transport event dispatch and application state updates.
use super::*;
impl Studio {
    pub(super) fn events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.io.rx.try_recv() {
            match event {
                Event::Response(tag, result) => {
                    self.io.pending = false;
                    if tag
                        .playback_generation()
                        .is_some_and(|g| g != self.player.generation)
                    {
                        continue;
                    }
                    let bytes = match result {
                        Ok(b) => b,
                        Err(mut e) => {
                            if let Some(Dialog::Output(output)) = &mut self.dialog {
                                let matching = match &tag {
                                    Tag::Render {
                                        id,
                                        snapshot,
                                        play: false,
                                        ..
                                    }
                                    | Tag::DeckAudio { id, snapshot, .. } => {
                                        output.matches(id, snapshot)
                                    }
                                    _ => false,
                                };
                                if matching {
                                    output.error = Some(e);
                                    continue;
                                }
                            }

                            if let Tag::Properties(ref id) = tag {
                                if let Some(Dialog::Properties(p)) = &mut self.dialog
                                    && p.track.id == *id
                                {
                                    p.error = Some(e);
                                }
                                continue;
                            }

                            if tag.playback_generation().is_some() {
                                self.stop();
                            }

                            if let Tag::JCardSaved(v) = tag
                                && let Some(c) = &mut self.card
                            {
                                c.failed(v);
                            }
                            if let Tag::Graph(generation) = tag {
                                self.graph.failed(generation, e.clone());
                            }
                            if matches!(&tag, Tag::Imported { .. } | Tag::ImportSaved { .. })
                                && let Some(progress) = self.feedback.import.take()
                            {
                                for error in progress.read_errors {
                                    e.push('\n');
                                    e.push_str(&error);
                                }
                            }
                            if matches!(&tag, Tag::ImportSaved { .. }) {
                                self.reload();
                                self.notify_error(format!("{}\n{e}",tr(self.ja,"音源はライブラリに保存済みです。リストへの反映でエラーが発生しました。","Audio files were saved to the library, but updating the list reported an error.")));
                            } else {
                                self.notify_error(e);
                            }
                            continue;
                        }
                    };
                    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                    match tag {
                        Tag::Properties(id) => {
                            if let Some(Dialog::Properties(p)) = &mut self.dialog
                                && p.track.id == id
                            {
                                if let Ok(track) =
                                    serde_json::from_value::<Track>(v["track"].clone())
                                {
                                    p.track = track;
                                }
                                p.media = Some(v);
                            }
                        }
                        Tag::Library => {
                            if let Ok(lib) = serde_json::from_value(v) {
                                self.library = lib;
                                let len = self.entries_len();
                                self.selected.retain(|i| *i < len);
                                let missing = self
                                    .library
                                    .tracks
                                    .iter()
                                    .filter(|t| {
                                        t.mono.is_none() && !self.mono_attempted.contains(&t.id)
                                    })
                                    .take(8)
                                    .map(|t| t.id.clone())
                                    .collect::<Vec<_>>();
                                if !missing.is_empty() {
                                    self.mono_attempted.extend(missing.clone());
                                    self.io.json(
                                        Tag::Mono,
                                        "POST",
                                        "/api/mono",
                                        json!({"tracks":missing}),
                                    );
                                }
                            }
                        }
                        Tag::Mono => {
                            for t in v["tracks"].as_array().into_iter().flatten() {
                                if let Some(old) = self
                                    .library
                                    .tracks
                                    .iter_mut()
                                    .find(|old| Some(old.id.as_str()) == t["id"].as_str())
                                {
                                    old.mono = t["mono"].as_bool();
                                }
                            }
                            if self
                                .library
                                .tracks
                                .iter()
                                .any(|t| t.mono.is_none() && !self.mono_attempted.contains(&t.id))
                            {
                                self.reload();
                            }
                        }
                        Tag::EntryAudio {
                            generation,
                            duration,
                        } => {
                            if generation == self.player.generation
                                && let Some(url) = v["urls"][0].as_str()
                            {
                                self.io.json(
                                    Tag::Audio {
                                        generation,
                                        offset: 0.,
                                        duration,
                                    },
                                    "GET",
                                    url,
                                    Value::Null,
                                );
                            }
                        }
                        Tag::Imported { target } => {
                            let mut incoming = v["tracks"].as_array().cloned().unwrap_or_default();
                            incoming.extend(
                                v["duplicates"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .map(|d| d["track"].clone()),
                            );
                            if v["id"].is_string() {
                                incoming.push(v.clone());
                            }
                            for t in incoming {
                                if let Ok(track) = serde_json::from_value::<Track>(t)
                                    && !self.library.tracks.iter().any(|old| old.id == track.id)
                                {
                                    self.library.tracks.push(track);
                                }
                            }
                            let mut ids = v["track_ids"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect::<Vec<_>>();
                            if ids.is_empty()
                                && let Some(id) = v["id"].as_str()
                            {
                                ids.push(id.into());
                            }
                            let mut seen = BTreeSet::new();
                            ids.retain(|id| seen.insert(id.clone()));
                            let mut message = if self
                                .feedback
                                .import
                                .as_ref()
                                .is_some_and(|p| p.url.is_some())
                            {
                                tr(
                                    self.ja,
                                    "URLの音源を取り込みました。",
                                    "Audio imported from the URL.",
                                )
                                .to_owned()
                            } else {
                                format!(
                                    "{} {}",
                                    ids.len(),
                                    tr(
                                        self.ja,
                                        "曲を取り込みました。登録済みのファイルは重複登録しません。",
                                        "tracks imported. Existing files are reused."
                                    )
                                )
                            };
                            let read_errors = self
                                .feedback
                                .import
                                .as_mut()
                                .map(|p| std::mem::take(&mut p.read_errors))
                                .unwrap_or_default();
                            let had_errors = !read_errors.is_empty()
                                || v["errors"]
                                    .as_array()
                                    .is_some_and(|errors| !errors.is_empty());
                            for error in read_errors {
                                message.push('\n');
                                message.push_str(&error);
                            }
                            for e in v["errors"].as_array().into_iter().flatten() {
                                if let Some(e) = e.as_str() {
                                    message.push('\n');
                                    message.push_str(e);
                                }
                            }
                            if had_errors && ids.is_empty() {
                                self.feedback.import = None;
                                self.notify_error(format!(
                                    "{}\n{}",
                                    tr(
                                        self.ja,
                                        "ファイルを追加できませんでした。",
                                        "The files could not be added."
                                    ),
                                    message
                                        .split_once('\n')
                                        .map_or(message.as_str(), |(_, details)| details)
                                ));
                                self.reload();
                                continue;
                            }
                            if let Some((id, side)) = target {
                                if let Some(c) =
                                    self.library.collections.iter().find(|c| c.id == id)
                                {
                                    let existing = if c.kind == "playlist" {
                                        c.tracks.clone()
                                    } else {
                                        c.sides
                                            .iter()
                                            .flatten()
                                            .map(|e| e.track_id.clone())
                                            .collect()
                                    };
                                    ids.retain(|id| !existing.contains(id));
                                }
                                self.append_to(&id, side, ids, Some(message), Some(had_errors));
                            } else {
                                self.feedback.import = None;
                                self.notify_kind(
                                    message,
                                    if had_errors {
                                        NoticeKind::Warning
                                    } else {
                                        NoticeKind::Success
                                    },
                                );
                            }
                            if !self.feedback.importing() {
                                self.reload();
                            }
                        }
                        Tag::ImportSaved { message, warning } => {
                            self.feedback.import = None;
                            self.notify_kind(
                                message,
                                if warning {
                                    NoticeKind::Warning
                                } else {
                                    NoticeKind::Success
                                },
                            );
                            self.reload();
                        }
                        Tag::Reordered => self.reload(),
                        Tag::Changed(notice) => {
                            if let Some(id) = v["id"].as_str().filter(|_| v["kind"].is_string())
                                && !self.library.collections.iter().any(|c| c.id == id)
                            {
                                self.select(Some(id.into()));
                                if let Ok(c) = serde_json::from_value::<Collection>(v.clone()) {
                                    self.library.collections.push(c);
                                }
                                if v["kind"] == "cassette"
                                    && v["sides"].as_array().is_some_and(|sides| {
                                        sides
                                            .iter()
                                            .all(|side| side.as_array().is_some_and(Vec::is_empty))
                                    })
                                    && let Ok(settings) =
                                        serde_json::from_value(v["settings"].clone())
                                {
                                    self.dialog = Some(Dialog::Settings {
                                        target: Some(id.into()),
                                        value: settings,
                                    });
                                }
                            }
                            let errors = v["errors"]
                                .as_array()
                                .map(|es| {
                                    es.iter()
                                        .map(|e| e.as_str().unwrap_or("?"))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                })
                                .unwrap_or_default();
                            if !errors.is_empty() {
                                self.notify_error(errors);
                            } else if let Some(message) = notice {
                                self.notify_warning(message);
                            } else {
                                self.notify(tr(self.ja, "完了しました", "Completed").into());
                            }
                            self.reload();
                        }
                        Tag::Message => {
                            self.notify_success(
                                tr(self.ja, "解析が完了しました", "Analysis completed").into(),
                            );
                            self.reload();
                        }
                        Tag::Plan { name, settings } => {
                            match serde_json::from_value::<[Vec<Entry>; 2]>(v) {
                                Ok(sides) => {
                                    let tracks = sides
                                        .iter()
                                        .flatten()
                                        .map(|e| e.track_id.clone())
                                        .collect::<BTreeSet<_>>()
                                        .into_iter()
                                        .collect();
                                    self.save_collection(Collection {
                                        id: String::new(),
                                        name,
                                        kind: "cassette".into(),
                                        tracks,
                                        sides,
                                        settings,
                                    });
                                }
                                Err(e) => self.notify_error(e.to_string()),
                            }
                        }
                        Tag::Render {
                            snapshot,
                            id,
                            generation,
                            play,
                            prepare,
                            side,
                        } => {
                            if let Ok(urls) =
                                serde_json::from_value::<[String; 2]>(v["urls"].clone())
                            {
                                self.exports
                                    .insert(id.clone(), (snapshot.clone(), urls.clone()));
                                if !play
                                    && let Some(Dialog::Output(output)) = &mut self.dialog
                                    && output.matches(&id, &snapshot)
                                {
                                    output.urls = Some(urls.clone());
                                }
                                if prepare {
                                    self.io.json(
                                        Tag::DeckAudio {
                                            id: id.clone(),
                                            side,
                                            snapshot,
                                        },
                                        "GET",
                                        &urls[side],
                                        Value::Null,
                                    );
                                }
                                if play && generation == self.player.generation {
                                    if let Some(Dialog::PlaybackPreparing {
                                        generation: g,
                                        loading,
                                        ..
                                    }) = &mut self.dialog
                                        && *g == generation
                                    {
                                        *loading = true;
                                    }
                                    self.io.json(
                                        Tag::Audio {
                                            generation,
                                            offset: 0.,
                                            duration: 0.,
                                        },
                                        "GET",
                                        &urls[side],
                                        Value::Null,
                                    );
                                }
                            } else if play && generation == self.player.generation {
                                self.stop();
                                self.notify_error(
                                    tr(
                                        self.ja,
                                        "再生データの生成結果を取得できませんでした。",
                                        "Could not read the prepared playback audio.",
                                    )
                                    .into(),
                                );
                            } else if let Some(Dialog::Output(output)) = &mut self.dialog
                                && output.matches(&id, &snapshot)
                            {
                                output.error = Some(
                                    tr(
                                        self.ja,
                                        "WAV生成結果を取得できませんでした。",
                                        "Could not read the generated WAV results.",
                                    )
                                    .into(),
                                );
                            }
                        }
                        Tag::DeckAudio { id, side, snapshot } => {
                            self.prepared_audio = Some((id, side, snapshot, bytes));
                        }
                        Tag::Preview {
                            generation,
                            offset,
                            duration,
                        } => {
                            if generation == self.player.generation
                                && let Some(url) = v["url"].as_str()
                            {
                                self.io.json(
                                    Tag::Audio {
                                        generation,
                                        offset,
                                        duration,
                                    },
                                    "GET",
                                    url,
                                    Value::Null,
                                );
                            }
                        }
                        Tag::Audio {
                            generation,
                            offset,
                            duration,
                        } => {
                            if generation == self.player.generation {
                                if matches!(&self.dialog,Some(Dialog::PlaybackPreparing{generation:g,..}) if *g==generation)
                                {
                                    self.dialog = None;
                                }
                                match self.player.play(bytes, offset, duration) {
                                    Ok(()) => {
                                        if let Some(at) = self.session.pending_seek.take() {
                                            self.player.seek(at);
                                        }
                                        self.preview.ready(generation);
                                    }
                                    Err(e) => {
                                        self.stop();
                                        self.notify_error(e);
                                    }
                                }
                            }
                        }
                        Tag::Download(name) => {
                            platform::save(name, bytes, self.io.tx.clone(), ctx.clone())
                        }
                        Tag::Graph(generation) => {
                            self.graph.complete(generation, v);
                            self.reload();
                        }
                        Tag::JCardRefresh(id) => {
                            if let Ok(lib) = serde_json::from_value(v) {
                                self.library = lib;
                                if let Some(card) = &mut self.card {
                                    card.refresh_latest(&id, &self.library);
                                }
                            }
                        }
                        Tag::JCard(id) => {
                            if let Some(c) = self.library.collections.iter().find(|c| c.id == id) {
                                self.card = Some(Card::new(
                                    c,
                                    &self.library.tracks,
                                    if v.is_null() { None } else { Some(v) },
                                ));
                                if let Some(card) = &mut self.card {
                                    card.clipboard = self.card_clipboard.take();
                                }
                            }
                        }
                        Tag::JCardSaved(version) => {
                            if let Some(c) = &mut self.card {
                                c.saved(version);
                            }
                        }
                        Tag::GainThenStereo { id, strength } => self.io.json(
                            Tag::Changed(None),
                            "PUT",
                            &format!("/api/tracks/{id}/pseudo-stereo"),
                            json!({"strength":strength}),
                        ),
                    }
                }
                Event::ImportReading(names) => {
                    self.feedback.begin_import(names, ctx.input(|i| i.time));
                }
                Event::Files { files, errors } => {
                    if files.is_empty() {
                        self.feedback.import = None;
                        if !errors.is_empty() {
                            self.notify_error(errors.join("\n"));
                        }
                    } else {
                        if !self.feedback.importing() {
                            self.feedback.begin_import(
                                files.iter().map(|(name, _)| name.clone()).collect(),
                                ctx.input(|i| i.time),
                            );
                        }
                        if let Some(progress) = &mut self.feedback.import {
                            progress.phase = ImportPhase::Analyzing;
                            progress.read_errors.extend(errors);
                        }
                        self.io.upload(
                            files,
                            &self.config(),
                            self.active.clone().map(|id| (id, self.side)),
                        );
                    }
                }
                Event::Artwork(b) => {
                    if let Some(c) = &mut self.card
                        && let Err(e) = c.add_image(b)
                    {
                        self.notify_error(e);
                    }
                }
                Event::Design(b) => {
                    if let Some(c) = &mut self.card
                        && let Err(e) = c.load(&b)
                    {
                        self.notify_error(e);
                    }
                }
                Event::Notice { message, error } => {
                    if let Some(Dialog::Output(output)) = &mut self.dialog {
                        if error {
                            output.error = Some(message);
                        } else {
                            output.notice = Some(message);
                        }
                    } else {
                        self.notify_kind(
                            message,
                            if error {
                                NoticeKind::Error
                            } else {
                                NoticeKind::Success
                            },
                        );
                    }
                }
            }
        }
    }
}
