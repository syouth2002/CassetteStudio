"use strict";
// Reference tone signal generation and lifecycle.
// Share the music element so output-device selection and playback volume match.
let referenceToneUrl = null,
  referenceToneTimer = null,
  referenceToneGeneration = 0;
function referenceToneWav(frequency, level) {
  const rate = 48000,
    frames = rate,
    buffer = new ArrayBuffer(44 + frames * 4),
    view = new DataView(buffer);
  const text = (offset, value) => {
    for (let i = 0; i < value.length; i++)
      view.setUint8(offset + i, value.charCodeAt(i));
  };
  text(0, "RIFF");
  view.setUint32(4, buffer.byteLength - 8, true);
  text(8, "WAVE");
  text(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 2, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 16, true);
  text(36, "data");
  view.setUint32(40, frames * 4, true);
  const amplitude = 32767 * Math.pow(10, level / 20);
  for (let i = 0; i < frames; i++) {
    const sample = Math.round(
      amplitude * Math.sin((2 * Math.PI * frequency * i) / rate),
    );
    view.setInt16(44 + i * 4, sample, true);
    view.setInt16(46 + i * 4, sample, true);
  }
  return buffer;
}
window.csStopReferenceTone = () => {
  referenceToneGeneration++;
  clearTimeout(referenceToneTimer);
  referenceToneTimer = null;
  if (referenceToneUrl) {
    audio.pause();
    audio.loop = false;
    audio.removeAttribute("src");
    audio.load();
    URL.revokeObjectURL(referenceToneUrl);
    referenceToneUrl = null;
  }
  $("tone-start").disabled = false;
  $("tone-stop").disabled = true;
  $("tone-frequency").disabled = false;
  $("tone-level").disabled = false;
  $("tone-status").textContent = csText("停止中");
};
function bindReferenceToneControls() {
  $("tone-open").onclick = () => {
    stop();
    $("options-dialog").close();
    $("tone-dialog").showModal();
  };
  $("tone-start").onclick = async () => {
    stop();
    const frequency = Number($("tone-frequency").value),
      level = Number($("tone-level").value);
    if (
      ![400, 1000].includes(frequency) ||
      !Number.isFinite(level) ||
      level < -30 ||
      level > -3
    ) {
      toast(csText("基準音の設定を確認してください"));
      return;
    }
    const generation = referenceToneGeneration;
    referenceToneUrl = URL.createObjectURL(
      new Blob([referenceToneWav(frequency, level)], { type: "audio/wav" }),
    );
    audio.onloadedmetadata = null;
    audio.onerror = null;
    audio.src = referenceToneUrl;
    audio.loop = true;
    $("tone-start").disabled = true;
    $("tone-stop").disabled = false;
    $("tone-frequency").disabled = true;
    $("tone-level").disabled = true;
    $("tone-status").textContent = csText(
      "基準音を出力中：{0} Hz / {1} dBFS（ピーク）",
      frequency,
      level,
    );
    try {
      await audio.play();
      if (generation !== referenceToneGeneration) return;
      referenceToneTimer = setTimeout(() => {
        stop();
      }, 60000);
    } catch (e) {
      if (generation === referenceToneGeneration) {
        stop();
        toast(csText("基準音を再生できません: {0}", e.message));
      }
    }
  };
  $("tone-stop").onclick = () => stop();
  $("tone-dialog").addEventListener("close", () => stop());
  window.addEventListener("pagehide", () => window.csStopReferenceTone());
}
