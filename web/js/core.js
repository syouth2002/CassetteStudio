"use strict";
// DOM access, escaping and presentation-only formatting.
const $ = (id) => document.getElementById(id);
const esc = (s) =>
  String(s ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const time = (n) => {
  n = Math.max(0, Math.round(n || 0));
  return `${Math.floor(n / 60)}:${String(n % 60).padStart(2, "0")}`;
};
const duration = (t) => Math.max(0, t.trim_end - t.trim_start);
const db = (n) => `${n >= 0 ? "+" : ""}${n.toFixed(1)} dB`;
