"use strict";
// Entry point: register controls once, then load the library after localization.
async function init() {
  let canAnalyze = false;
  await task(csText("ライブラリを読み込んでいます…"), async () => {
    lib = await api("/api/library");
    draw();
    const s = await api("/api/status");
    canAnalyze = s.ffmpeg && s.ffprobe;
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
  if (canAnalyze) void detectMissingMono().catch((error) => toast(error.message));
}

bindCommonControls();
bindLibraryControls();
bindAnalysisControls();
bindLoudnessGraphControls();
bindEntriesControls();
bindSettingsControls();
bindPlaybackControls();
bindReferenceToneControls();
JCardEditor.bind();
csI18nReady.then(init).catch((error) => toast(error.message));
