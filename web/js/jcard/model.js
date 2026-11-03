"use strict";
// Portable, millimetre-based artwork model shared by preview and exports.
window.JCardModel = (() => {
  const clone = (v) => JSON.parse(JSON.stringify(v));
  const xml = (s) =>
    String(s ?? "").replace(
      /[&<>"']/g,
      (c) =>
        ({
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&apos;",
        })[c],
    );
  const color = (s) => (/^#[0-9a-f]{6}$/i.test(s) ? s : "#233b36");
  const families = { sans: "sans-serif", serif: "serif", mono: "monospace" };
  const width = (d) =>
    d.flap +
    d.spine +
    d.front +
    (d.extensions || []).reduce((a, b) => a + b, 0);
  const sheetSizes = { a4: [210, 297], letter: [215.9, 279.4] };
  function paperLayout(w, h, paper = "a4", margin = 5) {
    const size = sheetSizes[paper];
    if (!size || !Number.isFinite(margin) || margin < 0 || margin > 20)
      return null;
    for (const [pageWidth, pageHeight] of [size, [size[1], size[0]]])
      if (
        w + 2 * margin <= pageWidth + 1e-8 &&
        h + 2 * margin <= pageHeight + 1e-8
      )
        return { pageWidth, pageHeight, landscape: pageWidth > pageHeight };
    return null;
  }
  const layout = (d) =>
    paperLayout(width(d) + 10, d.height + 10, d.paper || "a4", d.margin ?? 5);
  const frontStart = (d, face = "front") =>
    face === "back" ? width(d) - d.flap - d.spine - d.front : d.flap + d.spine;
  function folds(d, face = "front") {
    let x = d.flap + d.spine + d.front;
    const result = [d.flap, d.flap + d.spine];
    for (const panel of d.extensions || []) {
      result.push(x);
      x += panel;
    }
    return [
      ...new Set(
        result
          .filter((n) => n > 0 && n < width(d))
          .map((n) => (face === "back" ? width(d) - n : n)),
      ),
    ].sort((a, b) => a - b);
  }
  const surface = (d, face = "front") => ({
    ...d,
    elements: face === "back" ? d.backElements || [] : d.elements,
    background: face === "back" ? d.backBackground || "#fffdf7" : d.background,
  });
  function upgrade(value) {
    validate(value);
    const d = clone(value);
    if (d.version === 1) {
      d.version = 2;
      d.extensions = [];
      d.paper = "a4";
      d.margin = 5;
      d.backElements = [];
      d.backBackground = "#ffffff";
      if (!layout(d)) d.margin = 0;
    }
    return validate(d);
  }
  function withExtensions(d, extensions) {
    const next = clone(d),
      delta =
        extensions.reduce((a, b) => a + b, 0) -
        (d.extensions || []).reduce((a, b) => a + b, 0);
    next.extensions = extensions;
    for (const e of next.backElements) e.x += delta;
    return validate(next);
  }
  const text = (id, x, y, w, h, value, size = 10, binding = "") => ({
    id,
    kind: "text",
    x,
    y,
    w,
    h,
    rotation: 0,
    text: value,
    size,
    lineHeight: 1.3,
    color: "#233b36",
    font: "sans",
    align: "left",
    bold: false,
    vertical: false,
    binding,
  });
  function values(c, tracks) {
    const lines = (side) =>
      c.sides[side]
        .map((e, i) => {
          const t = tracks.find((t) => t.id === e.track_id),
            sec = Math.max(0, Math.round(e.duration));
          return `${String(i + 1).padStart(2, "0")}  ${t?.title || "?"}${t?.artist ? " — " + t.artist : ""}  ${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, "0")}`;
        })
        .join("\n");
    return {
      title: c.name,
      spine: c.name,
      sideA: "SIDE A\n" + lines(0),
      sideB: "SIDE B\n" + lines(1),
    };
  }
  function create(c, tracks) {
    const v = values(c, tracks),
      d = {
        version: 2,
        extensions: [],
        paper: "a4",
        margin: 5,
        backBackground: "#ffffff",
        backElements: [],
        front: 65,
        spine: 12,
        flap: 25,
        height: 100,
        background: "#fffdf7",
        elements: [],
      };
    d.elements = [
      text("title", 41, 6, 57, 16, v.title, 17, "title"),
      text("spine", 28, 5, 7, 90, v.spine, 11, "spine"),
      text("side-a", 41, 27, 27, 68, v.sideA, 8, "sideA"),
      text("side-b", 72, 27, 27, 68, v.sideB, 8, "sideB"),
      text("notes", 3, 6, 19, 86, "CASSETTE\nSTUDIO", 10),
    ];
    d.elements[1].vertical = true;
    return d;
  }
  // Early editors cleared binding on every text edit. Recognize their original
  // track-list IDs, while an explicit "manual" choice always opts out.
  function binding(item) {
    if (item.binding === "manual") return "";
    if (["title", "spine", "sideA", "sideB"].includes(item.binding))
      return item.binding;
    return (
      {
        "side-a": "sideA",
        "side-b": "sideB",
        "back-a": "sideA",
        "back-b": "sideB",
      }[item.id] || ""
    );
  }
  function refresh(d, c, tracks) {
    const v = values(c, tracks);
    for (const item of [...d.elements, ...(d.backElements || [])]) {
      const source = item.kind === "text" ? binding(item) : "";
      if (source) {
        item.text = v[source];
        item.binding = source;
      }
    }
  }
  function panels(d, face) {
    let x = 0;
    const result = [];
    for (const [key, w] of [
      ["flap", d.flap],
      ["spine", d.spine],
      ["front", d.front],
      ...(d.extensions || []).map((w, i) => ["extra-" + i, w]),
    ]) {
      result.push({ key, x: face === "back" ? width(d) - x - w : x, w });
      x += w;
    }
    return result;
  }
  function pasteElement(clipboard, d, face, newId, offset = 0) {
    const item = clone(clipboard.item);
    item.id = newId;
    const source = clipboard.panels.find(
      (p) =>
        p.w > 0 &&
        clipboard.item.x + clipboard.item.w / 2 >= p.x &&
        clipboard.item.x + clipboard.item.w / 2 <= p.x + p.w,
    );
    const target = panels(d, face).find((p) => p.key === source?.key);
    if (source && target) item.x = target.x + (item.x - source.x);
    item.x = Math.max(
      0,
      Math.min(Math.max(0, width(d) - item.w), item.x + offset),
    );
    item.y = Math.max(
      0,
      Math.min(Math.max(0, d.height - item.h), item.y + offset),
    );
    return item;
  }
  function wrap(e) {
    const unit = (e.size * 25.4) / 72,
      limit = (e.vertical ? e.h : e.w) / unit,
      lines = [];
    for (const paragraph of e.text.split("\n")) {
      let line = "",
        n = 0;
      const advance = (char) => (/[\u0000-\u00ff]/.test(char) ? 0.58 : 1);
      const tokens =
        paragraph.match(
          /[A-Za-z0-9][A-Za-z0-9:.'’_-]*|[ \t]+|[^\x00-\x7f]|./gu,
        ) || [];
      for (const token of tokens) {
        const tokenWidth = [...token].reduce(
          (sum, char) => sum + advance(char),
          0,
        );
        if (n + tokenWidth > limit && line && tokenWidth <= limit) {
          lines.push(line.trimEnd());
          line = "";
          n = 0;
        }
        if (!line && /^\s+$/.test(token)) continue;
        for (const char of token) {
          if (n + advance(char) > limit && line) {
            lines.push(line.trimEnd());
            line = "";
            n = 0;
          }
          line += char;
          n += advance(char);
        }
      }
      lines.push(line);
    }
    return lines;
  }
  function overflow(d) {
    return d.elements
      .filter(
        (e) =>
          e.x < 0 ||
          e.y < 0 ||
          e.x + e.w > width(d) ||
          e.y + e.h > d.height ||
          (e.kind === "text" &&
            ((wrap(e).length * e.size * 25.4) / 72) * e.lineHeight >
              (e.vertical ? e.w : e.h)),
      )
      .map((e) => e.id);
  }
  function element(e, index) {
    const transform = `rotate(${Number(e.rotation) || 0} ${e.x + e.w / 2} ${e.y + e.h / 2})`;
    let content = "";
    if (e.kind === "rect")
      content = `<rect x="${e.x}" y="${e.y}" width="${e.w}" height="${e.h}" fill="${color(e.color)}"/>`;
    if (e.kind === "image")
      content = `<image x="${e.x}" y="${e.y}" width="${e.w}" height="${e.h}" href="${xml(e.src)}" preserveAspectRatio="xMidYMid meet"/>`;
    if (e.kind === "text") {
      const size = (e.size * 25.4) / 72,
        step = size * e.lineHeight;
      const lines = wrap(e)
        .map((line, i) =>
          e.vertical
            ? `<text x="${e.x + e.w - step * (i + 0.8)}" y="${e.y}" style="writing-mode:vertical-rl;text-orientation:mixed">${xml(line)}</text>`
            : `<text x="${e.align === "center" ? e.x + e.w / 2 : e.align === "right" ? e.x + e.w : e.x}" y="${e.y + size + i * step}" text-anchor="${e.align === "center" ? "middle" : e.align === "right" ? "end" : "start"}">${xml(line)}</text>`,
        )
        .join("");
      content = `<defs><clipPath id="clip-${index}"><rect x="${e.x}" y="${e.y}" width="${e.w}" height="${e.h}"/></clipPath></defs><g clip-path="url(#clip-${index})" font-family="${families[e.font] || "sans-serif"}" font-size="${size}" font-weight="${e.bold ? 700 : 400}" fill="${color(e.color)}">${lines}</g>`;
    }
    return `<g data-jcard-id="${xml(e.id)}" transform="${transform}">${content}</g>`;
  }
  function svg(d, guides = true, editing = false, face = "front") {
    const w = width(d),
      h = d.height,
      m = editing ? 0 : 5,
      view = surface(d, face);
    let body = `<defs><clipPath id="card-bounds"><rect width="${w}" height="${h}"/></clipPath></defs><g clip-path="url(#card-bounds)"><rect width="${w}" height="${h}" fill="${color(view.background)}"/>${view.elements.map(element).join("")}</g>`;
    if (guides) {
      body += `<g fill="none" stroke="#555" stroke-width="0.15" pointer-events="none"><rect class="cut-guide" width="${w}" height="${h}" stroke-dasharray="2 1"/>${folds(
        d,
        face,
      )
        .map(
          (x) =>
            `<path class="fold-guide" d="M${x} 0V${h}" stroke-dasharray="0.6 0.8"/>`,
        )
        .join("")}`;
      if (!editing)
        body +=
          folds(d, face)
            .map(
              (x) =>
                `<path class="fold-mark" d="M${x} -4V-1 M${x} ${h + 1}V${h + 4}"/>`,
            )
            .join("") +
          `<path class="cut-mark" d="M-5 0H-1 M0 -5V-1 M${w + 1} 0H${w + 5} M${w} -5V-1 M-5 ${h}H-1 M0 ${h + 1}V${h + 5} M${w + 1} ${h}H${w + 5} M${w} ${h + 1}V${h + 5}"/>`;
      body += "</g>";
    }
    return `<svg xmlns="http://www.w3.org/2000/svg" width="${w + m * 2}mm" height="${h + m * 2}mm" viewBox="${-m} ${-m} ${w + m * 2} ${h + m * 2}">${body}</svg>`;
  }
  function validate(d) {
    const num = (n, a, b) =>
      typeof n === "number" && Number.isFinite(n) && n >= a && n <= b;
    if (
      ![1, 2].includes(d?.version) ||
      !num(d.front, 30, 100) ||
      !num(d.spine, 5, 30) ||
      !num(d.flap, 0, 100) ||
      !num(d.height, 50, 200) ||
      !Array.isArray(d.elements) ||
      d.elements.length > 150
    )
      throw Error("Invalid J-CARD document");
    if (d.version === 2) {
      if (
        !Array.isArray(d.extensions) ||
        d.extensions.length > 30 ||
        !d.extensions.every((n) => num(n, 10, 100)) ||
        !Object.hasOwn(sheetSizes, d.paper) ||
        !num(d.margin, 0, 20)
      )
        throw Error("Invalid J-CARD print settings");
      if (!layout(d)) throw Error("選択した用紙に実寸で収まりません");
      validate({ ...d, version: 1, elements: d.backElements });
    }
    const ids = new Set();
    for (const e of d.elements) {
      if (
        !/^[a-zA-Z0-9_-]{1,100}$/.test(e.id) ||
        ids.has(e.id) ||
        !["text", "rect", "image"].includes(e.kind) ||
        !num(e.x, -300, 300) ||
        !num(e.y, -300, 300) ||
        !num(e.w, 1, 300) ||
        !num(e.h, 1, 300) ||
        !num(e.rotation, -360, 360)
      )
        throw Error("Invalid J-CARD element");
      ids.add(e.id);
      if (
        e.kind === "text" &&
        (typeof e.text !== "string" ||
          new TextEncoder().encode(e.text).length > 60000 ||
          !num(e.size, 4, 100) ||
          !num(e.lineHeight, 0.8, 3))
      )
        throw Error("Invalid J-CARD text");
      if (
        e.kind === "image" &&
        (typeof e.src !== "string" ||
          e.src.length > 8000030 ||
          !/^data:image\/(png|jpeg);base64,[A-Za-z0-9+/=]+$/.test(e.src))
      )
        throw Error("Use PNG / JPEG images");
    }
    return d;
  }
  // One physical sheet, outside/inside pages. Both pages use identical placement;
  // the inside artwork has its own readable coordinates and mirrored fold positions.
  function pdfPages(pages, mmWidth, mmHeight, options = {}) {
    const sheet = paperLayout(
      mmWidth,
      mmHeight,
      options.paper || "a4",
      options.margin ?? 0,
    );
    if (!sheet) throw Error("選択した用紙に実寸で収まりません");
    if (!pages.length || pages.length > 2)
      throw Error("Invalid J-CARD PDF pages");
    const encoder = new TextEncoder(),
      chunks = [],
      offsets = [0];
    let length = 0;
    const push = (b) => {
      b = typeof b === "string" ? encoder.encode(b) : b;
      chunks.push(b);
      length += b.length;
    };
    const obj = (id, body) => {
      offsets[id] = length;
      push(`${id} 0 obj\n`);
      push(body);
      push("\nendobj\n");
    };
    const pageW = (sheet.pageWidth * 72) / 25.4,
      pageH = (sheet.pageHeight * 72) / 25.4,
      w = (mmWidth * 72) / 25.4,
      h = (mmHeight * 72) / 25.4;
    // Preserve conventional rounded A4 dimensions in the emitted page boxes.
    const rw = Number(pageW.toFixed(4)),
      rh = Number(pageH.toFixed(4));
    push("%PDF-1.4\n");
    obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(
      2,
      `<< /Type /Pages /Kids [${pages.map((_, i) => `${3 + i * 3} 0 R`).join(" ")}] /Count ${pages.length} >>`,
    );
    pages.forEach((page, i) => {
      const id = 3 + i * 3;
      obj(
        id,
        `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${rw} ${rh}] /Resources << /XObject << /Im0 ${id + 1} 0 R >> >> /Contents ${id + 2} 0 R >>`,
      );
      offsets[id + 1] = length;
      push(
        `${id + 1} 0 obj\n<< /Type /XObject /Subtype /Image /Width ${page.pixelWidth} /Height ${page.pixelHeight} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${page.jpeg.length} >>\nstream\n`,
      );
      push(page.jpeg);
      push("\nendstream\nendobj\n");
      const stream = `q ${w.toFixed(5)} 0 0 ${h.toFixed(5)} ${((pageW - w) / 2).toFixed(5)} ${((pageH - h) / 2).toFixed(5)} cm /Im0 Do Q\n`;
      obj(
        id + 2,
        `<< /Length ${encoder.encode(stream).length} >>\nstream\n${stream}endstream`,
      );
    });
    const count = 3 + pages.length * 3,
      xref = length;
    push(`xref\n0 ${count}\n0000000000 65535 f \n`);
    for (let i = 1; i < count; i++)
      push(String(offsets[i]).padStart(10, "0") + " 00000 n \n");
    push(
      `trailer\n<< /Size ${count} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`,
    );
    const out = new Uint8Array(length);
    let n = 0;
    for (const b of chunks) {
      out.set(b, n);
      n += b.length;
    }
    return out;
  }
  function pdf(jpeg, pixelWidth, pixelHeight, w, h) {
    return pdfPages([{ jpeg, pixelWidth, pixelHeight }], w, h);
  }
  // Pointer deltas are in document coordinates. Resize in the box's rotated axes,
  // preserving the opposite edge/corner rather than moving its fixed anchor.
  function resizeBox(original, handle, dx, dy, step = 0.5) {
    const angle = (original.rotation * Math.PI) / 180;
    const cos = Math.cos(angle),
      sin = Math.sin(angle);
    const localX = dx * cos + dy * sin,
      localY = -dx * sin + dy * cos;
    const sx = handle.includes("e") ? 1 : handle.includes("w") ? -1 : 0;
    const sy = handle.includes("s") ? 1 : handle.includes("n") ? -1 : 0;
    const snap = (n) => Math.max(1, Math.min(300, Math.round(n / step) * step));
    const w =
      sx && Math.abs(localX) > 1e-9
        ? snap(original.w + sx * localX)
        : original.w;
    const h =
      sy && Math.abs(localY) > 1e-9
        ? snap(original.h + sy * localY)
        : original.h;
    const cx = (sx * (w - original.w)) / 2,
      cy = (sy * (h - original.h)) / 2;
    const x = original.x + (original.w - w) / 2 + cx * cos - cy * sin;
    const y = original.y + (original.h - h) / 2 + cx * sin + cy * cos;
    if (x < -300 || x > 300 || y < -300 || y > 300) return { ...original };
    return { ...original, x, y, w, h };
  }
  function rotateBox(original, start, point, step = 1) {
    const cx = original.x + original.w / 2,
      cy = original.y + original.h / 2;
    if (Math.hypot(point.x - cx, point.y - cy) < 0.1) return { ...original };
    const delta =
      ((Math.atan2(point.y - cy, point.x - cx) -
        Math.atan2(start.y - cy, start.x - cx)) *
        180) /
      Math.PI;
    const rotation = Math.round((original.rotation + delta) / step) * step;
    return {
      ...original,
      rotation: ((((rotation + 180) % 360) + 360) % 360) - 180,
    };
  }
  return {
    binding,
    panels,
    pasteElement,
    upgrade,
    surface,
    frontStart,
    folds,
    layout,
    paperLayout,
    withExtensions,
    pdfPages,
    resizeBox,
    rotateBox,
    clone,
    xml,
    width,
    text,
    create,
    refresh,
    wrap,
    overflow,
    svg,
    validate,
    pdf,
  };
})();
