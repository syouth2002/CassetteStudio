import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile, readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
const root = process.cwd(),
  dir = path.join(root, "work", `lufs-side-graph-${Date.now()}`),
  data = path.join(dir, "data");
await mkdir(path.join(data, "sources"), { recursive: true });
const id = randomUUID(),
  silent = randomUUID(),
  short = randomUUID();
const ff = path.join(
  root,
  "tools",
  `ffmpeg${process.platform === "win32" ? ".exe" : ""}`,
);
for (const [key, expr, len] of [
  [id, "0.1*sin(2*PI*440*t)*if(lt(t,6),0.25,1)", 12],
  [silent, "0", 4],
  [short, "0.1*sin(2*PI*440*t)", 1],
]) {
  const out = spawnSync(
    ff,
    [
      "-v",
      "error",
      "-f",
      "lavfi",
      "-i",
      `aevalsrc=${expr.replaceAll(",", String.fromCharCode(92) + ",")}:s=44100:d=${len}`,
      "-ac",
      "2",
      "-f",
      "wav",
      path.join(data, "sources", key),
    ],
    { encoding: "utf8", windowsHide: true },
  );
  assert.equal(out.status, 0, out.stderr);
}
const tracks = [
  [id, 12],
  [silent, 4],
  [short, 1],
].map(([id, duration]) => ({
  id,
  title: "Synthetic LUFS test",
  artist: "Test",
  filename: "test.wav",
  source: "local",
  duration,
  trim_start: 0,
  trim_end: duration,
}));
await writeFile(
  path.join(data, "library.json"),
  JSON.stringify({ tracks, collections: [] }),
);
const sourceBefore = await readFile(path.join(data, "sources", id));
const server = spawn(
  path.join(
    root,
    "target",
    "debug",
    `cassette-studio${process.platform === "win32" ? ".exe" : ""}`,
  ),
  [],
  {
    cwd: root,
    windowsHide: true,
    env: { ...process.env, CASSETTE_DATA: data, CASSETTE_PORT: "0" },
  },
);
let logs = "";
server.stdout.on("data", (b) => (logs += b));
server.stderr.on("data", (b) => (logs += b));
let base;
const settings = { normalize: true, target_lufs: -18, remove_dc: true };
async function graph(key, extra = {}) {
  const r = await fetch(`${base}/api/tracks/${key}/loudness-graph`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ settings, ...extra }),
  });
  return { status: r.status, value: await r.json() };
}
async function request(route, body) {
  const r = await fetch(base + route, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return { status: r.status, value: await r.json() };
}
try {
  for (let i = 0; i < 100; i++) {
    base = logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
    if (base) break;
    await new Promise((r) => setTimeout(r, 50));
  }
  assert.ok(base, logs);
  const entry = (track_id, offset, duration, fade_out = 0) => ({
    track_id,
    offset,
    duration,
    fade_in: 0,
    fade_out,
    continuation: false,
  });
  let c = {
    id: randomUUID(),
    name: "Side graph test",
    kind: "cassette",
    tracks: [],
    settings: {
      side_seconds: 30,
      lead_in: 2,
      gap: 4,
      normalize: true,
      remove_dc: true,
      target_lufs: -18,
      wav_sample_rate: 48000,
    },
    sides: [[entry(id, 0, 6), entry(id, 6, 6, 2)], [entry(short, 0, 1)]],
  };
  assert.equal((await request("/api/collections", c)).status, 200);
  const route = `/api/collections/${c.id}/loudness-graph`;
  const a = await request(route, { side: 0 });
  assert.equal(a.status, 200, JSON.stringify(a));
  const g = a.value.graph;
  assert.equal(g.duration, 30);
  assert.equal(a.value.target_lufs, -18);
  assert.deepEqual(
    g.markers.map((m) => [m.time, m.end]),
    [
      [2, 8],
      [12, 18],
    ],
  );
  assert.ok(
    g.processed.points
      .filter((p) => p.time < 2)
      .every((p) => p.momentary === null),
  );
  assert.ok(
    g.processed.points
      .filter((p) => p.time > 9 && p.time < 12)
      .every((p) => p.momentary === null),
  );
  assert.ok(
    g.processed.points
      .filter((p) => p.time > 22)
      .every((p) => p.short_term === null),
  );
  const render = await request("/api/render", {
    sides: c.sides,
    settings: c.settings,
  });
  assert.equal(render.status, 200);
  const wav = path.join(dir, "side-a.wav");
  await writeFile(
    wav,
    Buffer.from(await (await fetch(base + render.value.urls[0])).arrayBuffer()),
  );
  const ref = spawnSync(
    ff,
    [
      "-hide_banner",
      "-nostats",
      "-i",
      wav,
      "-af",
      "ebur128=peak=true",
      "-f",
      "null",
      "-",
    ],
    { encoding: "utf8", windowsHide: true },
  );
  assert.equal(ref.status, 0, ref.stderr);
  const summary = ref.stderr.slice(ref.stderr.lastIndexOf("Summary:"));
  assert.equal(
    g.processed.integrated_lufs,
    +summary.match(/I:\s+([-0-9.]+)/)[1],
  );
  assert.equal(
    g.processed.true_peak_db,
    +summary.match(/Peak:\s+([-0-9.]+)/)[1],
  );
  const cache = path.join(data, "loudness-graphs"),
    files = await readdir(cache);
  assert.equal(
    files.filter((f) => f.endsWith(".wav") || f.endsWith(".pcm")).length,
    0,
  );
  const cached = files.find((f) => f.startsWith("side-")),
    mtime = (await stat(path.join(cache, cached))).mtimeMs;
  assert.deepEqual((await request(route, { side: 0 })).value, a.value);
  assert.equal((await stat(path.join(cache, cached))).mtimeMs, mtime);
  const b = await request(route, { side: 1 });
  assert.equal(b.status, 200);
  assert.equal(b.value.graph.markers.length, 1);
  assert.notEqual(
    b.value.graph.processed.integrated_lufs,
    g.processed.integrated_lufs,
  );
  c.sides[1] = [];
  assert.equal((await request("/api/collections", c)).status, 200);
  const empty = await request(route, { side: 1 });
  assert.equal(empty.status, 200);
  assert.equal(empty.value.graph.processed.integrated_lufs, null);
  assert.equal(empty.value.graph.markers.length, 0);
  c.settings.lead_in = 3;
  assert.equal((await request("/api/collections", c)).status, 200);
  const changed = await request(route, { side: 0 });
  assert.equal(changed.value.graph.markers[0].time, 3);
  c.sides[0].reverse();
  assert.equal((await request("/api/collections", c)).status, 200);
  assert.notDeepEqual((await request(route, { side: 0 })).value.graph, g);
  assert.equal((await request(route, { side: 2 })).status, 400);
  assert.equal(
    (await request("/api/collections/missing/loudness-graph", { side: 0 }))
      .status,
    400,
  );
  c.settings.side_seconds = 6;
  assert.equal((await request("/api/collections", c)).status, 200);
  assert.equal((await request(route, { side: 0 })).status, 400);
  assert.deepEqual(
    await readFile(path.join(data, "sources", id)),
    sourceBefore,
  );
  console.log(
    "PASS: full A/B sides, WAV agreement (48 kHz), lead-in/gaps/tail, fades, track boundaries, cache reuse/invalidation, empty side, overcapacity/invalid input, unchanged source, temporary-file cleanup",
  );
  console.log(dir);
} finally {
  server.kill();
}
