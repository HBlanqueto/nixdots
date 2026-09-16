/* islands — DOM helpers and selector health.
 *
 * Small element factory plus a selector health check that reports missing
 * chrome elements instead of failing silently: findings go to the Browser
 * Console at startup and are stored for the Diagnostics section of the
 * settings page.
 */

const REQUIRED_SELECTORS = Object.freeze({
  toolbox: "#navigator-toolbox",
  menubar: "#toolbar-menubar",
  tabStrip: "#TabsToolbar",
  tabStripTarget: "#TabsToolbar-customization-target",
  tabs: "#tabbrowser-tabs",
  tabScroll: "#tabbrowser-arrowscrollbox",
  navBar: "#nav-bar",
  navBarTarget: "#nav-bar-customization-target",
  back: "#back-button",
  forward: "#forward-button",
  stopReload: "#stop-reload-button",
  urlbarContainer: "#urlbar-container",
  urlbar: "#urlbar",
  urlbarInput: "#urlbar-input",
  identityBox: "#identity-box",
  pageActions: "#page-action-buttons",
  unifiedExtensions: "#unified-extensions-button",
  panelUI: "#PanelUI-button",
  browser: "#browser",
  tabbox: "#tabbrowser-tabbox",
  tabpanels: "#tabbrowser-tabpanels",
  browserContainer: ".browserContainer",
});

/** Selectors that only exist in the Nova-era DOM (Firefox 157+). */
const NOVA_SELECTORS = Object.freeze({
  notificationsToolbar: "#notifications-toolbar",
  verticalTabs: "#vertical-tabs",
  smartwindowAsk: "#smartwindow-ask-button",
  downloadsButton: "#downloads-button",
  taskbarTabsFavicon: "#taskbar-tabs-favicon",
});

export function createElement(doc, tag, attrs = {}, isHTML = false) {
  const el = isHTML ? doc.createElement(tag) : doc.createXULElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") el.className = v;
    else if (k === "textContent") el.textContent = v;
    else if (k === "style") el.setAttribute("style", v);
    else el.setAttribute(k, v);
  }
  return el;
}

class SelectorHealthImpl {
  constructor() {
    this._last = null;
  }

  /** Check a window document and report. Returns the report object. */
  check(win) {
    const doc = win.document;
    const missing = [];
    for (const [name, sel] of Object.entries(REQUIRED_SELECTORS)) {
      if (!doc.querySelector(sel)) missing.push(sel);
    }
    const novaMissing = [];
    for (const [name, sel] of Object.entries(NOVA_SELECTORS)) {
      if (!doc.querySelector(sel)) novaMissing.push(sel);
    }
    const report = {
      version: Services.appinfo.version,
      nova: Services.prefs.getBoolPref("browser.nova.enabled", false),
      missing,
      novaMissing,
      checkedAt: Date.now(),
    };
    this._last = report;
    if (missing.length) {
      console.warn(
        "uc.islands: missing chrome selectors — the design may look degraded:",
        missing.join(", ")
      );
    }
    return report;
  }

  getReport() {
    return this._last;
  }
}

const selectorHealth = new SelectorHealthImpl();
export { selectorHealth as SelectorHealth };