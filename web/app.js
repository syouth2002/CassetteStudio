"use strict";
// Entry point: register controls once, then load the library after localization.
async function init() {
  await task(csText("ライブラリを読み込んでいます…"), async () => {
    lib = await api("/api/library");
    draw();
    const s = await api("/api/status");
    $("tool-status").textContent =
      s.ffmpeg && s.ffprobe
        ? csText(
            "● 音声エンジン準備完了{0}",
            s.yt_dlp ? csText(" · URL対応") : "",
          )
        : csText("△ 音声ツールのセットアップが必要");
    if (!s.ffmpeg || !s.ffprobe)
      toast(
        csText(
          "FFmpegが見つかりません。scripts/setup-tools.ps1 またはセットアップ手順を実行してください",
        ),
      );
  });
}

bindCommonControls();
bindLibraryControls();
bindAnalysisControls();
bindEntriesControls();
bindSettingsControls();
bindPlaybackControls();
bindReferenceToneControls();
csI18nReady.then(init).catch((error) => toast(error.message));
