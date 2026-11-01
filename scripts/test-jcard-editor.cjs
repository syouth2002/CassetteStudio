// Exercise real editor event bindings with deterministic storage/timers.
const fs = require("node:fs"),
  vm = require("node:vm"),
  assert = require("node:assert/strict");
const nodes = new Map(),
  timers = new Map();
let seq = 0,
  stored = null,
  fail = false,
  delay = false,
  active = 0,
  maxActive = 0;
const pending = [];
const copy = (v) => JSON.parse(JSON.stringify(v)),
  tick = () => new Promise((r) => setImmediate(r));
class Element {
  constructor(id) {
    this.id = id;
    this.value = "";
    this.checked = true;
    this.disabled = false;
    this.open = false;
    this.listeners = {};
    this.dataset = {};
    this.style = {};
    this.parentElement = { clientWidth: 800, clientHeight: 600 };
    this.textContent = "";
  }
  addEventListener(name, fn) {
    (this.listeners[name] ??= []).push(fn);
  }
  async emit(name, event = {}) {
    for (const fn of this.listeners[name] || [])
      await fn({ target: this, preventDefault() {}, ...event });
  }
  set innerHTML(value) {
    this.html = value;
    if (this.id === "jc-properties") {
      this.children = [];
      for (const m of value.matchAll(
        /<(input|textarea|select)\b([^>]*\bid="([^"]+)"[^>]*)>/g,
      )) {
        const n = new Element(m[3]);
        n.type = m[2].match(/type="([^"]+)"/)?.[1] || m[1];
        n.value = m[2].match(/value="([^"]*)"/)?.[1] || "";
        nodes.set(n.id, n);
        this.children.push(n);
      }
    }
  }
  get innerHTML() {
    return this.html || "";
  }
  querySelectorAll(selector) {
    if (this.id === "jc-canvas") return [];
    return this.children || [...nodes.values()].filter((n) => n !== this);
  }
  querySelector() {
    return { insertAdjacentHTML() {} };
  }
  insertAdjacentHTML(_, html) {
    if (this.id === "jc-properties") this.innerHTML = this.innerHTML + html;
  }
  showModal() {
    this.open = true;
  }
  close() {
    this.open = false;
  }
  blur() {}
  focus() {}
}
const html = fs.readFileSync("web/index.html", "utf8");
for (const m of html.matchAll(/\bid="([^"]+)"/g))
  nodes.set(m[1], new Element(m[1]));
nodes.get("jc-zoom").value = "100";
const cassette = {
  id: "cassette",
  kind: "cassette",
  name: "Editor test",
  sides: [[{ track_id: "track", duration: 60 }], []],
};
const context = {
  window: null,
  document: {
    getElementById: (id) => nodes.get(id),
    activeElement: { blur() {} },
  },
  TextEncoder,
  console,
  crypto: require("node:crypto").webcrypto,
  csText: (s, ...args) => s.replace(/\{(\d+)\}/g, (_, i) => args[i]),
  csTranslateMessage: (s) => s,
  toast() {},
  lib: { tracks: [{ id: "track", title: "Track" }], collections: [cassette] },
  isTape: () => true,
  busy: 0,
  current: () => cassette,
  saveChain: Promise.resolve(),
  setTimeout: (fn) => {
    timers.set(++seq, fn);
    return seq;
  },
  clearTimeout: (id) => timers.delete(id),
  addEventListener() {},
};
context.window = context;
context.$ = (id) => nodes.get(id);
let latestLibrary = copy(context.lib),
  libraryReads = 0,
  refreshFails = false;
context.api = async (url, body, method) => {
  if (url === "/api/library") {
    libraryReads++;
    if (refreshFails) throw Error("refresh error");
    return copy(latestLibrary);
  }
  if (method !== "PUT") return stored && copy(stored);
  active++;
  maxActive = Math.max(maxActive, active);
  if (delay) await new Promise((resolve) => pending.push(resolve));
  try {
    if (fail) throw Error("disk error");
    stored = copy(body);
    return copy(stored);
  } finally {
    active--;
  }
};
vm.createContext(context);
for (const p of ["web/js/jcard/model.js", "web/js/jcard/editor.js"])
  vm.runInContext(fs.readFileSync(p, "utf8"), context);
vm.runInContext("JCardEditor.bind()", context);
const node = (id) => nodes.get("jc-" + id),
  click = (id) => node(id).emit("click"),
  change = (id, value) => {
    node(id).value = value;
    return node(id).emit("change");
  };
