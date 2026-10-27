"use strict";
// Playback/output/export buttons and recording countdown bindings.
function bindPlaybackControls() {
  $("output-btn").hidden = !(
    typeof navigator.mediaDevices?.selectAudioOutput === "function" &&
    typeof audio.setSinkId === "function"
  );
  $("export-btn").onclick = () =>
    task(csText("A・B面をWAVに書き出しています…"), async () => {
      const cache = await rendered();
      showDownloads(cache);
      toast(csText("書き出しが完了しました。A・B面をそれぞれ保存できます"));
    });
  $("record-btn").onclick = () =>
    task(csText("デッキ録音用の音声を準備しています…"), async () => {
      const cache = await rendered();
      showDownloads(cache);
      await loadTape(cache, side, 0, false);
      $("record-detail").textContent =
        `${cache.name} / SIDE ${side ? "B" : "A"} / ${time(cache.settings.side_seconds)}`;
      $("record-start").textContent = csText("5秒後に再生開始");
      $("record-start").disabled = false;
      $("record-dialog").showModal();
    });
  $("record-start").onclick = () => {
    $("record-start").disabled = true;
    let remaining = 5;
    $("record-start").textContent = csText("{0}秒後に開始…", remaining);
    countdown = setInterval(() => {
      remaining--;
      $("record-start").textContent = csText("{0}秒後に開始…", remaining);
      if (remaining <= 0) {
        clearInterval(countdown);
        countdown = null;
        $("record-dialog").close();
        audio
          .play()
          .then(() => ($("play-btn").textContent = "Ⅱ"))
          .catch((e) => toast(e.message));
      }
    }, 1000);
  };
  $("record-dialog").addEventListener("close", () => {
    if (countdown) {
      clearInterval(countdown);
      countdown = null;
    }
  });
  $("play-btn").onclick = async () => {
    if (!player) {
      await startPlayback(selected.size ? Math.min(...selected) : 0);
      return;
    }
    if (gapTimer) {
      clearTimeout(gapTimer);
      gapTimer = null;
      $("play-btn").textContent = "▶";
      return;
    }
    if (audio.paused) {
      if (player.mode === "playlist") {
        const e = player.list[player.index],
          t = track(e.track_id);
        if (audio.currentTime >= t.trim_start + e.offset + e.duration - 0.03) {
          if (player.index < player.list.length - 1) {
            await playPlaylistIndex(player.index + 1);
            return;
          }
          audio.currentTime = t.trim_start + e.offset;
        }
      }
      await audio.play();
      $("play-btn").textContent = "Ⅱ";
    } else {
      audio.pause();
      $("play-btn").textContent = "▶";
    }
  };
  $("stop-btn").onclick = stop;
  $("prev-btn").onclick = () => skip(-1);
  $("next-btn").onclick = () => skip(1);
  $("seek").oninput = (e) => {
    if (!player) return;
    const ratio = +e.target.value / 1000;
    if (player.mode === "tape")
      audio.currentTime = ratio * (audio.duration || 0);
    else {
      const entry = player.list[player.index];
      audio.currentTime =
        track(entry.track_id).trim_start +
        entry.offset +
        ratio * entry.duration;
    }
    updatePlayer();
  };
  $("volume").oninput = (e) => (audio.volume = +e.target.value);
  $("output-btn").onclick = async () => {
    if (!navigator.mediaDevices?.selectAudioOutput || !audio.setSinkId) {
      toast(
        csText(
          "このブラウザでは出力先選択に対応していません。OSの音声設定で出力先を選んでください",
        ),
      );
      return;
    }
    try {
      const device = await navigator.mediaDevices.selectAudioOutput();
      await audio.setSinkId(device.deviceId);
      toast(csText("出力先：{0}", device.label || csText("選択したデバイス")));
    } catch (e) {
      toast(e.message);
    }
  };
  audio.addEventListener("timeupdate", updatePlayer);
  audio.addEventListener("ended", () => {
    if (player?.mode === "tape") {
      $("play-btn").textContent = "▶";
      toast(
        player.side === 0
          ? csText(
              "A面が終了しました。テープを裏返し、B面を選んで録音してください",
            )
          : csText("B面が終了しました"),
      );
    }
  });
  document.addEventListener("keydown", (e) => {
    if (
      e.code === "Space" &&
      !["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(e.target.tagName) &&
      !document.querySelector("dialog[open]")
    ) {
      e.preventDefault();
      $("play-btn").click();
    }
  });
}
