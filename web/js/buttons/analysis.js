"use strict";
// Loudness/DC analysis and manual gain button bindings.
function bindAnalysisControls() {
  const audition = $("gain-audio");
  let auditionGeneration = 0;
  const pauseAudition = () => { auditionGeneration++; audition.pause(); audition.hidden = true; audition.removeAttribute("src"); };
  $("gain-dialog").addEventListener("close", pauseAudition);
  $("pseudo-stereo").onchange = pauseAudition;
  $("manual-gain").oninput = pauseAudition;
  function showTrackAudio(t, report = gainInfo(t)) {
    const l = t.pseudo_stereo
      ? (config().remove_dc ? t.pseudo_analysis?.corrected_loudness : t.pseudo_analysis?.loudness)
      : (config().remove_dc ? t.dc?.corrected_loudness : t.loudness);
    $("pseudo-stereo").disabled = t.mono !== true;
    $("pseudo-stereo").value = String(t.pseudo_stereo || 0);
    $("mono-status").textContent = csText(t.mono === true
      ? "モノラル：疑似ステレオ化を選択できます"
      : "ステレオ・多チャンネル音源：疑似ステレオ化は選択できません");
    $("gain-title").textContent = t.title;
    $("manual-gain").value = t.manual_gain_db || 0;
    $("gain-measurement").textContent = csText(
      t.pseudo_stereo ? "疑似ステレオ処理後の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}" : config().remove_dc
        ? "DC補正後の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}"
        : "元の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}",
      l?.integrated_lufs != null
        ? `${l.integrated_lufs.toFixed(1)} LUFS`
        : csText("測定対象外"),
      l?.true_peak_db != null
        ? `${l.true_peak_db.toFixed(1)} dBTP`
        : csText("測定対象外"),
      db(report.gain_db ?? report.gain),
      report.limited ? csText("（ピーク制限）") : "",
    );
    $("gain-dc-measurement").textContent = t.dc
      ? csText(
        "DCオフセット：L {0}% / R {1}%",
        (t.dc.offset[0] * 100).toFixed(4),
        (t.dc.offset[1] * 100).toFixed(4),
        )
      : csText("DC未解析");
  }
  $("analyze-volume").onclick = () =>
    task(csText("曲全体の音量とピークを解析しています…"), async () => {
      const es = entries(),
        ids = [
          ...new Set(
            (selected.size ? [...selected].map((i) => es[i]) : es).map(
              (e) => e.track_id,
            ),
          ),
        ];
      if (!ids.length) {
        toast(csText("音源を追加してください"));
        return;
      }
      const data = await api("/api/loudness", {
        tracks: ids,
        settings: config(),
      });
      mergeTracks(data.tracks);
      draw();
      toast(
        csText(
          "{0}曲の音量・DC解析が完了しました。録音設定で補正をONにできます",
          ids.length,
        ),
      );
    });
  $("adjust-volume").onclick = () =>
    task(csText("曲の音量を確認しています…"), async () => {
      if (selected.size !== 1) return;
      gainTrackId = entries()[[...selected][0]].track_id;
      const data = await api("/api/loudness", {
        tracks: [gainTrackId],
        settings: config(),
      });
      mergeTracks(data.tracks);
      pauseAudition();
      showTrackAudio(track(gainTrackId), data.reports[gainTrackId]);
      draw();
      $("gain-dialog").showModal();
    });
  $("gain-form").onsubmit = async (e) => {
    e.preventDefault();
    if (busy) return;
    const id = gainTrackId,
      value = +$("manual-gain").value,
      strength = +$("pseudo-stereo").value,
      preview = e.submitter?.id === "gain-preview";
    audition.pause();
    const generation = ++auditionGeneration;
    await task(csText("曲の音声設定を保存しています…"), async () => {
      if ((track(id)?.pseudo_stereo || 0) !== strength) {
        const updated = await api("/api/tracks/" + encodeURIComponent(id) + "/pseudo-stereo", { strength }, "PUT");
        mergeTracks([updated]);
      }
      const t = await api("/api/tracks/" + encodeURIComponent(id) + "/gain", { gain_db: value }, "PUT");
      mergeTracks([t]);
      if (player?.list.some((e) => e.track_id === id)) stop();
      renderCache = null;
      $("downloads").replaceChildren();
      showTrackAudio(t);
      draw();
      if (preview) {
        stop();
        const data = await api("/api/preview/" + encodeURIComponent(id), config());
        mergeMeasurements(data.track ? [data.track] : []);
        if (generation !== auditionGeneration || !$("gain-dialog").open) return;
        if (audio.sinkId && audition.setSinkId) await audition.setSinkId(audio.sinkId);
        if (generation !== auditionGeneration || !$("gain-dialog").open) return;
        audition.src = data.url;
        audition.volume = audio.volume;
        audition.hidden = false;
        audition.onloadedmetadata = () => { audition.currentTime = t.trim_start; };
        audition.ontimeupdate = () => { if (audition.currentTime >= t.trim_end) audition.pause(); };
        audition.load();
        await audition.play().catch(() => {});
      } else {
        $("gain-dialog").close();
      }
      toast(csText("曲の音声設定を保存しました"));
    });
  };
}
