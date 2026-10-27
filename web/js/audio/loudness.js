"use strict";
// Loudness display calculation; actual audio correction is performed by Rust.
function gainInfo(t, s = config()) {
  const l = s.remove_dc ? t.dc?.corrected_loudness : t.loudness,
    m = t.manual_gain_db || 0;
  if (!s.normalize && !m && !s.remove_dc)
    return { text: csText("補正なし"), gain: 0, limited: false };
  if (!l) return { text: csText("音量未解析"), gain: 0, limited: false };
  const requested =
    (s.normalize && l.integrated_lufs != null
      ? s.target_lufs - l.integrated_lufs
      : 0) + m;
  const gain = Math.min(
    requested,
    l.true_peak_db == null ? Infinity : s.peak_db - 0.1 - l.true_peak_db,
  );
  const limited = gain < requested - 0.01;
  return {
    text: `${db(gain)}${limited ? csText(" · ピーク制限") : ""}`,
    gain,
    limited,
  };
}
