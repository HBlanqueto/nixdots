/* islands — per-window URL bar behaviors.
 *
 * Host-only display: overlays a pointer-events-none label with just the
 * host over the real input while the bar is idle, so selection/copy/drag and
 * the security indicators keep working. Spotlight mode is CSS-driven (the
 * container expands into a fixed centered panel on focus) and needs no
 * JavaScript here.
 */

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);

class UrlbarController {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this._label = null;
    this._progressListener = null;
    this._unsubs = [];
  }

  start() {
    if (!PrefsStore.get("urlbar.host-only")) {
      this._unsubs.push(
        PrefsStore.observe((key, value) => {
          if (key === "urlbar.host-only" && value) this._build();
        })
      );
      return;
    }
    this._build();
  }

  _build() {
    if (this._label) return;
    const urlbar = this.doc.getElementById("urlbar");
    const container = this.doc.getElementById("urlbar-container");
    if (!urlbar || !container) return;

    this._label = this.doc.createElement("label");
    this._label.id = "uc-host-label";
    container.appendChild(this._label);

    this._updateHost();
    this._progressListener = {
      onLocationChange: () => this._updateHost(),
    };
    if (this.win.gBrowser) {
      this.win.gBrowser.addTabsProgressListener(this._progressListener);
    }
    this.doc.getElementById("tabbrowser-tabs")?.addEventListener(
      "TabSelect",
      () => this._updateHost(),
      true
    );
    this._tabSelectHandler = () => this._updateHost();
    this.win.gBrowser?.tabContainer?.addEventListener(
      "TabSelect",
      this._tabSelectHandler
    );
  }

  _updateHost() {
    if (!this._label || !this.win.gBrowser) return;
    try {
      const uri = this.win.gBrowser.currentURI;
      let host = uri?.displayHost || "";
      if (!host) host = uri?.spec || "";
      this._label.textContent = host;
    } catch (e) {
      this._label.textContent = "";
    }
  }

  stop() {
    for (const unsub of this._unsubs) unsub();
    this._unsubs = [];
    if (this._progressListener && this.win.gBrowser) {
      this.win.gBrowser.removeTabsProgressListener(this._progressListener);
      this._progressListener = null;
    }
    this.win.gBrowser?.tabContainer?.removeEventListener(
      "TabSelect",
      this._tabSelectHandler
    );
    this._label?.remove();
    this._label = null;
  }
}

try {
  if (window.top === window && !window.closed) {
    const uc = new UrlbarController(window);
    uc.start();
    window.addEventListener("unload", () => uc.stop(), { once: true });
  }
} catch (e) {
  console.error("uc.islands urlbar failed", e);
}