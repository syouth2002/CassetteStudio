"use strict";
// Collection/import buttons, track selection and drag/drop bindings.
function bindLibraryControls() {
  document.addEventListener("click", (e) => {
    const col = e.target.closest("[data-collection]");
    if (col) selectCollection(col.dataset.collection);
    const s = e.target.closest("[data-side],[data-show-side]");
    if (s) {
      side = +(s.dataset.side ?? s.dataset.showSide);
      selected.clear();
      draw();
    }
    const close = e.target.closest("[data-close]");
    if (close) $(close.dataset.close).close();
  });
  $("all-tracks").onclick = () => selectCollection("all");
  $("new-playlist").onclick = () => newCollection("playlist");
  $("new-cassette").onclick = () => newCollection("cassette");
  $("rename-btn").onclick = async () => {
    const c = current(),
      name = await askText(csText("新しい名前"), c.name);
    if (name?.trim()) {
      c.name = name.trim();
      changed();
    }
  };
  $("delete-collection").onclick = async () => {
    const c = current();
    if (
      !(await askConfirm(
        csText(
          "「{0}」を削除しますか？ 音源ファイルはライブラリに残ります。",
          c.name,
        ),
      ))
    )
      return;
    await saveChain;
    const r = await task(csText("削除しています…"), () =>
      api(`/api/collections/${c.id}`, {}, "DELETE"),
    );
    if (r) {
      lib.collections = lib.collections.filter((v) => v.id !== c.id);
      selectCollection("all");
    }
  };
  $("make-manual").onclick = () => makeTape(false);
  $("make-auto").onclick = () => makeTape(true);
  $("batch-start").onclick = async () => {
    const targets = [
      ...$("batch-targets").querySelectorAll("input:checked"),
    ].map((el) => el.value);
    if (!targets.length) {
      toast(csText("配置先の空カセットを選択してください"));
      return;
    }
    const tracks = [...batchTracks];
    $("batch-dialog").close();
    await task(
      csText(
        "カセットごとの録音時間に合わせて、重複しないように配置しています…",
      ),
      async () => {
        await saveChain;
        const result = await api("/api/fill-cassettes", { tracks, targets });
        for (const c of result.collections) {
          const i = lib.collections.findIndex((v) => v.id === c.id);
          lib.collections[i] = c;
        }
        if (
          player &&
          result.collections.some((c) => c.id === player.collection)
        )
          stop();
        renderCache = null;
        $("downloads").replaceChildren();
        selectCollection(result.collections[0].id);
        toast(
          csText(
            "{0}本に配置しました。未配置：{1}曲 · 収録済みのため除外：{2}曲{3}",
            result.collections.length,
            result.remaining,
            result.excluded,
            result.collections.length < targets.length
              ? csText("。曲を配置できなかったカセットは空のままです")
              : "",
          ),
        );
      },
    );
  };
  $("import-btn").onclick = () => $("file-input").click();
  $("file-input").onchange = (e) => {
    importFiles([...e.target.files]);
    e.target.value = "";
  };
  let dragDepth = 0;
  document.addEventListener("dragenter", (e) => {
    if ([...e.dataTransfer.types].includes("Files")) {
      e.preventDefault();
      dragDepth++;
      $("drop-overlay").style.display = "block";
    }
  });
  document.addEventListener("dragleave", (e) => {
    if ([...e.dataTransfer.types].includes("Files") && --dragDepth <= 0) {
      dragDepth = 0;
      $("drop-overlay").style.display = "none";
    }
  });
  document.addEventListener("dragover", (e) => e.preventDefault());
  document.addEventListener("drop", (e) => {
    e.preventDefault();
    dragDepth = 0;
    $("drop-overlay").style.display = "none";
    if (e.dataTransfer.files.length) importFiles([...e.dataTransfer.files]);
  });
  $("url-btn").onclick = () => $("url-dialog").showModal();
  $("url-form").onsubmit = async (e) => {
    e.preventDefault();
    const url = $("url-input").value,
      destination = active,
      s = structuredClone(config());
    $("url-dialog").close();
    await task(csText("URLから音声を取得して解析しています…"), async () => {
      const t = await api("/api/url", { url, settings: s });
      lib.tracks.push(t);
      const skipped = await attachTracks([t.id], destination);
      draw();
      if (!skipped) toast(csText("URLの音源を追加しました"));
    });
  };
  $("tracks").onchange = (e) => {
    if (e.target.type === "checkbox") {
      const i = +e.target.dataset.index;
      e.target.checked ? selected.add(i) : selected.delete(i);
      draw();
    }
  };
  $("tracks").onclick = (e) => {
    if (e.target.closest("input")) return;
    const row = e.target.closest("tr");
    if (!row) return;
    const i = +row.dataset.index;
    if (e.ctrlKey || e.metaKey) {
      selected.has(i) ? selected.delete(i) : selected.add(i);
    } else {
      selected = new Set([i]);
    }
    drawSelection();
  };
  $("tracks").ondblclick = (e) => {
    const row = e.target.closest("tr");
    if (row) startPlayback(+row.dataset.index).catch((e) => toast(e.message));
  };
  $("select-all").onchange = (e) => {
    selected = e.target.checked
      ? new Set(entries().map((_, i) => i))
      : new Set();
    draw();
  };
  $("tracks").ondragstart = (e) => {
    const row = e.target.closest("tr");
    if (!row || active === "all") return;
    drag = { index: +row.dataset.index, collection: active, side };
    e.dataTransfer.setData("text/plain", String(drag.index));
    e.dataTransfer.effectAllowed = "move";
  };
  $("tracks").ondragover = (e) => {
    e.preventDefault();
    document
      .querySelectorAll(".drag-over")
      .forEach((el) => el.classList.remove("drag-over"));
    e.target.closest("tr")?.classList.add("drag-over");
  };
  $("tracks").ondragend = () => {
    drag = null;
    document
      .querySelectorAll(".drag-over")
      .forEach((el) => el.classList.remove("drag-over"));
  };
  $("tracks").ondrop = (e) => {
    if (!drag) return;
    e.preventDefault();
    const row = e.target.closest("tr");
    if (!row || drag.collection !== active) return;
    const c = current(),
      to = +row.dataset.index;
    if (c.kind === "playlist") {
      const [v] = c.tracks.splice(drag.index, 1);
      c.tracks.splice(to, 0, v);
    } else {
      const [v] = c.sides[drag.side].splice(drag.index, 1);
      c.sides[side].splice(to, 0, v);
    }
    drag = null;
    selected.clear();
    changed();
  };
  $("remove-selected").onclick = () => {
    const c = current();
    if (!c || !selected.size) return;
    if (c.kind === "playlist")
      c.tracks = c.tracks.filter((_, i) => !selected.has(i));
    else c.sides[side] = c.sides[side].filter((_, i) => !selected.has(i));
    selected.clear();
    changed();
  };
  $("move-side").onclick = () => {
    const c = current();
    if (!isTape()) return;
    c.sides[1 - side].push(...c.sides[side].filter((_, i) => selected.has(i)));
    c.sides[side] = c.sides[side].filter((_, i) => !selected.has(i));
    selected.clear();
    changed();
  };
  $("add-library").onclick = () => {
    $("picker-list").innerHTML = lib.tracks
      .map(
        (t) =>
          `<label><input type="checkbox" value="${esc(t.id)}"><span>${esc(t.title)} <small> · ${time(duration(t))}</small></span></label>`,
      )
      .join("");
    $("picker-dialog").showModal();
  };
  $("picker-add").onclick = async () => {
    const ids = [...$("picker-list").querySelectorAll("input:checked")].map(
      (el) => el.value,
    );
    await attachTracks(ids);
    $("picker-dialog").close();
    changed();
  };
}