const runTimers = () => {
  const jobs = [...timers.values()];
  timers.clear();
  for (const fn of jobs) fn();
};
(async () => {
  await nodes.get("jcard-btn").emit("click");
  assert(node("dialog").open);
  runTimers();
  await tick();
  assert(stored && stored.version === 2);
  assert.equal(node("save-state").textContent, "保存済み");
  const front = copy(stored.elements);
  await change("face", "back");
  await change("text", "Inside edited");
  await change("background", "#112233");
  runTimers();
  await tick();
  assert.deepEqual(stored.elements, front);
  assert.equal(stored.backElements[0].text, "Inside edited");
  assert.equal(stored.backElements[0].binding, "sideA");
  assert.equal(stored.backBackground, "#112233");
  await click("add-panel");
  await click("add-panel");
  assert(node("add-panel").disabled);
  await click("close");
  assert(!node("dialog").open);
  assert.equal(stored.extensions.length, 2);
  await nodes.get("jcard-btn").emit("click");
  await change("face", "back");
  await change("background", "#334455");
  fail = true;
  await click("close");
  assert(node("dialog").open);
  assert.equal(stored.backBackground, "#112233");
  assert.equal(node("save-state").textContent, "保存できませんでした");
  fail = false;
  // A close arriving during a save must wait and flush the newest edit, without parallel PUTs.
  delay = true;
  await change("background", "#445566");
  runTimers();
  await tick();
  assert.equal(pending.length, 1);
  await change("background", "#556677");
  runTimers();
  const closing = click("close");
  await tick();
  assert.equal(active, 1);
  pending.shift()();
  await tick();
  assert.equal(active, 1);
  assert.equal(pending.length, 1);
  pending.shift()();
  await closing;
  await tick();
  assert(!node("dialog").open);
  assert.equal(stored.backBackground, "#556677");
  assert.equal(maxActive, 1);
  delay = false;
  await nodes.get("jcard-btn").emit("click");
  // The backend list differs from the cached frontend list; refresh must fetch it.
  latestLibrary.tracks.push({
    id: "new-track",
    title: "New title",
    artist: "New artist",
  });
  latestLibrary.collections[0].sides[0] = [
    { track_id: "new-track", duration: 125 },
    { track_id: "track", duration: 31 },
  ];
  const oldGeometry = copy(stored.elements.find((e) => e.id === "side-a"));
  await click("refresh");
  await click("close");
  assert(libraryReads > 0);
  const updated = stored.elements.find((e) => e.id === "side-a");
  assert.match(updated.text, /01  New title/);
  assert.match(updated.text, /2:05/);
  assert.match(updated.text, /02  Track/);
  for (const key of ["x", "y", "w", "h", "rotation", "font", "size"])
    assert.equal(updated[key], oldGeometry[key]);
  assert.equal(stored.backElements[0].text, updated.text);
  await nodes.get("jcard-btn").emit("click");
  await change("face", "back");
  await change("binding", "manual");
  await change("text", "Private notes");
  await click("refresh");
  await click("close");
  assert.equal(stored.backElements[0].text, "Private notes");
  // Move a linked title using Cut -> switch face -> Paste; one Undo reverses paste.
  await nodes.get("jcard-btn").emit("click");
  const title = copy(stored.elements[0]);
  await click("cut");
  await change("face", "back");
  await click("paste");
  await click("close");
  assert(!stored.elements.some((e) => e.id === title.id));
  let pasted = stored.backElements.at(-1);
  assert.equal(pasted.text, title.text);
  assert.equal(pasted.binding, title.binding);
  assert.notEqual(pasted.id, title.id);
  for (const key of ["w", "h", "rotation", "font", "size", "color"])
    assert.equal(pasted[key], title[key]);
  // Keyboard object shortcuts work on canvas, while native textarea shortcuts pass through.
  await nodes.get("jcard-btn").emit("click");
  const frontCount = stored.elements.length,
    backCount = stored.backElements.length;
  const key = async (k, target = { tagName: "DIV" }) => {
    let prevented = false;
    await node("dialog").emit("keydown", {
      key: k,
      ctrlKey: true,
      target,
      preventDefault() {
        prevented = true;
      },
    });
    return prevented;
  };
  assert.equal(await key("x", { tagName: "TEXTAREA" }), false);
  assert.equal(await key("c"), true);
  await change("face", "back");
  assert.equal(await key("v", { tagName: "SELECT" }), true);
  await click("undo");
  await click("redo");
  await click("close");
  assert.equal(stored.elements.length, frontCount);
  assert.equal(stored.backElements.length, backCount + 1);
  await nodes.get("jcard-btn").emit("click");
  const beforeFailedRefresh = copy(stored);
  refreshFails = true;
  await click("refresh");
  refreshFails = false;
  await click("close");
  assert.deepEqual(stored, beforeFailedRefresh);
  console.log(
    "PASS: bound editor autosave, inside-only text/background edits, panel limit, close flush, save failure retains open document, serialized in-flight saves, fresh mix refresh, manual opt-out, cross-face Cut/Paste, keyboard copy/paste, undo/redo and refresh failure",
  );
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
