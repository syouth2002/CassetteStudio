// Exercise actual portable artwork model; no user library is opened.
const fs = require("node:fs"),
  vm = require("node:vm"),
  assert = require("node:assert/strict");
const context = { window: {}, TextEncoder };
vm.createContext(context);
vm.runInContext(fs.readFileSync("web/js/jcard/model.js", "utf8"), context);
const M = context.window.JCardModel;
const tracks = [{ id: "t", title: "曲名 <script>&", artist: "Artist" }],
  entry = { track_id: "t", duration: 65 };
const c = {
  name: "日本語のカセット",
  sides: [[entry], [{ ...entry, duration: 30 }]],
};
const d = M.create(c, tracks);
M.validate(d);
assert.equal(M.width(d), 102);
assert.match(d.elements.find((e) => e.binding === "sideA").text, /1:05/);
d.elements[0].text = "manual";
d.elements[0].binding = "";
M.refresh(d, { ...c, name: "New name" }, tracks);
assert.equal(d.elements[0].text, "manual");
assert.equal(d.elements[1].text, "New name");
const svg = M.svg(d);
assert.match(svg, /width="112mm"/);
assert.match(svg, /&lt;script&gt;/);
assert.match(svg, /&amp;/);
assert.doesNotMatch(svg, /<script>/);
assert.match(svg, /clip-path/);
d.elements[0].text = "a".repeat(1000);
assert(M.overflow(d).includes("title"));
const bad = M.clone(d);
bad.elements[0].x = Infinity;
assert.throws(() => M.validate(bad));
const external = M.clone(d);
external.elements.push({
  id: "bad",
  kind: "image",
  x: 0,
  y: 0,
  w: 10,
  h: 10,
  rotation: 0,
  src: "https://example.com/image.png",
});
assert.throws(() => M.validate(external));
const pdf = M.pdf(Uint8Array.from([255, 216, 255, 217]), 1, 1, 112, 110),
  text = Buffer.from(pdf).toString("latin1");
assert.match(text, /%PDF-1.4/);
assert.match(text, /595.2756 841.8898/);
const xref = Number(text.match(/startxref\n(\d+)/)[1]);
assert.equal(text.slice(xref, xref + 4), "xref");
for (const [i, offset] of [
  ...text.slice(xref).matchAll(/(\d{10}) 00000 n/g),
].entries())
  assert(text.slice(+offset[1]).startsWith(`${i + 1} 0 obj`));
assert.match(
  Buffer.from(M.pdf(new Uint8Array(), 1, 1, 240, 210)).toString(),
  /841.8898 595.2756/,
);
console.log(
  "PASS: initial dimensions, cassette bindings, manual unlinking, Unicode XML escaping, clipping/overflow, validation, actual-size portrait/landscape PDF and byte offsets",
);

assert.deepEqual(
  Array.from(M.wrap(M.text("words", 0, 0, 27, 60, "Cassette Studio Demo", 8))),
  ["Cassette Studio", "Demo"],
);
// Rotated resize must preserve its opposite anchor in document coordinates.
const box = {
  ...M.text("resize", 10, 15, 40, 20, "日本語", 12),
  binding: "sideA",
};
const near = (a, b) => assert(Math.abs(a - b) < 1e-8, `${a} != ${b}`);
function anchor(b, h) {
  const sx = h.includes("e") ? 1 : h.includes("w") ? -1 : 0,
    sy = h.includes("s") ? 1 : h.includes("n") ? -1 : 0,
    a = (b.rotation * Math.PI) / 180,
    lx = (-sx * b.w) / 2,
    ly = (-sy * b.h) / 2;
  return {
    x: b.x + b.w / 2 + lx * Math.cos(a) - ly * Math.sin(a),
    y: b.y + b.h / 2 + lx * Math.sin(a) + ly * Math.cos(a),
  };
}
for (const angle of [0, 30, 90, -120, 179])
  for (const handle of ["nw", "n", "ne", "e", "se", "s", "sw", "w"]) {
    const original = { ...box, rotation: angle },
      next = M.resizeBox(original, handle, 7, -4, 0.5),
      a = anchor(original, handle),
      b = anchor(next, handle);
    near(a.x, b.x);
    near(a.y, b.y);
    assert.equal(next.text, box.text);
    assert.equal(next.size, box.size);
    assert.equal(next.binding, box.binding);
    if (!/[ew]/.test(handle)) assert.equal(next.w, box.w);
    if (!/[ns]/.test(handle)) assert.equal(next.h, box.h);
    assert.equal(next.w * 2, Math.round(next.w * 2));
    assert.equal(next.h * 2, Math.round(next.h * 2));
  }
const east = M.resizeBox(box, "e", 10, 0, 0.5);
near(east.x, 10);
near(east.w, 50);
near(east.y, 15);
const west = M.resizeBox(box, "w", 10, 0, 0.5);
near(west.x, 20);
near(west.w, 30);
assert.equal(M.resizeBox(box, "e", -1000, 0, 0.5).w, 1);
assert.equal(M.resizeBox(box, "se", 1000, 1000, 0.5).w, 300);
const center = { x: box.x + box.w / 2, y: box.y + box.h / 2 },
  start = { x: center.x, y: center.y - 20 };
