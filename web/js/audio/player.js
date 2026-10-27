"use strict";
// Music playback, seeking, track transitions and cancellation.
function stop() {
  window.csStopReferenceTone?.();
  playGeneration++;
  clearTimeout(gapTimer);
  clearInterval(countdown);
  gapTimer = null;
  countdown = null;
  audio.pause();
  if (audio.src)
    audio.currentTime =
      player?.mode === "playlist"
        ? player.list[player.index].offset +
          track(player.list[player.index].track_id).trim_start
        : 0;
  player = null;
  $("play-btn").textContent = "▶";
  $("playing-title").textContent = csText("再生する曲を選択");
  $("playing-detail").textContent = "Cassette Studio";
  $("seek").value = 0;
  $("elapsed").textContent = "0:00";
  $("remaining").textContent = "−0:00";
  markPlaying();
}
function tapeIndex(position, p) {
  let elapsed = p.lead_in || 0;
  for (let i = 0; i < p.list.length; i++) {
    elapsed += p.list[i].duration;
    if (position < elapsed + p.gap || i === p.list.length - 1) return i;
    elapsed += p.gap;
  }
  return 0;
}
function startAt(i, p) {
  return p.list
    .slice(0, i)
    .reduce((n, e) => n + e.duration + p.gap, p.lead_in || 0);
}
function updatePlayer() {
  if (!player) return;
  const p = player;
  let position = audio.currentTime,
    length = audio.duration || 0;
  if (p.mode === "playlist") {
    const e = p.list[p.index],
      t = track(e.track_id);
    position -= t.trim_start + e.offset;
    length = e.duration;
    if (position >= length - 0.02 && !audio.paused) {
      audio.pause();
      if (p.index < p.list.length - 1) {
        gapTimer = setTimeout(
          () => playPlaylistIndex(p.index + 1).catch((e) => toast(e.message)),
          p.gap * 1000,
        );
      } else {
        $("play-btn").textContent = "▶";
      }
    }
  } else {
    p.index = tapeIndex(position, p);
  }
  const e = p.list[p.index],
    t = e && track(e.track_id);
  $("playing-title").textContent = t?.title || p.name;
  $("playing-detail").textContent =
    p.mode === "tape"
      ? `${p.name} · SIDE ${p.side ? "B" : "A"} · ${p.index + 1}/${p.list.length}`
      : `${t?.artist || csText("プレイリスト")} · ${p.index + 1}/${p.list.length}`;
  $("elapsed").textContent = time(position);
  $("remaining").textContent = `−${time(length - position)}`;
  $("seek").value = length ? Math.min(1000, (position / length) * 1000) : 0;
  markPlaying();
}
async function playPlaylistIndex(i) {
  const p = player;
  if (!p || p.mode !== "playlist" || i < 0 || i >= p.list.length) return;
  clearTimeout(gapTimer);
  gapTimer = null;
  const gen = ++playGeneration;
  audio.pause();
  p.index = i;
  const e = p.list[i],
    t = track(e.track_id);
  const data = await api(
    `/api/preview/${encodeURIComponent(t.id)}`,
    p.settings,
  );
  if (gen !== playGeneration) return;
  mergeMeasurements(data.track ? [data.track] : []);
  draw();
  audio.src = data.url;
  await new Promise((resolve, reject) => {
    audio.onloadedmetadata = () => resolve();
    audio.onerror = () => reject(Error(csText("音声を再生できません")));
    audio.load();
  });
  if (gen !== playGeneration) return;
  audio.currentTime = t.trim_start + e.offset;
  await audio.play();
  $("play-btn").textContent = "Ⅱ";
  updatePlayer();
}
async function startPlayback(index = 0) {
  if (busy) return;
  if (isTape()) {
    const chosenSide = side;
    const cache = await task(
      csText("曲間とフェードを反映した録音用音声を準備しています…"),
      rendered,
    );
    if (!cache) return;
    await loadTape(cache, chosenSide, index, true);
  } else {
    const playable = entries()
        .map((e, row) => ({ e, row }))
        .filter((v) => v.e.duration > 0),
      list = playable.map((v) => v.e);
    if (!list.length) {
      toast(csText("再生できる曲がありません"));
      return;
    }
    const start = playable.findIndex((v) => v.row >= index);
    stop();
    player = {
      mode: "playlist",
      list: structuredClone(list),
      rows: playable.map((v) => v.row),
      index: 0,
      gap: config().gap,
      settings: structuredClone(config()),
      collection: active,
      name: current()?.name || csText("すべての曲"),
    };
    await task(csText("再生を準備しています…"), () =>
      playPlaylistIndex(start < 0 ? list.length - 1 : start),
    );
  }
}
async function loadTape(cache, chosenSide, index, play) {
  stop();
  const list = cache.sides[chosenSide];
  if (!list.length) {
    toast(csText("この面には曲がありません"));
    return;
  }
  const gen = playGeneration;
  player = {
    mode: "tape",
    list,
    index: Math.min(index, list.length - 1),
    side: chosenSide,
    gap: cache.settings.gap,
    lead_in: cache.settings.lead_in || 0,
    collection: cache.collection,
    name: cache.name,
  };
  audio.src = cache.urls[chosenSide];
  await new Promise((resolve, reject) => {
    audio.onloadedmetadata = resolve;
    audio.onerror = () => reject(Error(csText("音声を読み込めません")));
    audio.load();
  });
  if (gen !== playGeneration) return;
  audio.currentTime = player.index === 0 ? 0 : startAt(player.index, player);
  if (play) {
    await audio.play();
    $("play-btn").textContent = "Ⅱ";
  }
  updatePlayer();
}

async function skip(delta) {
  if (!player) return;
  const i = Math.max(0, Math.min(player.list.length - 1, player.index + delta));
  if (player.mode === "playlist")
    await task(csText("再生を準備しています…"), () => playPlaylistIndex(i));
  else {
    audio.currentTime = startAt(i, player);
    updatePlayer();
  }
}
