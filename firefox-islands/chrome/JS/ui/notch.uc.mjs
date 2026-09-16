/* islands — dynamic notch (notch variant only).
 *
 * Turns the URL notch into a live status surface: page-loading progress, the
 * number of active downloads, and the selected tab's audio state with a mute
 * action. Each state is toggleable and fully event-driven — no timers run
 * while idle.
 */

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);

class DynamicNotch {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.root = win.document.documentElement;
    this._box = null;
    this._progress = null;
    this._media = null;
    this._downloads = null;
    this._progressListener = null;
    this._downloadsView = null;
    this._downloadsList = null;
    this._activeDownloads = 0;
    this._loading = false;
    this._unsubs = [];
    this._tabSelectHandler = () => this._updateMedia();
  }

  start() {
    this._unsubs.push(PrefsStore.observe((key) => {
      if (key.startsWith("notch.")) this._updateGates();
    }));
    this._build();
    this._updateGates();
  }

  _build() {
    const urlbar = this.doc.getElementById("urlbar-container");
    if (!urlbar || this._box) return;

    this._box = this.doc.createElement("div");
    this._box.id = "uc-notch";

    this._progress = this.doc.createElement("div");
    this._progress.id = "uc-notch-progress";
    this._box.appendChild(this._progress);

    this._downloads = this.doc.createElement("button");
    this._downloads.id = "uc-notch-downloads";
    this._downloads.className = "uc-notch-action";
    this._downloads.type = "button";
    this._downloads.setAttribute("tabindex", "0");
    this._box.appendChild(this._downloads);

    this._media = this.doc.createElement("button");
    this._media.id = "uc-notch-media";
    this._media.className = "uc-notch-action";
    this._media.type = "button";
    this._media.setAttribute("tabindex", "0");
    this._box.appendChild(this._media);

    urlbar.appendChild(this._box);

    this._media.addEventListener("click", () => {
      const tab = this.win.gBrowser?.selectedTab;
      if (tab && typeof tab.toggleMuteAudio === "function") tab.toggleMuteAudio();
    });

    this.win.gBrowser?.tabContainer?.addEventListener(
      "TabAttrModified",
      this._tabSelectHandler
    );
    this.win.gBrowser?.tabContainer?.addEventListener(
      "TabSelect",
      this._tabSelectHandler
    );
  }

  _updateGates() {
    if (!this._box) return;
    this.root.toggleAttribute(
      "uc-dn-loading",
      PrefsStore.get("notch.loading")
    );
    this.root.toggleAttribute(
      "uc-dn-downloads",
      PrefsStore.get("notch.downloads")
    );
    this.root.toggleAttribute(
      "uc-dn-media",
      PrefsStore.get("notch.media")
    );
    if (PrefsStore.get("notch.loading")) this._observeLoading();
    else this._stopLoading();
    if (PrefsStore.get("notch.downloads")) this._observeDownloads();
    else this._stopDownloads();
    if (PrefsStore.get("notch.media")) this._updateMedia();
    else this._box.removeAttribute("data-media");
  }

  // ---- loading progress ---------------------------------------------------

  _observeLoading() {
    if (this._progressListener) return;
    this._progressListener = {
      onStateChange: (browser, webProgress, request, stateFlags, status) => {
        const selectedBrowser = this.win.gBrowser?.selectedBrowser;
        if (browser !== selectedBrowser) return;
        const isNetwork = stateFlags & Ci.nsIWebProgressListener.STATE_IS_NETWORK;
        if (!isNetwork) return;
        if (stateFlags & Ci.nsIWebProgressListener.STATE_START) {
          this._setLoading(true);
        } else if (stateFlags & Ci.nsIWebProgressListener.STATE_STOP) {
          this._setLoading(false);
        }
      },
      onProgressChange: (browser, webProgress, request, curSelf, maxSelf, curTotal, maxTotal) => {
        if (!this._loading || !this._progress) return;
        const pct = maxTotal > 0 ? Math.round((curTotal / maxTotal) * 100) : -1;
        this._progress.style.setProperty("--uc-notch-progress", pct + "%");
        this._progress.toggleAttribute("data-known", pct >= 0);
      },
    };
    this.win.gBrowser?.addTabsProgressListener(this._progressListener);
  }

  _stopLoading() {
    if (this._progressListener) {
      this.win.gBrowser?.removeTabsProgressListener(this._progressListener);
      this._progressListener = null;
    }
    this._setLoading(false);
  }

  _setLoading(on) {
    this._loading = on;
    this._box?.toggleAttribute("data-loading", on);
    if (this._progress) {
      this._progress.style.setProperty("--uc-notch-progress", "0%");
      this._progress.removeAttribute("data-known");
    }
  }

  // ---- downloads ----------------------------------------------------------

  async _observeDownloads() {
    if (this._downloadsList) return;
    try {
      const { Downloads } = ChromeUtils.importESModule(
        "resource://gre/modules/Downloads.sys.mjs"
      );
      this._downloadsList = await Downloads.getList(Downloads.ALL);
      this._downloadsView = {
        onDownloadAdded: d => this._trackDownload(d, 1),
        onDownloadRemoved: d => this._trackDownload(d, -1),
        onDownloadChanged: d => {
          if (!d.succeeded && !d.canceled && !d.error) this._trackDownload(d, 1);
          else this._trackDownload(d, -1);
        },
      };
      this._downloadsList.addView(this._downloadsView);
      this._activeDownloads = 0;
      const items = await this._downloadsList.getAll();
      for (const d of items) {
        if (!d.succeeded && !d.canceled && !d.error) this._activeDownloads++;
      }
      this._renderDownloads();
    } catch (e) {
      console.error("uc.islands downloads view failed", e);
    }
  }

  _trackDownload(d, delta) {
    if (d.succeeded || d.canceled || d.error) return;
    this._activeDownloads = Math.max(0, this._activeDownloads + delta);
    this._renderDownloads();
  }

  _renderDownloads() {
    if (!this._box || !this._downloads) return;
    this._box.toggleAttribute("data-downloads", this._activeDownloads > 0);
    this._downloads.textContent = this._activeDownloads > 0 ? String(this._activeDownloads) : "";
  }

  _stopDownloads() {
    if (this._downloadsList && this._downloadsView) {
      this._downloadsList.removeView(this._downloadsView);
    }
    this._downloadsList = null;
    this._downloadsView = null;
    this._activeDownloads = 0;
    if (this._box) this._box.removeAttribute("data-downloads");
  }

  // ---- media ----------------------------------------------------------------

  _updateMedia() {
    if (!this._box || !this._media) return;
    const tab = this.win.gBrowser?.selectedTab;
    const playing = tab?.hasAttribute?.("soundplaying");
    const muted = tab?.hasAttribute?.("muted") || tab?.hasAttribute?.("activemedia-blocked");
    if (playing) {
      this._box.setAttribute("data-media", muted ? "muted" : "playing");
      this._media.setAttribute("aria-label", muted ? "Unmute" : "Mute");
    } else {
      this._box.removeAttribute("data-media");
    }
  }

  stop() {
    for (const unsub of this._unsubs) unsub();
    this._unsubs = [];
    this._stopLoading();
    this._stopDownloads();
    this.win.gBrowser?.tabContainer?.removeEventListener(
      "TabAttrModified",
      this._tabSelectHandler
    );
    this.win.gBrowser?.tabContainer?.removeEventListener(
      "TabSelect",
      this._tabSelectHandler
    );
    this._box?.remove();
    this._box = null;
    this._progress = null;
    this._media = null;
    this._downloads = null;
  }
}

try {
  if (window.top === window && !window.closed) {
    const dn = new DynamicNotch(window);
    dn.start();
    window.addEventListener("unload", () => dn.stop(), { once: true });
  }
} catch (e) {
  console.error("uc.islands dynamic notch failed", e);
}