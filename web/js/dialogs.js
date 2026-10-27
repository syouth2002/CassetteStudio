"use strict";
// Notifications, busy-state UI and reusable name/confirmation dialogs.
function placeToast() {
  const dialogs = [...document.querySelectorAll("dialog[open]")];
  (dialogs.at(-1) || document.body).appendChild($("toast"));
}
function toast(msg) {
  clearTimeout(toastTimer);
  placeToast();
  $("toast").textContent = csTranslateMessage(msg);
  $("toast").hidden = false;
  toastTimer = setTimeout(() => ($("toast").hidden = true), 8000);
}
async function task(message, fn) {
  busy++;
  $("busy-label").textContent = message;
  $("busy").hidden = false;
  document
    .querySelectorAll(
      ".toolbar button,#tone-open,#analyze-volume,#export-btn,#record-btn,#save-settings,#reanalyze-btn",
    )
    .forEach((b) => (b.disabled = true));
  try {
    return await fn();
  } catch (e) {
    toast(e.message);
    return null;
  } finally {
    busy--;
    if (!busy) {
      $("busy").hidden = true;
      document
        .querySelectorAll(
          ".toolbar button,#tone-open,#analyze-volume,#export-btn,#record-btn,#save-settings,#reanalyze-btn",
        )
        .forEach((b) => (b.disabled = false));
    }
  }
}
window.askText = (title, value = "") =>
  new Promise((resolve) => {
    const dialog = document.getElementById("name-dialog"),
      input = document.getElementById("name-value");
    document.getElementById("name-prompt").textContent = title;
    input.value = value;
    dialog.returnValue = "";
    dialog.addEventListener(
      "close",
      () => resolve(dialog.returnValue === "ok" ? input.value : null),
      { once: true },
    );
    dialog.showModal();
    input.focus();
    input.select();
  });
window.askConfirm = (message) =>
  new Promise((resolve) => {
    const dialog = document.getElementById("confirm-dialog");
    dialog.returnValue = "";
    document.getElementById("confirm-message").textContent = message;
    dialog.addEventListener(
      "close",
      () => resolve(dialog.returnValue === "ok"),
      { once: true },
    );
    dialog.showModal();
  });
