"use strict";
// Cassette rendering requests and cassette-specific download links.
async function rendered() {
  const c = current();
  if (!c || c.kind !== "cassette")
    throw Error(csText("カセットを選択してください"));
  if (c.sides.some((es) => total(es) > config().side_seconds + 0.001))
    throw Error(csText("面の長さを超えています。曲を移動・削除してください"));
  const sides = structuredClone(c.sides),
    s = structuredClone(c.settings),
    id = c.id,
    name = c.name;
  const key = JSON.stringify([
    id,
    name,
    sides,
    s,
    lib.tracks.map((t) => [t.id, t.manual_gain_db || 0, t.dc || null]),
  ]);
  if (renderCache?.key === key) return renderCache;
  const data = await api("/api/render", { sides, settings: s });
  mergeMeasurements(data.tracks || []);
  draw();
  renderCache = {
    key: JSON.stringify([
      id,
      name,
      sides,
      s,
      lib.tracks.map((t) => [t.id, t.manual_gain_db || 0, t.dc || null]),
    ]),
    urls: data.urls,
    sides,
    settings: s,
    collection: id,
    name,
  };
  return renderCache;
}
function showDownloads(cache) {
  downloadCaches.set(cache.collection, cache);
  syncDownloads();
}
function syncDownloads() {
  const c = current(),
    cache = c?.kind === "cassette" ? downloadCaches.get(c.id) : null;
  const key = c
    ? JSON.stringify([
        c.id,
        c.name,
        c.sides,
        c.settings,
        lib.tracks.map((t) => [t.id, t.manual_gain_db || 0, t.dc || null]),
      ])
    : null;
  if (!cache || cache.key !== key) {
    $("downloads").replaceChildren();
    return;
  }
  $("downloads").innerHTML = cache.urls
    .map((u, i) =>
      csText(
        '<a href="{0}" download="{1}-{2}.wav">↓ SIDE {3} · WAVを保存</a>',
        esc(u),
        esc(cache.name),
        i ? "B" : "A",
        i ? "B" : "A",
      ),
    )
    .join("");
}
