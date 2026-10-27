"use strict";
// Cross-dialog notification placement and unhandled-error binding.
function bindCommonControls() {
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
