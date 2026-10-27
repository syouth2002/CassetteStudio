"use strict";
// Cassette fitting and automatic arrangement.
const total = (entries, s = config()) =>
  (s.lead_in || 0) +
  entries.reduce((n, e) => n + e.duration, 0) +
  Math.max(0, entries.length - 1) * s.gap;
function fitTapeEntry(entry, sourceSeconds, occupied, s) {
  const available = Math.max(0, s.side_seconds - occupied),
    remaining = Math.max(0, sourceSeconds - entry.offset);
  if (remaining <= 0 || available < 0.1) return null;
  const cut = remaining > available + 0.000001;
  const length = cut ? Math.floor((available + 1e-9) * 10) / 10 : remaining;
  return {
    ...entry,
    duration: length,
    fade_in: 0,
    fade_out: cut ? Math.min(s.fade, Math.floor((length + 1e-9) * 2) / 2) : 0,
  };
}
async function makeTape(optimize) {
  const blanks = lib.collections.filter(
    (c) => c.kind === "cassette" && c.sides.every((es) => es.length === 0),
  );
  if (optimize && blanks.length) {
    const source = isTape() ? lib.tracks.map((t) => whole(t.id)) : entries();
    batchTracks = [
      ...new Set(
        (!isTape() && selected.size
          ? [...selected].sort((a, b) => a - b).map((i) => source[i])
          : source
        )
          .filter((e) => e.duration > 0)
          .map((e) => e.track_id),
      ),
    ];
    if (!batchTracks.length) {
      toast(csText("収録する音源をライブラリに追加してください"));
      return;
    }
    $("batch-source").textContent = csText(
      "選曲元：{0}（{1}曲）",
      isTape() ? csText("すべての曲") : current()?.name || csText("すべての曲"),
      batchTracks.length,
    );
    $("batch-targets").innerHTML = blanks
      .map((c) =>
        csText(
          '<label class="batch-target"><input type="checkbox" value="{0}" checked><span>{1}<small>片面 {2} × 2面 · 曲間 {3}秒</small></span></label>',
          esc(c.id),
          esc(c.name),
          time(c.settings.side_seconds),
          c.settings.gap,
        ),
      )
      .join("");
    $("batch-dialog").showModal();
    return;
  }
  const list = entries(),
    ids = (
      selected.size
        ? [...selected].sort((a, b) => a - b).map((i) => list[i])
        : list
    )
      .filter((e) => e.duration > 0)
      .map((e) => e.track_id);
  if (!ids.length) {
    toast(csText("音楽・動画ファイルを追加し、収録する曲を選択してください"));
    return;
  }
  const s = structuredClone(config()),
    name = await askText(
      csText("作成するカセットの名前"),
      numberedName(current()?.name || "My Mixtape"),
    );
  if (!name?.trim()) return;
  await task(csText("曲の長さを計算してA・B面へ配置しています…"), async () => {
    const sides = await api("/api/plan", {
      tracks: ids,
      settings: s,
      optimize,
    });
    const c = await api("/api/collections", {
      id: "",
      name: name.trim(),
      kind: "cassette",
      tracks: ids,
      sides,
      settings: s,
    });
    lib.collections.push(c);
    selectCollection(c.id);
    const used = new Set(sides.flat().map((e) => e.track_id));
    const omitted = ids.filter((id) => !used.has(id)).length;
    toast(
      omitted
        ? csText("カセットを作成しました。収まらなかった曲：{0}曲", omitted)
        : csText("カセットを作成しました"),
    );
  });
}
