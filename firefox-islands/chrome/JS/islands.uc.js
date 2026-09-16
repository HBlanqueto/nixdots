// ==UserScript==
// @name           islands
// @description    per-window entry for the islands layout
// @loadOrder      1
// ==/UserScript==

/* Loads the per-window controller module. Runs in every browser.xhtml
   document; skipped for popup, taskbar-tab and chromeless windows, and when
   the global kill switch is off. The controller module imports the shared
   singletons through ChromeUtils.importESModule so every window talks to the
   same prefs store, event bus and theme source. */

(async function () {
  const root = document.documentElement;
  if (window.top !== window) return;
  if (
    root.hasAttribute("popup-window") ||
    root.hasAttribute("taskbartab") ||
    root.hasAttribute("chromeless-window") ||
    root.hasAttribute("web-extension-popup-window")
  ) {
    return;
  }
  if (!Services.prefs.getBoolPref("uc.islands.enabled", true)) return;

  const modules = [
    "ui/islands.uc.mjs",
    "ui/motion.uc.mjs",
    "ui/urlbar.uc.mjs",
    "ui/notch.uc.mjs",
    "ui/safari-merge.uc.mjs",
    "ui/settings.uc.mjs",
  ];
  for (const mod of modules) {
    try {
      await import("chrome://userscripts/content/" + mod);
    } catch (e) {
      console.error("uc.islands module failed to load:", mod, e);
    }
  }
})();