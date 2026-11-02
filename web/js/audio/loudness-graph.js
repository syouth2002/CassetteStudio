"use strict";
// Selection-scoped analysis; SVG stays sharp at any window size.
function bindLoudnessGraphControls() {
  let data = null,
    generation = 0,
    target = -18,
    cassetteId = null;
  const dialog = $("lufs-dialog"),
    plot = $("lufs-plot");
  function drawGraph() {
    if (!data) return;
    const bounds = plot.getBoundingClientRect?.();
    const key = $("lufs-window").value,
      width = Math.max(320, bounds?.width || 880),
      height = Math.max(240, bounds?.height || 440),
      right = width - 22,
      floor = height - 40;
    const values = [
      target,
      ...[data.original, data.processed]
        .flatMap((s) => s.points.map((p) => p[key]))
        .filter(Number.isFinite),
    ];
    const bottom = Math.min(
        -45,
        Math.floor(values.reduce((a, b) => Math.min(a, b), target) / 5) * 5,
      ),
      top = Math.max(
        0,
        Math.ceil(values.reduce((a, b) => Math.max(a, b), target) / 5) * 5,
      );
    const x = (t) => 58 + (t / data.duration) * (right - 58),
      y = (v) => 24 + ((top - v) / (top - bottom)) * (floor - 24);
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    svg.setAttribute("width", "100%");
    svg.style.display = "block";
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", csText("LUFSグラフ"));
    function add(name, attrs, text) {
      const el = document.createElementNS(svg.namespaceURI, name);
      for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
      if (text !== undefined) el.textContent = text;
      svg.appendChild(el);
      return el;
    }
    for (let v = bottom; v <= top; v += 5) {
      add("line", { x1: 58, x2: right, y1: y(v), y2: y(v), stroke: "#d8d5ce" });
      add(
        "text",
        {
          x: 50,
          y: y(v) + 4,
          "text-anchor": "end",
          fill: "#555",
          "font-size": 12,
        },
        v,
      );
    }
    for (let i = 0; i <= 5; i++)
      add(
        "text",
        {
          x: x((data.duration * i) / 5),
          y: height - 15,
          "text-anchor": "middle",
          fill: "#555",
          "font-size": 12,
        },
        time((data.duration * i) / 5),
      );
    add("text", { x: 8, y: 12, fill: "#555", "font-size": 12 }, "LUFS");
    add("line", {
      x1: 58,
      x2: right,
      y1: y(target),
      y2: y(target),
      stroke: "#946322",
      "stroke-dasharray": "6 4",
    });
    for (const [i, marker] of (data.markers || []).entries()) {
      const line = add("line", {
        x1: x(marker.time),
        x2: x(marker.time),
        y1: 24,
        y2: floor,
        stroke: "#b8b1a4",
        "stroke-dasharray": "2 4",
      });
      const title = document.createElementNS(svg.namespaceURI, "title");
      title.textContent = `${i + 1}. ${marker.title} (${time(marker.time)}–${time(marker.end)})`;
      line.appendChild(title);
      add(
        "text",
        { x: x(marker.time) + 3, y: 18, fill: "#777", "font-size": 10 },
        i + 1,
      );
    }
    for (const [series, color] of [
      [data.original, "#777c87"],
      [data.processed, "#2e7467"],
    ]) {
      let d = "",
        connected = false;
      for (const p of series.points) {
        if (!Number.isFinite(p[key])) {
          connected = false;
          continue;
        }
        d += `${connected ? "L" : "M"}${x(p.time).toFixed(2)},${y(p[key]).toFixed(2)} `;
        connected = true;
      }
      add("path", { d, fill: "none", stroke: color, "stroke-width": 2 });
    }
    plot.replaceChildren(svg);
    $("lufs-tracks").textContent = (data.markers || [])
      .map((m, i) => `${i + 1}. ${time(m.time)} ${m.title}`)
      .join("\n");
    const stat = (s) =>
      `${s.integrated_lufs == null ? "—" : s.integrated_lufs.toFixed(1)} LUFS / ${s.true_peak_db == null ? "—" : s.true_peak_db.toFixed(1)} dBTP`;
    $("lufs-stats").textContent = csText(
      "補正前：{0}\n録音設定適用後：{1}",
      stat(data.original),
      stat(data.processed),
    );
    $("lufs-target").textContent = csText(
      "目標：{0} LUFS（破線）",
      target.toFixed(1),
    );
  }
  $("lufs-window").onchange = drawGraph;
  if (typeof ResizeObserver === "function") {
    new ResizeObserver(() => {
      if (dialog.open && data) drawGraph();
    }).observe(plot);
  }
  dialog.addEventListener("close", () => {
    generation++;
  });
  $("lufs-open").onclick = async () => {
    if (selected.size !== 1 || busy) return;
    const entry = structuredClone(entries()[[...selected][0]]),
      t = track(entry.track_id),
      s = config(),
      token = ++generation;
    cassetteId = null;
    dialog.setAttribute("data-mode", "track");
    $("lufs-side-row").hidden = true;
    $("lufs-note").textContent = csText(
      "選択した再生範囲を解析します。先頭の測定窓が満たない部分と無音は線を表示しません。",
    );
    $("lufs-tracks").textContent = "";
    data = null;
    target = s.target_lufs;
    $("lufs-title").textContent = t.title;
    plot.textContent = csText("LUFSを解析しています…");
    $("lufs-stats").textContent = "";
    $("lufs-target").textContent = "";
    dialog.showModal();
    $("lufs-open").disabled = true;
    try {
      await saveChain;
      const result = await api(
        `/api/tracks/${encodeURIComponent(t.id)}/loudness-graph`,
        { settings: s, entry: current()?.kind === "cassette" ? entry : null },
      );
      if (token !== generation || !dialog.open) return;
      data = result;
      drawGraph();
    } catch (e) {
      if (token === generation && dialog.open) {
        plot.textContent = csText("解析に失敗しました");
        toast(e.message);
      }
    } finally {
      if (!dialog.open) drawSelection();
    }
  };
  async function loadSide() {
    if (!cassetteId) return;
    const id = cassetteId,
      chosen = +$("lufs-side").value,
      token = ++generation;
    data = null;
    plot.textContent = csText("LUFSを解析しています…");
    $("lufs-stats").textContent = "";
    $("lufs-tracks").textContent = "";
    $("lufs-target").textContent = "";
    $("lufs-title").textContent =
      `${current()?.name || ""} / SIDE ${chosen === 0 ? "A" : "B"}`;
    try {
      await saveChain;
      const result = await api(
        `/api/collections/${encodeURIComponent(id)}/loudness-graph`,
        { side: chosen },
      );
      if (token !== generation || !dialog.open) return;
      data = result.graph;
      target = result.target_lufs;
      $("lufs-title").textContent = result.title;
      drawGraph();
    } catch (e) {
      if (token === generation && dialog.open) {
        plot.textContent = csText("解析に失敗しました");
        toast(e.message);
      }
    }
  }
  $("lufs-side").onchange = loadSide;
  $("lufs-side-open").onclick = () => {
    const c = current();
    if (busy || c?.kind !== "cassette") return;
    cassetteId = c.id;
    dialog.setAttribute("data-mode", "side");
    $("lufs-side-row").hidden = false;
    $("lufs-side").value = String(side);
    $("lufs-note").textContent = csText(
      "面全体を解析します。先頭・曲間・末尾の無音とフェードを含みます。灰色は音量・DC補正前、番号は曲の開始位置です。",
    );
    dialog.showModal();
    loadSide();
  };
  dialog.addEventListener("close", drawSelection);
}
