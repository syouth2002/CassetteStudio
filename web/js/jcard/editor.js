"use strict";
// Editor controls remain separate from audio and collection persistence.
const JCardEditor = (() => {
  const M = JCardModel;
  let doc = null,
    cassette = null,
    chosen = null,
    undo = [],
    redo = [],
    saved = "",
    drag = null,
    working = false;
  let face = "front",
    saveTimer = null,
    saving = null,
    saveError = "",
    closing = false;
  let elementClipboard = null;
  let customSpine = false;
  const $j = (id) => document.getElementById("jc-" + id),
    t = (ja) => csText(ja);
  const elements = () => (face === "back" ? doc.backElements : doc.elements);
  const setElements = (items) => {
    doc[face === "back" ? "backElements" : "elements"] = items;
  };
  const backgroundKey = () =>
    face === "back" ? "backBackground" : "background";
  const e = () => doc && elements().find((e) => e.id === chosen);
  const dirty = () => doc && JSON.stringify(doc) !== saved;
  function download(data, type, extension, suffix = "") {
    const url = URL.createObjectURL(new Blob([data], { type })),
      a = document.createElement("a");
    a.href = url;
    a.download =
      (cassette.name.replace(/[\\/:*?"<>|]/g, "_") || "J-CARD") +
      suffix +
      "." +
      extension;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 30000);
  }
  function mutate(fn) {
    const before = JSON.stringify(doc);
    fn();
    if (JSON.stringify(doc) !== before) {
      undo.push(before);
      if (undo.length > 40) undo.shift();
      redo = [];
    }
    render();
  }
  const control = (id, label, type, value, attrs = "") =>
    `<label>${t(label)}<input id="jc-${id}" type="${type}" value="${M.xml(value)}" ${attrs}></label>`;
  function preview() {
    $j("canvas").innerHTML = M.svg(doc, true, true, face);
    $j("canvas")
      .querySelectorAll("[data-jcard-id]")
      .forEach((g) => {
        const item = elements().find((e) => e.id === g.dataset.jcardId);
        g.insertAdjacentHTML(
          "beforeend",
          `<rect class="jc-hit ${item.id === chosen ? "jc-selected" : ""}" x="${item.x}" y="${item.y}" width="${item.w}" height="${item.h}" fill="transparent"/>`,
        );
      });
    const item = e();
    if (item) {
      const scale = (4 * Number($j("zoom").value)) / 100;
      const size = 9 / scale,
        offset = 26 / scale;
      const cx = item.x + item.w / 2,
        cy = item.y + item.h / 2;
      const points = [
        ["nw", item.x, item.y],
        ["n", cx, item.y],
        ["ne", item.x + item.w, item.y],
        ["e", item.x + item.w, cy],
        ["se", item.x + item.w, item.y + item.h],
        ["s", cx, item.y + item.h],
        ["sw", item.x, item.y + item.h],
        ["w", item.x, cy],
      ];
      const handles = points
        .map(
          ([name, x, y]) =>
            `<rect data-jcard-handle="${name}" class="jc-resize jc-resize-${name}" x="${x - size / 2}" y="${y - size / 2}" width="${size}" height="${size}" aria-label="${M.xml(t("サイズ変更"))}"/>`,
        )
        .join("");
      $j("canvas")
        .querySelector("svg")
        .insertAdjacentHTML(
          "beforeend",
          `<g class="jc-transform-controls" data-jcard-id="${M.xml(item.id)}" transform="rotate(${item.rotation} ${cx} ${cy})"><rect class="jc-transform-outline" x="${item.x}" y="${item.y}" width="${item.w}" height="${item.h}"/><path class="jc-rotation-line" d="M${cx} ${item.y}V${item.y - offset}"/>${handles}<circle data-jcard-handle="rotate" class="jc-rotate" cx="${cx}" cy="${item.y - offset}" r="${size * 0.65}" aria-label="${M.xml(t("回転"))}"/></g>`,
        );
    }
    $j("canvas").style.width =
      `${(M.width(doc) * 4 * Number($j("zoom").value)) / 100}px`;
    $j("save-state").textContent = saving
      ? t("保存中…")
      : saveError
        ? t("保存できませんでした")
        : dirty()
          ? t("保存待ち")
          : t("保存済み");
    const sheet = M.layout(doc);
    $j("paper-status").textContent = csText(
      "展開寸法 {0} × {1} mm · {2}",
      M.width(doc),
      doc.height,
      sheet?.landscape ? t("横向き") : t("縦向き"),
    );
    $j("add-panel").disabled = working || closing || !canAddPanel();
    $j("undo").disabled = working || closing || !undo.length;
    $j("redo").disabled = working || closing || !redo.length;
    $j("cut").disabled = $j("copy").disabled = working || closing || !e();
    $j("paste").disabled =
      working || closing || !elementClipboard || elements().length >= 150;
  }
  function properties() {
    const item = e(),
      p = $j("properties");
    if (!item) {
      p.innerHTML = `<p>${t("要素を選択してください")}</p>`;
      return;
    }
    p.innerHTML = `<div class="jc-fields">${control("x", "横位置（mm）", "number", item.x, 'step="0.5" min="-300" max="300"')}${control("y", "縦位置（mm）", "number", item.y, 'step="0.5" min="-300" max="300"')}${control("w", "幅（mm）", "number", item.w, 'step="0.5" min="1" max="300"')}${control("h", "高さ（mm）", "number", item.h, 'step="0.5" min="1" max="300"')}${control("rotation", "回転（度）", "number", item.rotation, 'step="1" min="-360" max="360"')}</div>`;
    p.insertAdjacentHTML(
      "beforeend",
      `<p class="hint" id="jc-transform-help">${M.xml(t("枠の四隅・辺をドラッグしてサイズ変更、上の丸で回転。Shiftで15度刻み、Escで取消。"))}</p>`,
    );
    if (item.kind !== "image")
      p.insertAdjacentHTML(
        "beforeend",
        control("color", "色", "color", item.color),
      );
    if (item.kind === "text") {
      p.insertAdjacentHTML(
        "beforeend",
        `<label>${t("連動する内容")}<select id="jc-binding"><option value="manual">${t("自由入力（更新しない）")}</option><option value="title">${t("カセット名")}</option><option value="spine">${t("背表紙の名前")}</option><option value="sideA">${t("SIDE Aの曲一覧")}</option><option value="sideB">${t("SIDE Bの曲一覧")}</option></select></label>`,
      );
      $j("binding").value = M.binding(item) || "manual";
    }
    if (item.kind === "text")
      p.insertAdjacentHTML(
        "beforeend",
        `<label>${t("文字")}<textarea id="jc-text" rows="7">${M.xml(item.text)}</textarea></label><p class="hint">${M.binding(item) ? t("「曲一覧を更新」で、連動先の内容に置き換えます。書式と配置は保持します。") : t("自由に編集する文字です。")}</p><div class="jc-fields">${control("size", "文字サイズ（pt）", "number", item.size, 'min="4" max="100" step="0.5"')}${control("lineHeight", "行間", "number", item.lineHeight, 'min="0.8" max="3" step="0.1"')}</div><label>${t("書体")}<select id="jc-font"><option value="sans">${t("ゴシック体")}</option><option value="serif">${t("明朝体")}</option><option value="mono">${t("等幅")}</option></select></label><label>${t("文字揃え")}<select id="jc-align"><option value="left">${t("左揃え")}</option><option value="center">${t("中央揃え")}</option><option value="right">${t("右揃え")}</option></select></label><label><input type="checkbox" id="jc-bold" ${item.bold ? "checked" : ""}> ${t("太字")}</label><label><input type="checkbox" id="jc-vertical" ${item.vertical ? "checked" : ""}> ${t("縦書き")}</label>`,
      );
    if (item.kind === "text") {
      $j("font").value = item.font;
      $j("align").value = item.align;
    }
    p.querySelectorAll("input,textarea,select").forEach((input) =>
      input.addEventListener("change", () => {
        const key = input.id.slice(3),
          before = M.clone(item);
        const value =
          input.type === "checkbox"
            ? input.checked
            : input.type === "number"
              ? Number(input.value)
              : input.value;
        const test = M.clone(doc),
          target = (face === "back" ? test.backElements : test.elements).find(
            (e) => e.id === chosen,
          );
        target[key] = value;

        try {
          M.validate(test);
          mutate(() => Object.assign(item, target));
        } catch (error) {
          Object.assign(item, before);
          toast(error.message);
          properties();
        }
      }),
    );
  }
  function render() {
    if (!doc) return;
    $j("layers").innerHTML = elements()
      .map(
        (item, i) =>
          `<button type="button" data-id="${item.id}" class="${item.id === chosen ? "active" : ""}">${i + 1}. ${M.xml(item.kind === "text" ? item.text.slice(0, 35) || t("文字") : item.kind === "image" ? t("画像") : t("四角形"))}</button>`,
      )
      .join("");
    for (const key of ["front", "spine", "flap", "height", "background"])
      $j(key).value = key === "background" ? doc[backgroundKey()] : doc[key];
    if (doc.spine !== 12) customSpine = true;
    $j("spine-mode").value = customSpine ? "custom" : "standard";
    $j("spine").disabled = !customSpine;
    $j("face").value = face;
    $j("paper").value = doc.paper;
    $j("margin").value = doc.margin;
    $j("panels").innerHTML = doc.extensions
      .map(
        (w, i) =>
          `<div class="jc-panel-width"><label>${csText("追加パネル {0}（mm）", i + 1)}<input type="number" min="10" max="100" step="0.5" value="${w}" data-panel="${i}"></label><button type="button" data-remove-panel="${i}" aria-label="${M.xml(t("パネルを外す"))}">−</button></div>`,
      )
      .join("");
    preview();
    properties();
    scheduleSave();
  }
  async function open() {
    if (!isTape() || busy) return;
    const target = M.clone(current());
    try {
      await saveChain;
      const stored = await api("/api/jcards/" + encodeURIComponent(target.id));
      doc = stored ? M.upgrade(stored) : M.create(target, lib.tracks);
      customSpine = doc.spine !== 12;
      face = "front";
      saveError = "";
      closing = false;
      clearTimeout(saveTimer);
      cassette = target;
      saved = stored ? JSON.stringify(stored) : "";
      chosen = doc.elements[0]?.id;
      undo = [];
      redo = [];
      $j("heading").textContent = "J-CARD · " + target.name;
      $j("dialog").showModal();
      const stage = $j("canvas").parentElement;
      $j("zoom").value = Math.max(
        25,
        Math.min(
          200,
          Math.floor(
            Math.min(
              (stage.clientWidth - 48) / (M.width(doc) * 4),
              (stage.clientHeight - 48) / (doc.height * 4),
            ) * 100,
          ),
        ),
      );
      render();
    } catch (error) {
      toast(error.message);
    }
  }
  function canAddPanel() {
    return (
      doc.extensions.length < 30 &&
      !!M.layout({ ...doc, extensions: [...doc.extensions, doc.front] })
    );
  }
  function scheduleSave() {
    clearTimeout(saveTimer);
    if (doc && dirty() && !drag && !working && !closing)
      saveTimer = setTimeout(() => {
        saveNow().catch((error) =>
          toast(csText("自動保存に失敗しました: {0}", error.message)),
        );
      }, 750);
  }
  async function saveNow() {
    while (saving) await saving;
    if (!doc || !dirty()) return;
    const copy = M.clone(doc),
      target = cassette.id;
    saveError = "";
    saving = (async () => {
      M.validate(copy);
      await api("/api/jcards/" + encodeURIComponent(target), copy, "PUT");
      saved = JSON.stringify(copy);
    })();
    preview();
    try {
      await saving;
    } catch (error) {
      saveError = error.message;
      throw error;
    } finally {
      saving = null;
      if (doc) preview();
    }
    if (dirty() && !closing) scheduleSave();
  }
  async function close() {
    if (working || closing || drag) return;
    document.activeElement?.blur();
    clearTimeout(saveTimer);
    closing = true;
    $j("dialog")
      .querySelectorAll("button,input,select,textarea")
      .forEach((b) => (b.disabled = true));
    try {
      await saveNow();
      $j("dialog").close();
      doc = null;
    } catch (error) {
      toast(csText("自動保存に失敗しました: {0}", error.message));
    } finally {
      closing = false;
      $j("dialog")
        .querySelectorAll("button,input,select,textarea")
        .forEach((b) => (b.disabled = false));
      if (doc) preview();
    }
  }
  async function refreshTracks() {
    await saveChain;
    const latest = await api("/api/library");
    const updated = latest.collections.find(
      (c) => c.id === cassette.id && c.kind === "cassette",
    );
    if (!updated) throw Error(t("カセットが見つかりません"));
    const hasLists = [...doc.elements, ...doc.backElements].some(
      (item) =>
        item.kind === "text" && ["sideA", "sideB"].includes(M.binding(item)),
    );
    mutate(() => M.refresh(doc, updated, latest.tracks));
    cassette = M.clone(updated);
    $j("heading").textContent = "J-CARD · " + cassette.name;
    toast(
      t(
        hasLists
          ? "最新のミックスリストを反映しました"
          : "曲一覧に連動するボックスがありません。右ペインの「連動する内容」を指定してください。",
      ),
    );
  }
  function copyElement(cut = false) {
    if (working || closing || drag || !e()) return;
    const item = M.clone(e());
    if (item.kind === "text") item.binding = M.binding(e()) || "manual";
    elementClipboard = {
      item,
      panels: M.panels(doc, face),
      face,
      cassetteId: cassette.id,
      cut,
      pasted: 0,
    };
    if (cut)
      mutate(() => {
        setElements(elements().filter((item) => item.id !== chosen));
        chosen = null;
      });
    else preview();
  }
  function pasteElement() {
    if (
      working ||
      closing ||
      drag ||
      !elementClipboard ||
      elements().length >= 150
    )
      return;
    const same =
      elementClipboard.face === face &&
      elementClipboard.cassetteId === cassette.id;
    const offset =
      elementClipboard.pasted * 2 + (same && !elementClipboard.cut ? 2 : 0);
    const item = M.pasteElement(elementClipboard, doc, face, id(), offset);
    const next = M.clone(doc);
    next[face === "back" ? "backElements" : "elements"].push(item);
    M.validate(next);
    add(item);
    elementClipboard.pasted++;
  }
  function add(item) {
    if (elements().length >= 150) {
      toast(t("J-CARDの要素は150個までです"));
      return;
    }
    mutate(() => {
      elements().push(item);
      chosen = item.id;
    });
  }
  const id = () => "item-" + crypto.randomUUID();
  async function imageFile(file) {
    if (!file) return;
    if (
      file.size > 5 * 1024 * 1024 ||
      !["image/png", "image/jpeg"].includes(file.type)
    )
      throw Error(t("5MB以下のPNG・JPEGを選択してください"));
    const src = await new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(r.result);
      r.onerror = reject;
      r.readAsDataURL(file);
    });
    const img = new Image();
    img.src = src;
    await img.decode();
    if (img.width * img.height > 40_000_000)
      throw Error(t("画像の解像度が大きすぎます"));
    add({
      id: id(),
      kind: "image",
      x: M.frontStart(doc, face) + 5,
      y: 25,
      w: Math.min(50, doc.front - 10),
      h: Math.min(60, (50 * img.height) / img.width),
      rotation: 0,
      src,
    });
  }
  async function exportPDF() {
    const snapshot = M.clone(doc);
    M.validate(snapshot);
    const w = M.width(snapshot) + 10,
      h = snapshot.height + 10,
      pages = [];
    await document.fonts?.ready;
    for (const outputFace of ["front", "back"]) {
      const url = URL.createObjectURL(
        new Blob([M.svg(snapshot, true, false, outputFace)], {
          type: "image/svg+xml",
        }),
      );
      try {
        const image = new Image();
        image.src = url;
        await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = Math.ceil((w / 25.4) * 300);
        canvas.height = Math.ceil((h / 25.4) * 300);
        const ctx = canvas.getContext("2d");
        ctx.fillStyle = "white";
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
        const blob = await new Promise((resolve) =>
          canvas.toBlob(resolve, "image/jpeg", 0.97),
        );
        if (!blob) throw Error(t("PDFを生成できませんでした"));
        pages.push({
          jpeg: new Uint8Array(await blob.arrayBuffer()),
          pixelWidth: canvas.width,
          pixelHeight: canvas.height,
        });
      } finally {
        URL.revokeObjectURL(url);
      }
    }
    download(
      M.pdfPages(pages, w, h, {
        paper: snapshot.paper,
        margin: snapshot.margin,
      }),
      "application/pdf",
      "pdf",
    );
  }
  async function action(fn) {
    if (working) return;
    working = true;
    $j("dialog")
      .querySelectorAll("button")
      .forEach((b) => (b.disabled = true));
    try {
      await fn();
    } catch (error) {
      toast(error.message);
    } finally {
      working = false;
      $j("dialog")
        .querySelectorAll("button")
        .forEach((b) => (b.disabled = false));
      if (doc) {
        preview();
        scheduleSave();
      }
    }
  }
  function bind() {
    $("jcard-btn").addEventListener("click", open);
    $j("close").addEventListener("click", close);
    $j("dialog").addEventListener("cancel", (ev) => {
      ev.preventDefault();
      close();
    });
    $j("face").addEventListener("change", () => {
      face = $j("face").value;
      chosen = elements()[0]?.id;
      render();
    });
    $j("add-panel").addEventListener("click", () => {
      try {
        const next = M.withExtensions(doc, [...doc.extensions, doc.front]);
        mutate(() => (doc = next));
      } catch (error) {
        toast(csTranslateMessage(error.message));
      }
    });
    $j("panels").addEventListener("change", (event) => {
      if (event.target.dataset.panel === undefined) return;
      const panels = [...doc.extensions];
      panels[Number(event.target.dataset.panel)] = Number(event.target.value);
      try {
        const next = M.withExtensions(doc, panels);
        mutate(() => (doc = next));
      } catch (error) {
        toast(csTranslateMessage(error.message));
        render();
      }
    });
    $j("panels").addEventListener("click", (event) => {
      const button = event.target.closest("[data-remove-panel]");
      if (!button) return;
      const panels = doc.extensions.filter(
        (_, i) => i !== Number(button.dataset.removePanel),
      );
      try {
        const next = M.withExtensions(doc, panels);
        mutate(() => (doc = next));
      } catch (error) {
        toast(csTranslateMessage(error.message));
      }
    });
    $j("undo").addEventListener("click", () => {
      if (!undo.length) return;
      redo.push(JSON.stringify(doc));
      doc = JSON.parse(undo.pop());
      render();
    });
    $j("redo").addEventListener("click", () => {
      if (!redo.length) return;
      undo.push(JSON.stringify(doc));
      doc = JSON.parse(redo.pop());
      render();
    });
    $j("refresh").addEventListener("click", () => action(refreshTracks));
    $j("cut").addEventListener("click", () => copyElement(true));
    $j("copy").addEventListener("click", () => copyElement(false));
    $j("paste").addEventListener("click", () => {
      try {
        pasteElement();
      } catch (error) {
        toast(error.message);
      }
    });
    $j("text-add").addEventListener("click", () =>
      add(
        M.text(
          id(),
          M.frontStart(doc, face) + 5,
          15,
          40,
          20,
          t("文字を入力"),
          12,
        ),
      ),
    );
    $j("rect-add").addEventListener("click", () =>
      add({
        id: id(),
        kind: "rect",
        x: M.frontStart(doc, face) + 5,
        y: 30,
        w: 30,
        h: 20,
        rotation: 0,
        color: "#b2c7b9",
      }),
    );
    $j("image-add").addEventListener("click", () => $j("image-file").click());
    $j("image-file").addEventListener("change", () =>
      action(async () => {
        await imageFile($j("image-file").files[0]);
        $j("image-file").value = "";
      }),
    );
    $j("delete").addEventListener("click", () => {
      if (e())
        mutate(() => {
          setElements(elements().filter((e) => e.id !== chosen));
          chosen = null;
        });
    });
    $j("duplicate").addEventListener("click", () => {
      if (e()) {
        const copy = M.clone(e());
        if (copy.kind === "text") copy.binding = M.binding(e()) || "manual";
        copy.id = id();
        copy.x += 2;
        copy.y += 2;
        add(copy);
      }
    });
    for (const [name, step] of [
      ["forward", 1],
      ["backward", -1],
    ])
      $j(name).addEventListener("click", () => {
        const i = elements().findIndex((e) => e.id === chosen),
          n = i + step;
        if (i >= 0 && n >= 0 && n < elements().length)
          mutate(() => {
            [elements()[i], elements()[n]] = [elements()[n], elements()[i]];
          });
      });
    for (const axis of ["horizontal", "vertical"])
      $j("center-" + axis).addEventListener("click", () => {
        if (e())
          mutate(() => {
            const item = e();
            if (axis === "horizontal") item.x = (M.width(doc) - item.w) / 2;
            else item.y = (doc.height - item.h) / 2;
          });
      });
    $j("layers").addEventListener("click", (event) => {
      const item = event.target.closest("[data-id]");
      if (item) {
        chosen = item.dataset.id;
        render();
      }
    });
    $j("spine-mode").addEventListener("change", () => {
      customSpine = $j("spine-mode").value === "custom";
      if (customSpine) {
        render();
        $j("spine").focus();
      } else {
        const next = M.clone(doc);
        next.spine = 12;
        try {
          M.validate(next);
          mutate(() => (doc = next));
        } catch (error) {
          toast(error.message);
          render();
        }
      }
    });
    for (const key of [
      "front",
      "spine",
      "flap",
      "height",
      "background",
      "paper",
      "margin",
    ])
      $j(key).addEventListener("change", () => {
        const next = M.clone(doc);
        next[key === "background" ? backgroundKey() : key] = [
          "background",
          "paper",
        ].includes(key)
          ? $j(key).value
          : Number($j(key).value);
        try {
          M.validate(next);
          mutate(() => (doc = next));
        } catch (error) {
          toast(error.message);
          render();
        }
      });
    $j("zoom").addEventListener("input", preview);
    $j("svg").addEventListener("click", () =>
      download(
        M.svg(doc, $j("guides").checked, false, face),
        "image/svg+xml",
        "svg",
        face === "back" ? "-inside" : "-outside",
      ),
    );
    $j("pdf").addEventListener("click", () => action(exportPDF));
    $j("json").addEventListener("click", () =>
      download(JSON.stringify(doc, null, 2), "application/json", "jcard.json"),
    );
    $j("load").addEventListener("click", () => $j("json-file").click());
    $j("json-file").addEventListener("change", () =>
      action(async () => {
        const file = $j("json-file").files[0];
        if (!file) return;
        if (file.size > 12 * 1024 * 1024)
          throw Error(t("J-CARDファイルが大きすぎます"));
        const next = M.validate(JSON.parse(await file.text()));
        mutate(() => {
          doc = M.upgrade(next);
          customSpine = doc.spine !== 12;
          face = "front";
          chosen = null;
        });
        $j("json-file").value = "";
      }),
    );
    const canvas = $j("canvas");
    canvas.addEventListener("pointerdown", (event) => {
      if (working || drag || event.button !== 0) return;
      const hit = event.target.closest("[data-jcard-id]");
      if (!hit) {
        chosen = null;
        render();
        return;
      }
      chosen = hit.dataset.jcardId;
      const item = e(),
        svg = canvas.querySelector("svg");
      const matrix = svg.getScreenCTM();
      if (!item || !matrix) return;
      const inverse = matrix.inverse();
      const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
        inverse,
      );
      drag = {
        pointerId: event.pointerId,
        mode: event.target.dataset.jcardHandle || "move",
        start: { x: point.x, y: point.y },
        original: M.clone(item),
        inverse,
        before: JSON.stringify(doc),
      };
      canvas.focus();
      canvas.setPointerCapture(event.pointerId);
      event.preventDefault();
      render();
    });
    canvas.addEventListener("pointermove", (event) => {
      if (!drag || event.pointerId !== drag.pointerId) return;
      const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
        drag.inverse,
      );
      const dx = point.x - drag.start.x,
        dy = point.y - drag.start.y;
      const step = $j("snap").checked ? 0.5 : 0.1;
      let next;
      if (drag.mode === "rotate")
        next = M.rotateBox(
          drag.original,
          drag.start,
          point,
          event.shiftKey ? 15 : 1,
        );
      else if (drag.mode !== "move")
        next = M.resizeBox(drag.original, drag.mode, dx, dy, step);
      else {
        const round = (n) =>
          Math.max(-300, Math.min(300, Math.round(n / step) * step));
        next = {
          ...drag.original,
          x: round(drag.original.x + dx),
          y: round(drag.original.y + dy),
        };
      }
      Object.assign(e(), next);
      preview();
    });
    const end = (event) => {
      if (!drag || event.pointerId !== drag.pointerId) return;
      if (event.type === "pointercancel") doc = JSON.parse(drag.before);
      else if (JSON.stringify(doc) !== drag.before) {
        undo.push(drag.before);
        if (undo.length > 40) undo.shift();
        redo = [];
      }
      const pointerId = drag.pointerId;
      drag = null;
      if (canvas.hasPointerCapture(pointerId))
        canvas.releasePointerCapture(pointerId);
      render();
    };
    canvas.addEventListener("pointerup", end);
    canvas.addEventListener("pointercancel", end);
    canvas.addEventListener("lostpointercapture", (event) => {
      if (drag)
        end({ ...event, pointerId: drag.pointerId, type: "pointercancel" });
    });
    $j("dialog").addEventListener("keydown", (event) => {
      if (drag) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          end({ pointerId: drag.pointerId, type: "pointercancel" });
        }
        return;
      }
      if (
        working ||
        closing ||
        event.target.isContentEditable ||
        /^(INPUT|TEXTAREA)$/.test(event.target.tagName)
      )
        return;
      if (
        (event.ctrlKey || event.metaKey) &&
        !event.altKey &&
        ["x", "c", "v"].includes(event.key.toLowerCase())
      ) {
        const key = event.key.toLowerCase();
        if (key !== "v" && !e()) return;
        if (key === "v" && !elementClipboard) return;
        event.preventDefault();
        if (key === "v") {
          try {
            pasteElement();
          } catch (error) {
            toast(error.message);
          }
        } else copyElement(key === "x");
        return;
      }
      if (event.target.tagName === "SELECT") return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        $j(event.shiftKey ? "redo" : "undo").click();
      }
      if (event.key === "Delete") {
        event.preventDefault();
        $j("delete").click();
      }
      if (
        e() &&
        ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)
      ) {
        event.preventDefault();
        const step = event.shiftKey ? 5 : 0.5;
        mutate(() => {
          const item = e(),
            key = /Left|Right/.test(event.key) ? "x" : "y";
          item[key] = Math.max(
            -300,
            Math.min(
              300,
              item[key] + (/Left|Up/.test(event.key) ? -step : step),
            ),
          );
        });
      }
    });
    window.addEventListener("beforeunload", (event) => {
      if (dirty()) {
        event.preventDefault();
        event.returnValue = "";
      }
    });
  }
  return { bind };
})();
