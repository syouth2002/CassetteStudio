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
    if (e.target.value !== "custom")
      $("side-minutes").value = +e.target.value / 2;
  };
  $("side-minutes").oninput = () => ($("tape-preset").value = "custom");
  $("settings-dialog").addEventListener("close", () => {
    if ($("settings-dialog").returnValue === "save") saveSettings();
  });
  $("reanalyze-btn").onclick = async () => {
    if (
      !(await askConfirm(
        csText(
          "ライブラリ全曲の無音を再解析します。作成済みカセットがある場合は先に削除が必要です。続けますか？",
        ),
      ))
    )
      return;
    const s = structuredClone(config());
    stop();
    $("options-dialog").close();
    await task(csText("ライブラリの無音を再解析しています…"), async () => {
      lib = await api("/api/reanalyze", s, "PUT");
      settings = s;
      localStorage.setItem("cs-settings", JSON.stringify(s));
      renderCache = null;
      draw();
      toast(csText("無音の再解析が完了しました"));
    });
  };
}
