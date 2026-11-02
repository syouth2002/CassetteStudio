// Exercise actual graph rendering and discard stale A/B responses after switching/closing.
const assert = require("node:assert/strict"),
  vm = require("node:vm"),
  fs = require("node:fs");
class Element {
  constructor(id) {
    this.id = id;
    this.style = {};
    this.children = [];
    this.attrs = {};
    this.listeners = {};
    this.value = "short_term";
    this.open = false;
  }
  setAttribute(k, v) {
    this.attrs[k] = v;
  }
  appendChild(el) {
    this.children.push(el);
  }
  replaceChildren(...els) {
    this.children = els;
  }
  addEventListener(k, fn) {
    (this.listeners[k] ??= []).push(fn);
  }
  showModal() {
    this.open = true;
  }
  close() {
    this.open = false;
    for (const fn of this.listeners.close || []) fn();
  }
}
const els = new Map(),
  get = (id) => {
    if (!els.has(id)) els.set(id, new Element(id));
    return els.get(id);
  };
const pending = [],
  errors = [];
const entry = {
    track_id: "track",
    offset: 0,
    duration: 5,
    fade_in: 0,
    fade_out: 1,
  },
  cassette = { id: "tape", kind: "cassette", name: "Demo" };
const context = {
  console,
  structuredClone,
  Number,
  Math,
  document: {
    createElementNS: (ns, name) => {
      const e = new Element(name);
      e.namespaceURI = ns;
      return e;
    },
  },
  $: get,
  busy: 0,
  selected: new Set([0]),
  entries: () => [entry],
  current: () => cassette,
  track: () => ({ id: "track", title: "Song" }),
  config: () => ({ target_lufs: -18 }),
  side: 0,
  saveChain: Promise.resolve(),
  drawSelection() {},
  toast: (e) => errors.push(e),
  time: (n) =>
    `${Math.floor(n / 60)}:${String(Math.round(n) % 60).padStart(2, "0")}`,
  csText: (s, ...args) => s.replace(/\{(\d+)\}/g, (_, n) => args[n]),
  api: (route, body) =>
    new Promise((resolve) => pending.push({ route, body, resolve })),
};
vm.createContext(context);
vm.runInContext(
  fs.readFileSync("web/js/audio/loudness-graph.js", "utf8"),
  context,
);
context.bindLoudnessGraphControls();
const series = {
  points: [
    { time: 1, momentary: -22, short_term: null },
    { time: 4, momentary: -18, short_term: -20 },
  ],
  integrated_lufs: -20,
  true_peak_db: -8,
};
const result = (name) => ({
  graph: {
    duration: 10,
    original: series,
    processed: series,
    markers: [{ time: 1, end: 5, title: "<b>Track</b>" }],
  },
  title: name,
  target_lufs: -18,
});
const tick = () => new Promise((resolve) => setImmediate(resolve));
(async () => {
  get("lufs-side-open").onclick();
  await tick();
  assert.equal(pending[0].body.side, 0);
  assert.equal(get("lufs-side-row").hidden, false);
  get("lufs-side").value = "1";
  get("lufs-side").onchange();
  await tick();
  assert.equal(pending[1].body.side, 1);
  pending[1].resolve(result("B"));
  await tick();
  assert.equal(get("lufs-title").textContent, "B");
  assert.equal(get("lufs-plot").children[0].attrs.viewBox, "0 0 880 440");
  assert.ok(get("lufs-tracks").textContent.includes("<b>Track</b>"));
  pending[0].resolve(result("A stale"));
  await tick();
  assert.equal(get("lufs-title").textContent, "B");
  get("lufs-plot").getBoundingClientRect = () => ({ width: 720, height: 480 });
  get("lufs-window").value = "momentary";
  get("lufs-window").onchange();
  assert.ok(
    get("lufs-plot").children[0].children.some(
      (e) => e.id === "path" && e.attrs.d.includes("L"),
    ),
  );
  assert.equal(get("lufs-plot").children[0].attrs.viewBox, "0 0 720 480");
  get("lufs-side").value = "0";
  get("lufs-side").onchange();
  await tick();
  get("lufs-dialog").close();
  pending[2].resolve(result("Closed stale"));
  await tick();
  assert.notEqual(get("lufs-title").textContent, "Closed stale");
  const trackPromise = get("lufs-open").onclick();
  await tick();
  assert.equal(get("lufs-side-row").hidden, true);
  assert.equal(pending[3].route, "/api/tracks/track/loudness-graph");
  pending[3].resolve({ duration: 5, original: series, processed: series });
  await trackPromise;
  assert.equal(get("lufs-tracks").textContent, "");
  assert.equal(errors.length, 0);
  console.log(
    "PASS: A/B switch, stale/closed response protection, SVG paths/markers, measurement-window switch, safe literal track legend, track-dialog reset",
  );
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
