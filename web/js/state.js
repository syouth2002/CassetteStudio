"use strict";
// Shared application state. Loaded before features; no event handlers are registered here.
const defaults = {
  side_seconds: 1800,
  gap: 2,
  lead_in: 0,
  wav_sample_rate: null,
  fill_threshold: 30,
  fade: 5,
  continuation: "restart",
  silence_db: -50,
  silence_min: 0.5,
  normalize: false,
  remove_dc: false,
  target_lufs: -18,
  peak_db: -1,
};
let gainTrackId = null,
  entryAutoDuration = null;
const downloadCaches = new Map();
let lib = { tracks: [], collections: [] },
  active = "all",
  side = 0,
  selected = new Set();
let settings = { ...defaults },
  busy = 0,
  saveChain = Promise.resolve(),
  drag = null,
  renderCache = null,
  batchTracks = [];
let player = null,
  playGeneration = 0,
  gapTimer = null,
  countdown = null,
  toastTimer = null;
const audio = $("audio");
audio.volume = 0.8;
try {
  settings = {
    ...defaults,
    ...JSON.parse(localStorage.getItem("cs-settings") || "{}"),
  };
} catch {}
const track = (id) => lib.tracks.find((t) => t.id === id);
const current = () => lib.collections.find((c) => c.id === active);
const config = () => ({ ...defaults, ...(current()?.settings || settings) });