const rotated = M.rotateBox(box, start, { x: center.x + 20, y: center.y });
near(rotated.rotation, 90);
near(rotated.x, box.x);
near(rotated.y, box.y);
assert.equal(rotated.text, box.text);
const angle = (22 * Math.PI) / 180;
assert.equal(
  M.rotateBox(
    box,
    start,
    { x: center.x + Math.sin(angle) * 20, y: center.y - Math.cos(angle) * 20 },
    15,
  ).rotation,
  15,
);
assert.deepEqual(
  JSON.parse(JSON.stringify(M.rotateBox(box, start, center))),
  JSON.parse(JSON.stringify(box)),
);
assert.doesNotMatch(
  M.svg({ ...d, elements: [rotated] }),
  /jc-resize|jc-rotate|data-jcard-handle/,
);
console.log(
  "PASS: all 8 resize anchors at 5 angles, minimum/maximum dimensions, text and binding retention, rotation center, 15-degree snapping, export without handles",
);
const base2 = M.create(c, tracks);
assert.equal(base2.version, 2);
assert(base2.backElements.length);
for (const paper of ["a4", "letter"]) {
  const tape = { ...M.clone(base2), paper };
  const plus2 = M.withExtensions(tape, [65, 65]);
  assert.equal(M.width(plus2), 232);
  assert.equal(M.layout(plus2).landscape, true);
  assert.throws(() => M.withExtensions(tape, [65, 65, 65]));
  assert.deepEqual(
    Array.from(M.folds(plus2, "back")),
    Array.from(M.folds(plus2))
      .map((x) => 232 - x)
      .sort((a, b) => a - b),
  );
  assert.equal(plus2.backElements[0].x, base2.backElements[0].x + 130);
}
assert(M.layout(M.withExtensions(base2, [40, 40, 40, 40])));
assert.throws(() =>
  M.withExtensions({ ...base2, paper: "letter" }, [40, 40, 40, 40]),
);
const legacy = {
  version: 1,
  front: 65,
  spine: 12,
  flap: 25,
  height: 100,
  background: "#ffffff",
  elements: [M.text("legacy", 5, 6, 30, 40, "Keep me")],
};
const migrated = M.upgrade(legacy);
assert.equal(migrated.version, 2);
assert.deepEqual(
  JSON.parse(JSON.stringify(migrated.elements)),
  JSON.parse(JSON.stringify(legacy.elements)),
);
assert.equal(migrated.backElements.length, 0);
const backSVG = M.svg(base2, true, false, "back");
assert.match(backSVG, /back-a/);
assert.doesNotMatch(backSVG, /data-jcard-id="title"/);
assert.match(backSVG, /cut-mark/);
assert.match(backSVG, /fold-mark/);
const two = Buffer.from(
  M.pdfPages(
    [
      { jpeg: new Uint8Array(), pixelWidth: 1, pixelHeight: 1 },
      { jpeg: new Uint8Array(), pixelWidth: 1, pixelHeight: 1 },
    ],
    242,
    110,
    { paper: "letter", margin: 5 },
  ),
).toString("latin1");
assert.match(two, /\/Count 2/);
assert.match(two, /MediaBox \[0 0 792 612\]/);
assert.throws(() =>
  M.pdfPages(
    [{ jpeg: new Uint8Array(), pixelWidth: 1, pixelHeight: 1 }],
    307,
    110,
    { paper: "a4", margin: 5 },
  ),
);
const x2 = Number(two.match(/startxref\n(\d+)/)[1]);
assert.equal(two.slice(x2, x2 + 4), "xref");
for (const [i, off] of [
  ...two.slice(x2).matchAll(/(\d{10}) 00000 n/g),
].entries())
  assert(two.slice(+off[1]).startsWith(`${i + 1} 0 obj`));
console.log(
  "PASS: legacy migration, independent faces, mirrored folds, A4/Letter real-size +2 limit, narrower extra panels, cutting/folding marks, two-page PDF offsets and paper sizes",
);
const legacyLists = M.create(c, tracks);
for (const item of [...legacyLists.elements, ...legacyLists.backElements])
  if (["sideA", "sideB"].includes(item.binding)) item.binding = "";
const updatedMix = { ...c, sides: [[{ track_id: "t", duration: 32 }], []] };
M.refresh(legacyLists, updatedMix, [{ ...tracks[0], title: "Updated" }]);
assert.match(
  legacyLists.elements.find((e) => e.id === "side-a").text,
  /Updated.*0:32/,
);
assert.equal(legacyLists.backElements[1].text, "SIDE B\n");
legacyLists.elements.find((e) => e.id === "side-a").binding = "manual";
legacyLists.elements.find((e) => e.id === "side-a").text = "Keep manually";
M.refresh(legacyLists, c, tracks);
assert.equal(
  legacyLists.elements.find((e) => e.id === "side-a").text,
  "Keep manually",
);
const sourceBox = {
  ...M.text("clip", 41, 12, 27, 45, "Linked text", 9, "sideA"),
  rotation: 37,
  bold: true,
  color: "#123456",
};
for (const extensions of [[], [65, 65]]) {
  const sheet = M.withExtensions(M.create(c, tracks), extensions),
    clip = { item: sourceBox, panels: M.panels(sheet, "front") };
  const pasted = M.pasteElement(clip, sheet, "back", "pasted");
  assert.equal(pasted.x, M.frontStart(sheet, "back") + 4);
  assert.equal(pasted.id, "pasted");
  for (const k of [
    "text",
    "w",
    "h",
    "rotation",
    "bold",
    "color",
    "size",
    "binding",
  ])
    assert.equal(pasted[k], sourceBox[k]);
  const repeated = M.pasteElement(clip, sheet, "back", "next", 2);
  assert.equal(repeated.x, pasted.x + 2);
  assert.equal(repeated.y, pasted.y + 2);
}
console.log(
  "PASS: legacy track-list relinking, emptied sides, explicit manual opt-out, clipboard styling/binding retention and cross-face panel placement",
);
