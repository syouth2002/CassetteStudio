"use strict";
// Cross-dialog notification placement and unhandled-error binding.
function bindCommonControls() {
  // Keep column headings directly below the controls, including wrapped translations.
  if (typeof ResizeObserver === "function") {
    const controls = $("track-controls");
    const updateHeaderOffset = () => {
      $("drop-zone").style.setProperty(
        "--track-controls-height",
        `${controls.getBoundingClientRect().height}px`,
      );
    };
    new ResizeObserver(updateHeaderOffset).observe(controls);
    updateHeaderOffset();
  }
  document.addEventListener(
    "close",
    (e) => {
      if (e.target.tagName === "DIALOG" && e.target.contains($("toast")))
        placeToast();
    },
    true,
  );
  window.addEventListener("unhandledrejection", (e) => {
    e.preventDefault();
    toast(e.reason?.message || String(e.reason));
  });
}
