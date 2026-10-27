"use strict";
// Loudness/DC analysis and manual gain button bindings.
function bindAnalysisControls() {
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
      const t = track(gainTrackId),
        l = config().remove_dc ? t.dc?.corrected_loudness : t.loudness,
        report = data.reports[gainTrackId];
      $("gain-title").textContent = t.title;
      $("manual-gain").value = t.manual_gain_db || 0;
      $("gain-measurement").textContent = csText(
        config().remove_dc
          ? "DC補正後の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}"
          : "元の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}",
        l?.integrated_lufs != null
          ? `${l.integrated_lufs.toFixed(1)} LUFS`
          : csText("測定対象外"),
        l?.true_peak_db != null
          ? `${l.true_peak_db.toFixed(1)} dBTP`
          : csText("測定対象外"),
        db(report.gain_db),
        report.limited ? csText("（ピーク制限）") : "",
      );
      $("gain-dc-measurement").textContent = t.dc
        ? csText(
            "DCオフセット：L {0}% / R {1}%",
            (t.dc.offset[0] * 100).toFixed(4),
            (t.dc.offset[1] * 100).toFixed(4),
          )
        : csText("DC未解析");
      draw();
      $("gain-dialog").showModal();
    });
  $("gain-form").onsubmit = async (e) => {
    e.preventDefault();
    const id = gainTrackId,
      value = +$("manual-gain").value;
    $("gain-dialog").close();
    await task(csText("音量補正を保存しています…"), async () => {
      const t = await api(
        `/api/tracks/${encodeURIComponent(id)}/gain`,
        { gain_db: value },
        "PUT",
      );
      mergeTracks([t]);
      if (player?.list.some((e) => e.track_id === id)) stop();
      renderCache = null;
      $("downloads").replaceChildren();
      draw();
      toast(csText("曲の音量補正を保存しました"));
    });
  };
}
