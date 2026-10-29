"use strict";
// Library/collection data operations and media import orchestration.
function mergeTracks(tracks) {
  for (const t of tracks) {
    const i = lib.tracks.findIndex((v) => v.id === t.id);
    if (i >= 0) lib.tracks[i] = t;
  }
}
function mergeMeasurements(tracks) {
  for (const t of tracks) {
    const old = track(t.id);
    if (old) {
      old.loudness = t.loudness;
      old.dc = t.dc;
    }
  }
}
const isTape = () => current()?.kind === "cassette";
function entries() {
  const c = current();
  return !c
    ? lib.tracks.map((t) => whole(t.id))
    : c.kind === "playlist"
      ? c.tracks.map(whole)
      : c.sides[side];
}
function whole(id) {
  return {
    track_id: id,
    offset: 0,
    duration: duration(track(id)),
    fade_in: 0,
    fade_out: 0,
    continuation: false,
  };
}
function changed() {
  if (player?.collection === active) stop();
  renderCache = null;
  $("downloads").replaceChildren();
  if (current()) persist(current());
  draw();
}
function selectCollection(id) {
  active = id;
  side = 0;
  selected.clear();
  draw();
}
function numberedName(base) {
  const prefix = `${base.trim()} · `;
  let highest = 0n;
  for (const c of lib.collections) {
    const name = c.name.trim();
    if (!name.startsWith(prefix)) continue;
    const suffix = name.slice(prefix.length);
    if (/^\d+$/.test(suffix)) {
      const n = BigInt(suffix);
      if (n > highest) highest = n;
    }
  }
  return `${prefix}${highest + 1n}`;
}
async function newCollection(kind) {
  const name = await askText(
    kind === "playlist" ? csText("プレイリスト名") : csText("カセット名"),
    numberedName(
      kind === "playlist" ? csText("新しいプレイリスト") : "Untitled Mixtape",
    ),
  );
  if (!name?.trim()) return;
  const chosen = [...selected]
    .sort((a, b) => a - b)
    .map((i) => entries()[i].track_id);
  const c = await task(csText("作成しています…"), () =>
    api("/api/collections", {
      id: "",
      name: name.trim(),
      kind,
      tracks: kind === "playlist" ? chosen : [],
      sides: [[], []],
      settings: structuredClone(config()),
    }),
  );
  if (c) {
    lib.collections.push(c);
    selectCollection(c.id);
    if (kind === "cassette") {
      fillSettings();
      $("settings-dialog").returnValue = "";
      $("settings-dialog").showModal();
    }
  }
}
async function importFiles(files) {
  if (!files.length) return;
  const destination = active;
  await task(
    csText("{0}件の音源を取り込み、前後の無音を解析しています…", files.length),
    async () => {
      const form = new FormData();
      form.append("settings", JSON.stringify(config()));
      for (const f of files) form.append("files", f);
      const r = await fetch("/api/import", { method: "POST", body: form });
      const data = await r.json();
      if (!r.ok) throw Error(data.error || csText("取り込みに失敗しました"));
      const duplicates = data.duplicates || [];
      for (const t of [...data.tracks, ...duplicates.map((d) => d.track)])
        if (!lib.tracks.some((old) => old.id === t.id)) lib.tracks.push(t);
      const skipped = await attachImportedTracks(
        data.track_ids || [
          ...data.tracks.map((t) => t.id),
          ...duplicates.map((d) => d.track.id),
        ],
        destination,
      );
      draw();
      let message = duplicates.length
        ? csText(
            "{0}件を新規追加、登録済み{1}件は重複登録しませんでした",
            data.tracks.length,
            duplicates.length,
          )
        : csText("{0}件の音源を追加しました", data.tracks.length);
      if (skipped)
        message +=
          "\n" +
          csText(
            "残り時間がないため、{0}曲をカセットに追加できませんでした。音源はライブラリに残っています。",
            skipped,
          );
      if (data.errors.length) message += "\n" + data.errors.join("\n");
      toast(message);
    },
  );
}
// File/URL imports reuse existing sources without inserting the same track twice in the target.
async function attachImportedTracks(ids, destination = active) {
  const c = lib.collections.find((c) => c.id === destination);
  if (!c) return 0;
  const existing = new Set(
    c.kind === "playlist" ? c.tracks : c.sides.flat().map((e) => e.track_id),
  );
  const unique = [...new Set(ids)].filter((id) => !existing.has(id));
  return unique.length ? attachTracks(unique, destination) : 0;
}
async function attachTracks(ids, destination = active) {
  const c = lib.collections.find((c) => c.id === destination);
  if (!c) return;
  let skipped = 0;
  if (c.kind === "playlist") c.tracks.push(...ids);
  else {
    const list = c.sides[side],
      s = { ...defaults, ...c.settings };
    for (const id of ids) {
      const e = whole(id);
      if (e.duration <= 0) continue;
      const fitted = fitTapeEntry(
        e,
        e.duration,
        total(list, s) + (list.length ? s.gap : 0),
        s,
      );
      if (fitted) list.push(fitted);
      else skipped++;
    }
    if (skipped)
      toast(
        csText(
          "残り時間がないため、{0}曲をカセットに追加できませんでした。音源はライブラリに残っています。",
          skipped,
        ),
      );
  }
  renderCache = null;
  await persist(c);
  return skipped;
}
