import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile, readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
const root = process.cwd(),
  dir = path.join(root, "work", `lufs-graph-${Date.now()}`),
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
try {
  for (let i = 0; i < 100; i++) {
    base = logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
    if (base) break;
    await new Promise((r) => setTimeout(r, 50));
  }
  assert.ok(base, logs);
  const first = await graph(id);
  assert.equal(first.status, 200, JSON.stringify(first));
  const g = first.value;
  assert.equal(g.duration, 12);
  assert.ok(g.original.points.length > 20);
  const quiet = g.original.points.find(
      (p) => p.time > 4 && p.time < 5,
    ).short_term,
    loud = g.original.points.find((p) => p.time > 10).short_term;
  assert.ok(loud - quiet > 11 && loud - quiet < 13, `${quiet} / ${loud}`);
  assert.ok(
    Math.abs(g.processed.integrated_lufs + 18) < 1,
    JSON.stringify(g.processed),
  );
  assert.ok(g.original.true_peak_db < 0);
  assert.ok(
    g.original.points
      .filter((p) => p.time < 3)
      .every((p) => p.short_term === null),
  );
  const cache = path.join(data, "loudness-graphs"),
    files = await readdir(cache),
    mtime = (await stat(path.join(cache, files[0]))).mtimeMs;
  assert.deepEqual((await graph(id)).value, g);
  assert.equal((await stat(path.join(cache, files[0]))).mtimeMs, mtime);
  const entry = {
    track_id: id,
    offset: 6,
    duration: 6,
    fade_in: 0,
    fade_out: 3,
    continuation: false,
  };
  const faded = await graph(id, { entry });
  assert.equal(faded.status, 200);
  assert.equal(faded.value.duration, 6);
  assert.ok(
    faded.value.processed.points.at(-1).momentary <
      faded.value.processed.points[2].momentary - 10,
  );
  const renderedResponse = await fetch(`${base}/api/render`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      settings: { ...settings, side_seconds: 6, gap: 0 },
      sides: [[entry], []],
    }),
  });
  assert.equal(renderedResponse.status, 200);
  const rendered = await renderedResponse.json(),
    wav = path.join(dir, "rendered.wav");
  await writeFile(
    wav,
    Buffer.from(await (await fetch(base + rendered.urls[0])).arrayBuffer()),
  );
  const reference = spawnSync(
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
  assert.equal(reference.status, 0, reference.stderr);
  const summary = reference.stderr.slice(
    reference.stderr.lastIndexOf("Summary:"),
  );
  assert.equal(
    faded.value.processed.integrated_lufs,
    +summary.match(/I:\s+([-0-9.]+)/)[1],
  );
  assert.equal(
    faded.value.processed.true_peak_db,
    +summary.match(/Peak:\s+([-0-9.]+)/)[1],
  );
  const changed = await graph(id, {
    settings: { ...settings, target_lufs: -24 },
  });
  assert.ok(
    Math.abs(
      changed.value.processed.integrated_lufs - g.processed.integrated_lufs + 6,
    ) < 0.2,
  );
  assert.equal(
    (await graph(id, { entry: { ...entry, duration: 30 } })).status,
    400,
  );
  assert.equal(
    (await graph(id, { entry: { ...entry, track_id: silent } })).status,
    400,
  );
  const silence = (await graph(silent)).value;
  assert.equal(silence.original.integrated_lufs, null);
  assert.ok(silence.original.points.every((p) => p.short_term === null));
  const brief = (await graph(short)).value;
  assert.ok(brief.original.points.every((p) => p.short_term === null));
  assert.ok(brief.original.points.some((p) => p.momentary !== null));
  assert.deepEqual(
    await readFile(path.join(data, "sources", id)),
    sourceBefore,
  );
  console.log(
    "PASS: LUFS windows, 12 dB dynamics, normalization, fades/range, true peak, cache reuse/invalidation, silence, short audio, validation, source unchanged",
  );
  console.log(dir);
} finally {
  server.kill();
}
