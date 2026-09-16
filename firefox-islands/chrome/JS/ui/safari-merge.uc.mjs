/* islands — experimental Safari-merge.
 *
 * Visually places the address field over the active tab and keeps it in sync
 * on tab select, scroll, resize and reorder. The urlbar-container becomes an
 * absolute overlay (CSS drives the look); this module only measures the
 * selected tab and writes --uc-merge-left / --uc-merge-width. Isolated on
 * purpose: if anything here throws, the module stops and the address bar
 * returns to its normal island placement. Off by default.
 */

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);

class SafariMerge {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.root = win.document.documentElement;
    this._unsubs = [];
    this._tabSelectHandler = () => this._sync();
    this._scrollHandler = () => this._sync();
    this._resizeHandler = () => this._sync();
    this._observer = null;
    this._running = false;
    this._failed = false;
  }

  start() {
    if (!PrefsStore.get("behavior.safari-merge")) {
      this._unsubs.push(
        PrefsStore.observe((key, value) => {
          if (key === "behavior.safari-merge") {
            if (value) this._run();
            else this._teardown();
          }
        })
      );
      return;
    }
    this._run();
  }

  _run() {
    if (this._running || this._failed) return;
    if (!this.win.gBrowser) return;
    this._running = true;
    this.root.setAttribute("uc-safari-merge", "");

    const tabs = this.doc.getElementById("tabbrowser-tabs");
    const scrollbox = this.doc.getElementById("tabbrowser-arrowscrollbox");
    tabs?.addEventListener("TabSelect", this._tabSelectHandler);
    tabs?.addEventListener("TabAttrModified", this._tabSelectHandler);
    scrollbox?.addEventListener("scroll", this._scrollHandler);
    this.win.addEventListener("resize", this._resizeHandler);

    if ("MutationObserver" in this.win) {
      this._observer = new this.win.MutationObserver(() => this._sync());
      this._observer.observe(tabs, { childList: true, subtree: false });
    }

    // The tabs island now fills the whole band under the overlay.
    this.root.style.setProperty("--uc-tabs-inline-start", "0px");
    this.root.style.setProperty("--uc-tabs-inline-end", "0px");

    this._sync();
  }

  _sync() {
    if (!this._running || this._failed) return;
    try {
      const tab = this.win.gBrowser?.selectedTab;
      const background = tab?.querySelector?.(".tab-background");
      if (!tab || !background) return;
      const rect = background.getBoundingClientRect();
      const gap = 8;
      this.root.style.setProperty(
        "--uc-merge-left",
        Math.round(rect.left - gap) + "px"
      );
      this.root.style.setProperty(
        "--uc-merge-width",
        Math.max(80, Math.round(rect.width + gap * 2)) + "px"
      );
    } catch (e) {
      console.error("uc.islands safari-merge failed, disabling", e);
      this._failed = true;
      this._teardown();
    }
  }

  _teardown() {
    if (!this._running) return;
    this._running = false;
    this.doc
      .getElementById("tabbrowser-tabs")
      ?.removeEventListener("TabSelect", this._tabSelectHandler);
    this.doc
      .getElementById("tabbrowser-tabs")
      ?.removeEventListener("TabAttrModified", this._tabSelectHandler);
    this.doc
      .getElementById("tabbrowser-arrowscrollbox")
      ?.removeEventListener("scroll", this._scrollHandler);
    this.win.removeEventListener("resize", this._resizeHandler);
    if (this._observer) {
      this._observer.disconnect();
      this._observer = null;
    }
    this.root.removeAttribute("uc-safari-merge");
    this.root.style.removeProperty("--uc-merge-left");
    this.root.style.removeProperty("--uc-merge-width");
    this.root.style.removeProperty("--uc-tabs-inline-start");
    this.root.style.removeProperty("--uc-tabs-inline-end");
    if (!this._failed) {
      // Let the controller re-measure the tabs region for the normal layout.
      this.root.dispatchEvent(new CustomEvent("uc:remeasure"));
    }
  }

  stop() {
    for (const unsub of this._unsubs) unsub();
    this._unsubs = [];
    this._teardown();
  }
}

try {
  if (window.top === window && !window.closed) {
    const sm = new SafariMerge(window);
    sm.start();
    window.addEventListener("unload", () => sm.stop(), { once: true });
  }
} catch (e) {
  console.error("uc.islands safari-merge failed to start", e);
}