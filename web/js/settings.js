"use strict";
// Conversion between recording settings and form values.
const settingFields = [
  ["gap", "gap"],
  ["lead-in", "lead_in"],
  ["fill-threshold", "fill_threshold"],
  ["fade", "fade"],
  ["continuation", "continuation"],
  ["silence-db", "silence_db"],
  ["silence-min", "silence_min"],
  ["target-lufs", "target_lufs"],
  ["peak-db", "peak_db"],
];
function fillSettings() {
  const s = config();
  $("side-minutes").value = Math.floor(s.side_seconds / 60);
  $("side-seconds").value = Number((s.side_seconds % 60).toFixed(9));
  const preset = [46, 60, 74, 80, 90, 120].find(
    (n) => Math.abs(n * 30 - s.side_seconds) < 0.001,
  );
  $("tape-preset").value = preset || "custom";
  $("normalize").value = String(s.normalize);
  $("wav-sample-rate").value = String(s.wav_sample_rate ?? "");
  $("remove-dc").value = String(s.remove_dc || false);
  for (const [id, key] of settingFields)
    $(id).value = ["gap", "fade", "lead_in"].includes(key)
      ? Math.max(key === "fade" ? 0.5 : 0, Math.round((s[key] ?? 0) * 2) / 2)
      : s[key];
}
function readSettings() {
  const s = {
    side_seconds: +$("side-minutes").value * 60 + +$("side-seconds").value,
    normalize: $("normalize").value === "true",
    remove_dc: $("remove-dc").value === "true",
    wav_sample_rate:
      $("wav-sample-rate").value === "" ? null : +$("wav-sample-rate").value,
  };
  for (const [id, key] of settingFields)
    s[key] = key === "continuation" ? $(id).value : +$(id).value;
  return s;
}
function saveSettings() {
  stop();
  settings = readSettings();
  localStorage.setItem("cs-settings", JSON.stringify(settings));
  if (current()) {
    current().settings = structuredClone(settings);
    changed();
  }
  renderCache = null;
  draw();
}
