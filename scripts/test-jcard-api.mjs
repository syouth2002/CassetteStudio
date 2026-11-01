// Real storage API tests run against an empty, isolated engine.
import { spawn } from "node:child_process";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
const root = process.cwd(),
  folder = path.join(root, "work", "jcard-" + Date.now());
await mkdir(folder, { recursive: true });
const server = spawn(
  path.join(
    root,
    "target/debug/cassette-studio" +
      (process.platform === "win32" ? ".exe" : ""),
  ),
  [],
  {
    cwd: root,
    windowsHide: true,
    env: { ...process.env, CASSETTE_DATA: folder, CASSETTE_PORT: "0" },
  },
);
let log = "",
  base;
server.stdout.on("data", (d) => (log += d));
server.stderr.on("data", (d) => (log += d));
async function api(url, body, method = "POST", status = 200) {
  const r = await fetch(
    base + url,
    body === undefined
      ? {}
      : {
          method,
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        },
  );
  const value = await r.json();
  assert.equal(r.status, status, JSON.stringify(value));
  return value;
}
try {
  for (let i = 0; i < 100; i++) {
    base = log.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
    if (base) break;
    await new Promise((r) => setTimeout(r, 50));
  }
  assert(base, log);
  const c = await api("/api/collections", {
      id: "",
      name: "J-CARD test",
      kind: "cassette",
      tracks: [],
      sides: [[], []],
      settings: { side_seconds: 1800 },
    }),
    url = "/api/jcards/" + c.id;
  assert.equal(await api(url), null);
  const d = {
    version: 1,
    front: 65,
    spine: 12,
    flap: 25,
    height: 100,
    background: "#ffffff",
    elements: [
      {
        id: "title",
        kind: "text",
        x: 40,
        y: 5,
        w: 50,
        h: 20,
        rotation: 0,
        size: 14,
        lineHeight: 1.3,
        text: "日本語 & English",
        binding: "title",
        font: "sans",
        color: "#234567",
      },
    ],
  };
  await api(url, d, "PUT");
  assert.deepEqual(await api(url), d);
  const next = structuredClone(d);
  next.elements[0].text = "Edited";
  next.elements[0].w = 47;
  next.elements[0].h = 28;
  next.elements[0].rotation = 37;
  await api(url, next, "PUT");
  assert.deepEqual(await api(url), next);
  const bad = structuredClone(d);
  bad.front = 1000;
  await api(url, bad, "PUT", 400);
  assert.deepEqual(await api(url), next);
  const image = structuredClone(d);
  image.elements = [
    {
      id: "image",
      kind: "image",
      x: 0,
      y: 0,
      w: 5,
      h: 5,
      rotation: 0,
      src: "data:image/svg+xml;base64,abc",
    },
  ];
  await api(url, image, "PUT", 400);
  await api("/api/jcards/missing", d, "PUT", 400);
  const dual = {
    ...next,
    version: 2,
    extensions: [65, 65],
    paper: "letter",
    margin: 5,
    backElements: [{ ...next.elements[0], text: "内側の保存" }],
    backBackground: "#ffffff",
  };
  await api(url, dual, "PUT");
  assert.deepEqual(await api(url), dual);
  const overflow = structuredClone(dual);
  overflow.extensions.push(65);
  await api(url, overflow, "PUT", 400);
  const invalidInside = structuredClone(dual);
  invalidInside.backElements[0].w = -1;
  await api(url, invalidInside, "PUT", 400);
  assert.deepEqual(await api(url), dual);
  const disk = JSON.parse(
    await readFile(path.join(folder, "jcards", c.id + ".json"), "utf8"),
  );
  assert.deepEqual(disk, dual);
  assert.equal((await api("/api/library")).collections[0].name, "J-CARD test");
  console.log(
    "PASS: cassette-specific J-CARD API, atomic replacement, disk persistence, Japanese text, invalid dimensions/images, missing cassette, unchanged collection",
  );
} finally {
  server.kill();
}
