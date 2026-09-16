/* islands — per-window layout controller.
 *
 * One instance per browser window. Applies the settings to the window's
 * root attributes and CSS custom properties, marks the island groups,
 * measures the tabs region, applies the theme palette and cleans up all of
 * it on unload. Kept intentionally small so a failing module can not take
 * the layout down.
 *
 * Note: as a window-scoped module, static imports would produce window-local
 * instances of the shared modules, so every singleton is fetched through
 * ChromeUtils.importESModule (same instances as bootstrap.sys.mjs).
 */

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);
const { EventBus } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/event-bus.sys.mjs"
);
const { ThemeSource } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/theme-source.sys.mjs"
);
const { WindowRegistry } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/window-registry.sys.mjs"
);
const { SelectorHealth } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/dom.sys.mjs"
);
const { Motion } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/motion.sys.mjs"
);

const BLUR_PX = "18px";

export class IslandsController {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.root = win.document.documentElement;
    this._unsubs = [];
    this._obs = null;
    this._mutationObserver = null;
    this._resizeObserver = null;
    this._started = false;
    this._lastTabsMeasure = { left: null, right: null };
  }

  start() {
    if (this._started) return;
    if (!PrefsStore.get("enabled")) return;
    this._started = true;

    this._unregister = WindowRegistry.register(this.win);
    ThemeSource.setProbeWindow(this.win);

    SelectorHealth.check(this.win);

    this._unsubs.push(PrefsStore.observe((key, value) => this._applySetting(key, value)));
    this._unsubs.push(EventBus.on("theme", ({ palette }) => this._applyTheme(palette)));

    this._markGroups();
    this._observeChrome();
    this._applyAll();

    this.win.addEventListener("resize", this._onResize, { passive: true });
    this.win.addEventListener("focus", this._onFocus, true);
    this.win.addEventListener("unload", this._onUnload, { once: true });
    this.root.addEventListener("uc:remeasure", this._onReMeasure);
  }

  _applyAll() {
    for (const def of PrefsStore.schema) {
      if (def.key === "presets.current") continue;
      this._applySetting(def.key, PrefsStore.get(def.key));
    }
    ThemeSource.apply();
  }

  _applySetting(key, value) {
    const def = PrefsStore.schema.find(s => s.key === key);
    if (!def) return;

    // Global kill switch: disable cleanly and hand the window back to the
    // CSS-only islands layout. Re-enabling takes effect on new windows.
    if (key === "enabled") {
      if (value) {
        this._applyAll();
      } else {
        this._resetWindowState();
        this.stop();
      }
      return;
    }

    // Colors are owned by the theme source; only the source attribute is
    // mirrored directly here.
    if (key.startsWith("colors.")) {
      if (key === "colors.source") this._mirrorAttr(def, value);
      ThemeSource.apply();
      return;
    }

    // Motion is combined: motion.enabled AND respect-reduced-motion.
    if (key === "motion.enabled" || key === "motion.respect-reduced-motion") {
      this._applyMotion();
      return;
    }
    if (key === "motion.speed") {
      this.root.style.setProperty("--uc-motion-speed", String(value));
      return;
    }
    if (key === "motion.spring-stiffness" || key === "motion.spring-damping") {
      this._applySpring();
      return;
    }

    // Native vertical tabs mirroring (Firefox's own pref stays in charge).
    if (key === "tabs-layout") {
      this._mirrorVerticalTabs(value);
      return;
    }

    if (key === "colors.blur") {
      this.root.style.setProperty("--uc-blur-amount", value ? BLUR_PX : "0px");
      this.root.toggleAttribute("uc-blur", !!value);
      return;
    }

    if (key === "variant") {
      EventBus.emit("layout:variant-before", { variant: value });
    }

    this._mirrorAttr(def, value);
    this._mirrorVar(def, value);

    if (key === "variant") {
      this._scheduleLayout();
      EventBus.emit("layout:variant-after", { variant: value });
    }

    // The layout depends on some of these; keep the tabs region honest.
    if (
      key === "gap" ||
      key === "bar-height" ||
      key === "notch-width" ||
      key === "tabs-layout"
    ) {
      this._scheduleLayout();
    }
    if (key === "tabs.favicon-only-threshold") {
      this._updateFaviconMode();
    }
  }

  _mirrorAttr(def, value) {
    if (!def.attr) return;
    if (def.attrWhen) {
      // Positive-off: the attribute with this value means "disabled".
      if (value === false || value === def.attrWhen) {
        this.root.setAttribute(def.attr, def.attrWhen);
      } else {
        this.root.removeAttribute(def.attr);
      }
      return;
    }
    if (def.type === "boolean") {
      this.root.toggleAttribute(def.attr, !!value);
    } else if (value) {
      this.root.setAttribute(def.attr, String(value));
    } else {
      this.root.removeAttribute(def.attr);
    }
  }

  _mirrorVar(def, value) {
    if (!def.cssVar) return;
    if (value === null || value === undefined || value === "") {
      this.root.style.removeProperty(def.cssVar);
    } else {
      this.root.style.setProperty(def.cssVar, def.cssUnit ? value + def.cssUnit : String(value));
    }
  }

  _applyMotion() {
    const enabled = PrefsStore.get("motion.enabled");
    const respect = PrefsStore.get("motion.respect-reduced-motion");
    const reduced = Motion.prefersReducedMotion(this.win);
    this.root.toggleAttribute("uc-motion", enabled && !(respect && reduced));
    this.root.style.setProperty("--uc-motion-speed", String(PrefsStore.get("motion.speed")));
    this._applySpring();
  }

  _applySpring() {
    const easing = Motion.springLinear(
      PrefsStore.get("motion.spring-stiffness"),
      PrefsStore.get("motion.spring-damping")
    );
    this.root.style.setProperty("--uc-spring", easing);
  }

  _applyTheme(palette) {
    const map = {
      canvasBase: "--uc-canvas-base",
      islandBg: "--uc-island-bg",
      islandBgHover: "--uc-island-bg-hover",
      fg: "--uc-fg",
      fgMuted: "--uc-fg-muted",
      accent: "--uc-accent",
      frameBorder: "--uc-frame-border",
      privateTint: "--uc-private-tint",
    };
    for (const [prop, cssVar] of Object.entries(map)) {
      const v = palette[prop];
      if (v) this.root.style.setProperty(cssVar, v);
      else this.root.style.removeProperty(cssVar);
    }
  }

  _mirrorVerticalTabs(value) {
    const vertical = value === "vertical";
    if (Services.prefs.getBoolPref("sidebar.verticalTabs", false) !== vertical) {
      Services.prefs.setBoolPref("sidebar.verticalTabs", vertical);
    }
    this._scheduleLayout();
  }

  /** Sync our pref back when Firefox's own vertical-tabs pref changes. */
  _observeVerticalTabsPref() {
    if (this._obs) return;
    this._obs = {
      observe() {
        const vertical = Services.prefs.getBoolPref("sidebar.verticalTabs", false);
        const want = vertical ? "vertical" : "horizontal";
        if (PrefsStore.get("tabs-layout") !== want) {
          PrefsStore.set("tabs-layout", want);
        }
      },
    };
    Services.prefs.addObserver("sidebar.verticalTabs", this._obs);
  }

  // ---- island group marking ----------------------------------------------

  /** Mark island members and their first/last edges without moving nodes. */
  _markGroups() {
    const target = this.doc.getElementById("nav-bar-customization-target");
    if (!target) return;
    let phase = "nav-start";
    let members = [];
    const flush = () => {
      members.forEach((el, i) => {
        el.setAttribute("data-uc-group", phase);
        el.removeAttribute("data-uc-edge");
        if (i === 0) el.setAttribute("data-uc-edge", "first");
        if (i === members.length - 1) el.setAttribute("data-uc-edge", "last");
      });
      members = [];
    };
    for (const el of target.children) {
      if (el.id === "urlbar-container") {
        if (members.length) flush();
        phase = "nav-end";
        continue;
      }
      if (
        el.tagName === "toolbarspring" ||
        el.tagName === "toolbartabstop" ||
        (el.classList && el.classList.contains("titlebar-spacer"))
      ) {
        el.removeAttribute("data-uc-group");
        el.removeAttribute("data-uc-edge");
        continue;
      }
      members.push(el);
    }
    flush();
  }

  // ---- tabs region measurement -------------------------------------------

  /** Recompute the tabs island placement, batching reads before writes. */
  measure() {
    const urlbar = this.doc.getElementById("urlbar-container");
    const panelUI = this.doc.getElementById("PanelUI-button");
    if (!urlbar) return;

    // Do not re-measure while the urlbar is focused: spotlight expands it
    // with a transform and the cached band stays stable underneath.
    if (urlbar.matches("#urlbar-container:has(#urlbar[focused])")) {
      this._restoreTabsMeasure();
      return;
    }

    const gap = parseFloat(
      getComputedStyle(this.root).getPropertyValue("--uc-gap")
    ) || 8;

    const urlRect = urlbar.getBoundingClientRect();

    // Anchor the region's right edge on the first visible member of the
    // nav-end island (downloads/extensions), falling back to the menu button
    // when the whole right island overflowed away.
    let rightEl = null;
    for (const el of this.doc.querySelectorAll(
      '#nav-bar-customization-target > [data-uc-group="nav-end"]'
    )) {
      if (el.getClientRects().length > 0) {
        rightEl = el;
        break;
      }
    }
    if (!rightEl && panelUI && panelUI.getClientRects().length > 0) {
      rightEl = panelUI;
    }
    if (!rightEl) {
      this._restoreTabsMeasure();
      return;
    }
    const rightRect = rightEl.getBoundingClientRect();

    const left = Math.round(urlRect.right + gap);
    const rightInset = Math.round(this.win.innerWidth - rightRect.left + gap);
    this._lastTabsMeasure = { left, right: rightInset };
    this.root.style.setProperty("--uc-tabs-inline-start", left + "px");
    this.root.style.setProperty("--uc-tabs-inline-end", rightInset + "px");
  }

  _restoreTabsMeasure() {
    const { left, right } = this._lastTabsMeasure;
    if (left !== null) {
      this.root.style.setProperty("--uc-tabs-inline-start", left + "px");
      this.root.style.setProperty("--uc-tabs-inline-end", right + "px");
    }
  }

  _scheduleLayout() {
    if (this._layoutPending) return;
    this._layoutPending = true;
    this.win.requestAnimationFrame(() => {
      this._layoutPending = false;
      this.measure();
    });
  }

  // ---- chrome change observation -----------------------------------------

  _observeChrome() {
    const target = this.doc.getElementById("nav-bar-customization-target");
    if (target && "MutationObserver" in this.win) {
      this._mutationObserver = new this.win.MutationObserver(() =>
        this._scheduleLayout()
      );
      this._mutationObserver.observe(target, { childList: true, subtree: false });
    }
    if ("ResizeObserver" in this.win) {
      this._resizeObserver = new this.win.ResizeObserver(() => {
        this._scheduleLayout();
        this._updateFaviconMode();
      });
      const urlbar = this.doc.getElementById("urlbar-container");
      if (urlbar) this._resizeObserver.observe(urlbar);
      const tabs = this.doc.getElementById("tabbrowser-tabs");
      if (tabs) this._resizeObserver.observe(tabs);
    }
    this._observeVerticalTabsPref();
  }

  /** Toggle favicon-only mode when the tab strip is overflowing and the
      average tab is narrower than the configured threshold. */
  _updateFaviconMode() {
    const tabs = this.doc.getElementById("tabbrowser-tabs");
    if (!tabs) return;
    const scrollbox = this.doc.getElementById("tabbrowser-arrowscrollbox");
    const overflowing = scrollbox?.getAttribute("overflowing") === "true";
    const count = this.win.gBrowser?.visibleTabs?.length || 0;
    const width = tabs.getBoundingClientRect().width;
    const avg = count ? width / count : 9999;
    const threshold = PrefsStore.get("tabs.favicon-only-threshold");
    tabs.toggleAttribute("uc-favicon-only", overflowing && avg < threshold);
  }

  _onResize = () => this._scheduleLayout();

  _onReMeasure = () => this._scheduleLayout();

  _onFocus = () => ThemeSource.setProbeWindow(this.win);

  _onUnload = () => this.stop();

  stop() {
    if (!this._started) return;
    this._started = false;
    for (const unsub of this._unsubs) {
      try {
        unsub();
      } catch (e) {}
    }
    this._unsubs = [];
    if (this._obs) {
      Services.prefs.removeObserver("sidebar.verticalTabs", this._obs);
      this._obs = null;
    }
    if (this._mutationObserver) this._mutationObserver.disconnect();
    if (this._resizeObserver) this._resizeObserver.disconnect();
    this.win.removeEventListener("resize", this._onResize);
    this.win.removeEventListener("focus", this._onFocus, true);
    this.win.removeEventListener("unload", this._onUnload);
    this.root.removeEventListener("uc:remeasure", this._onReMeasure);
    if (this._unregister) {
      this._unregister();
      this._unregister = null;
    }
  }

  /** Remove everything the JS layer applied so the CSS-only layout returns. */
  _resetWindowState() {
    for (const attr of [...this.root.attributes]) {
      if (attr.name.startsWith("uc-")) this.root.removeAttribute(attr.name);
    }
    for (const prop of [...this.root.style]) {
      if (prop.startsWith("--uc-")) this.root.style.removeProperty(prop);
    }
    for (const el of this.doc.querySelectorAll("[data-uc-group]")) {
      el.removeAttribute("data-uc-group");
      el.removeAttribute("data-uc-edge");
    }
  }
}

// Create one controller per top-level browser window. Failures here must not
// prevent the CSS-only layout from working.
try {
  if (window.top === window && !window.closed) {
    const ctrl = new IslandsController(window);
    ctrl.start();
  }
} catch (e) {
  console.error("uc.islands controller failed to start", e);
}