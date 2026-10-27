"use strict";
// Library/cassette DOM rendering. Playback UI markers are kept here as well.
function drawNav() {
  $("track-count").textContent = lib.tracks.length;
  $("all-tracks").classList.toggle("active", active === "all");
  for (const [kind, target] of [
    ["playlist", "playlists"],
    ["cassette", "cassettes"],
  ]) {
    $(target).innerHTML = lib.collections
      .filter((c) => c.kind === kind)
      .map(
        (c) =>
          `<button class="collection-item ${c.id === active ? "active" : ""}" data-collection="${esc(c.id)}">${kind === "cassette" ? '<span class="cassette-art" aria-hidden="true">● ●</span>' : ""}<span class="collection-text">${esc(c.name)}<small>${kind === "cassette" ? csText("A {0}曲 · B {1}曲", c.sides[0].length, c.sides[1].length) : csText("{0}曲", c.tracks.length)}</small></span></button>`,
      )
      .join("");
  }
}
function drawSelection() {
  const list = entries();
  document.querySelectorAll("#tracks tr").forEach((row) => {
    const chosen = selected.has(+row.dataset.index);
    row.classList.toggle("selected", chosen);
    row.querySelector("input").checked = chosen;
  });
  $("selection-count").textContent = selected.size
    ? csText("{0}曲選択", selected.size)
    : "";
  $("adjust-volume").disabled = selected.size !== 1;
  $("edit-entry").disabled = selected.size !== 1;
  $("move-side").disabled = !selected.size;
  $("remove-selected").disabled = !selected.size || active === "all";
  $("select-all").checked = list.length > 0 && selected.size === list.length;
  $("select-all").indeterminate =
    selected.size > 0 && selected.size < list.length;
}
function draw() {
  syncDownloads();
  drawNav();
  const c = current(),
    tape = isTape(),
    list = entries();
  $("settings-btn").textContent = csText(
    tape ? "⚙ 録音設定" : "⚙ 作成・取り込み設定",
  );
  $("analyze-volume").title = csText(
    "選択した曲、未選択なら表示中の曲を解析します。結果は同じ音源を使う全カセットで共有します。",
  );
  selected = new Set([...selected].filter((i) => i < list.length));
  $("collection-name").textContent = c?.name || csText("すべての曲");
  $("collection-type").textContent = tape
    ? "YOUR PERSONAL MIXTAPE"
    : c
      ? "PLAYLIST"
      : "YOUR MUSIC COLLECTION";
  $("collection-summary").textContent = tape
    ? csText(
        "{0} × 2面 / 曲間 {1}秒 / {2}",
        time(config().side_seconds),
        config().gap,
        config().continuation === "restart"
          ? csText("引き継ぎ：最初から")
          : csText("引き継ぎ：途中からフェードイン"),
      )
    : csText(
        "{0}曲 · 無音除去後 {1}",
        list.length,
        time(list.reduce((n, e) => n + e.duration, 0)),
      );
  for (const id of ["rename-btn", "delete-collection", "add-library"])
    $(id).hidden = !c;
  for (const id of [
    "cassette-overview",
    "side-tabs",
    "export-btn",
    "record-btn",
    "move-side",
    "edit-entry",
  ])
    $(id).hidden = !tape;
  $("list-label").hidden = tape;
  $("edit-entry").disabled = selected.size !== 1;
  $("move-side").disabled = !selected.size;
  $("remove-selected").disabled = !selected.size || active === "all";
  $("selection-count").textContent = selected.size
    ? csText("{0}曲選択", selected.size)
    : "";
  $("select-all").checked = list.length > 0 && selected.size === list.length;
  $("adjust-volume").disabled = selected.size !== 1;
  $("select-all").indeterminate =
    selected.size > 0 && selected.size < list.length;
  $("make-manual").hidden = tape;
  $("make-auto").hidden = tape && !c.sides.every((es) => es.length === 0);
  $("make-auto").textContent = tape
    ? csText("✦ 空のカセットへ自動配置")
    : csText("✦ 自動でカセット作成");
  $("edit-help").textContent =
    active === "all"
      ? csText("曲を選択してカセットやプレイリストを作成できます")
      : csText("⋮⋮ ドラッグで曲順変更 · ダブルクリックで再生");
  $("tracks").innerHTML = list
    .map((e, i) => {
      const t = track(e.track_id);
      if (!t) return "";
      const badges = [];
      if (e.continuation)
        badges.push(csText('<span class="badge">A面から引き継ぎ</span>'));
      if (e.fade_in)
        badges.push(
          csText(
            '<span class="badge fade">IN {0}秒</span>',
            e.fade_in.toFixed(1),
          ),
        );
      if (e.fade_out)
        badges.push(
          csText(
            '<span class="badge fade">OUT {0}秒</span>',
            e.fade_out.toFixed(1),
          ),
        );
      if (e.offset)
        badges.push(`<span class="badge">${time(e.offset)}〜</span>`);
      if (duration(t) <= 0)
        badges.push(csText('<span class="badge warning">全体が無音</span>'));
      else if (t.duration - duration(t) > 0.05)
        badges.push(csText('<span class="badge">無音除去</span>'));
      const correction = gainInfo(t);
      if (config().remove_dc)
        badges.push(
          `<span class="badge" title="${esc(t.dc ? csText("DCオフセット：L {0}% / R {1}%", (t.dc.offset[0] * 100).toFixed(4), (t.dc.offset[1] * 100).toFixed(4)) : csText("未解析"))}">${csText(t.dc ? "DC除去" : "DC未解析")}</span>`,
        );
      if (config().normalize || config().remove_dc || t.manual_gain_db)
        badges.push(
          `<span class="badge ${correction.limited ? "warning" : "gain-badge"}" title="${esc(t.loudness?.integrated_lufs != null ? csText("元の音量 {0} LUFS", t.loudness.integrated_lufs.toFixed(1)) : csText("未解析"))}">${esc(correction.text)}</span>`,
        );
      return csText(
        '<tr data-index="{0}" draggable="{1}" class="{2}"><td><input type="checkbox" aria-label="{3}を選択" data-index="{4}" {5}></td><td>{6}{7}</td><td class="title-cell"><span class="song-title">{8}</span><small>{9}</small></td><td>{10}</td><td>{11}</td><td>{12}</td></tr>',
        i,
        active !== "all",
        selected.has(i) ? "selected" : "",
        esc(t.title),
        i,
        selected.has(i) ? "checked" : "",
        active === "all" ? "" : '<span class="handle">⋮⋮</span>',
        String(i + 1).padStart(2, "0"),
        esc(t.title),
        esc(t.artist || t.filename),
        time(e.duration),
        time(t.duration),
        badges.join(""),
      );
    })
    .join("");
  $("empty-state").hidden = list.length > 0;
  if (tape) {
    $("cassette-overview").innerHTML = c.sides
      .map((es, i) => {
        const n = total(es),
          room = config().side_seconds - n;
        return `<div class="side-card ${i === side ? "active" : ""} ${room < -0.001 ? "over" : ""}" data-show-side="${i}"><div class="side-title"><span>SIDE ${i ? "B" : "A"} · ${es.length} TRACKS</span><span>${time(n)} / ${time(config().side_seconds)}</span></div><div class="bar"><div style="width:${Math.min(100, (n / config().side_seconds) * 100)}%"></div></div><p>${room < -0.001 ? csText("時間超過 {0} — 曲を移動・削除してください", time(-room)) : csText("残り {0}", time(room))}</p></div>`;
      })
      .join("");
    document
      .querySelectorAll("[data-side]")
      .forEach((b) => b.classList.toggle("active", +b.dataset.side === side));
  }
  markPlaying();
}
function markPlaying() {
  document
    .querySelectorAll("#tracks tr")
    .forEach((row) =>
      row.classList.toggle(
        "playing",
        !!player &&
          active === player.collection &&
          (!isTape() || side === player.side) &&
          +row.dataset.index === (player.rows?.[player.index] ?? player.index),
      ),
    );
}
