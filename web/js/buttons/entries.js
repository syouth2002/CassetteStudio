"use strict";
// Playback range and fade edit button bindings.
function bindEntriesControls() {
  $("edit-entry").onclick = () => {
    if (selected.size !== 1) return;
    entryAutoDuration = null;
    const e = entries()[[...selected][0]];
    for (const [id, key] of [
      ["entry-offset", "offset"],
      ["entry-duration", "duration"],
      ["entry-in", "fade_in"],
      ["entry-out", "fade_out"],
    ])
      $(id).value =
        key === "duration"
          ? e[key].toFixed(1)
          : key.startsWith("fade_")
            ? Math.round(e[key] * 2) / 2
            : e[key];
    $("entry-dialog").showModal();
  };
  $("entry-auto").onclick = () => {
    const index = [...selected][0],
      list = entries(),
      entry = list[index],
      s = config(),
      offset = +$("entry-offset").value;
    if (!Number.isFinite(offset) || offset < 0) {
      toast(csText("曲の長さとフェードの範囲を確認してください"));
      return;
    }
    const occupied =
      (s.lead_in || 0) +
      list.reduce((n, e, i) => n + (i === index ? 0 : e.duration), 0) +
      Math.max(0, list.length - 1) * s.gap;
    const fitted = fitTapeEntry(
      { ...entry, offset },
      duration(track(entry.track_id)),
      occupied,
      s,
    );
    if (!fitted) {
      toast(
        csText(
          "この面に再生時間を確保できません。ほかの曲を移動・削除してください",
        ),
      );
      return;
    }
    entryAutoDuration = fitted.duration;
    for (const [id, key] of [
      ["entry-duration", "duration"],
      ["entry-in", "fade_in"],
      ["entry-out", "fade_out"],
    ])
      $(id).value = key === "duration" ? fitted[key].toFixed(1) : fitted[key];
  };
  $("entry-duration").oninput = () => {
    entryAutoDuration = null;
  };
  $("entry-form").onsubmit = (e) => {
    e.preventDefault();
    const entry = entries()[[...selected][0]],
      candidate = { ...entry };
    for (const [id, key] of [
      ["entry-offset", "offset"],
      ["entry-duration", "duration"],
      ["entry-in", "fade_in"],
      ["entry-out", "fade_out"],
    ])
      candidate[key] = +$(id).value;
    if (
      entryAutoDuration !== null &&
      candidate.duration === Number(entryAutoDuration.toFixed(1))
    )
      candidate.duration = entryAutoDuration;
    else if (
      candidate.offset === entry.offset &&
      candidate.duration === Number(entry.duration.toFixed(1))
    )
      candidate.duration = entry.duration;
    const remaining = duration(track(entry.track_id)) - candidate.offset;
    if (
      candidate.duration > remaining &&
      candidate.duration - remaining <= 0.050001
    )
      candidate.duration = remaining;
    if (
      !Number.isFinite(candidate.fade_in) ||
      !Number.isFinite(candidate.fade_out) ||
      !Number.isFinite(candidate.offset) ||
      !Number.isFinite(candidate.duration) ||
      candidate.offset < 0 ||
      candidate.duration <= 0 ||
      candidate.fade_in < 0 ||
      candidate.fade_out < 0 ||
      candidate.offset + candidate.duration >
        duration(track(entry.track_id)) + 0.001 ||
      candidate.fade_in > candidate.duration ||
      candidate.fade_out > candidate.duration
    ) {
      toast(csText("曲の長さとフェードの範囲を確認してください"));
      return;
    }
    Object.assign(entry, candidate);
    $("entry-dialog").close();
    changed();
  };
}
