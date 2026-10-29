"use strict";
// Options, recording settings and silence reanalysis button bindings.
function bindSettingsControls() {
  $("options-btn").onclick = () => $("options-dialog").showModal();
  $("settings-btn").onclick = () => {
    fillSettings();
    $("settings-dialog").returnValue = "";
    $("settings-dialog").showModal();
  };
  $("tape-preset").onchange = (e) => {
    if (e.target.value !== "custom") {
      $("side-minutes").value = +e.target.value / 2;
      $("side-seconds").value = 0;
    }
  };
  for (const id of ["side-minutes", "side-seconds"])
    $(id).oninput = () => ($("tape-preset").value = "custom");
  $("settings-dialog").addEventListener("close", () => {
    if ($("settings-dialog").returnValue === "save") saveSettings();
  });
  $("reanalyze-btn").onclick = async () => {
    const list = entries();
    const ids = [
      ...new Set([...selected].map((i) => list[i]?.track_id).filter(Boolean)),
    ];
    if (!ids.length) {
      toast(csText("無音を再解析する曲を選択してください"));
      return;
    }
    const s = structuredClone(config());
    stop();
    $("options-dialog").close();
    $("workspace").inert = true;
    try {
      await task(
        csText("選択した{0}曲の無音を再解析しています…", ids.length),
        async () => {
          await saveChain;
          const updated = await api(
            "/api/reanalyze",
            { tracks: ids, settings: s },
            "PUT",
          );
          stop();
          lib = updated;
          selected.clear();
          renderCache = null;
          downloadCaches.clear();
          draw();
          const over = lib.collections.filter(
            (c) =>
              c.kind === "cassette" &&
              c.sides.some(
                (es) => total(es, c.settings) > c.settings.side_seconds + 0.001,
              ),
          ).length;
          toast(
            csText(
              over
                ? "無音の再解析が完了しました。{0}件のカセットが時間超過です。曲の移動や再生範囲の自動設定で調整してください"
                : "無音の再解析が完了し、ミックスリストの再生範囲を更新しました",
              over,
            ),
          );
        },
      );
    } finally {
      $("workspace").inert = false;
    }
  };
}
