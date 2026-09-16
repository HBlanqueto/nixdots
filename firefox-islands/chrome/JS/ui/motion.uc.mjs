/* islands — per-window motion.
 *
 * Drives the layout transitions that CSS alone cannot: the islands <-> notch
 * morph. It FLIPs the moving chrome (URL island, side islands, tabs island)
 * around the variant switch emitted by the controller. Everything is gated by
 * the uc-motion attribute (off under reduced motion / the motion setting),
 * animates transform/opacity only, and cancels cleanly.
 */

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);
const { EventBus } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/event-bus.sys.mjs"
);
const { Motion } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/motion.sys.mjs"
);
const { Waapi } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/waapi.sys.mjs"
);

class MotionController {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.root = win.document.documentElement;
    this._first = null;
    this._offBefore = null;
    this._offAfter = null;
  }

  start() {
    this._offBefore = EventBus.on("layout:variant-before", () => this._before());
    this._offAfter = EventBus.on("layout:variant-after", () => this._after());
  }

  _targets() {
    return [
      this.doc.getElementById("urlbar-container"),
      this.doc.getElementById("TabsToolbar-customization-target"),
      this.doc.getElementById("PanelUI-button"),
      ...this.doc.querySelectorAll(
        '#nav-bar-customization-target > [data-uc-group]'
      ),
    ].filter(el => el && el.isConnected);
  }

  _before() {
    this._first = this._targets().map(el => ({
      el,
      rect: el.getBoundingClientRect(),
    }));
  }

  _after() {
    if (!this._first) return;
    const first = this._first;
    this._first = null;
    if (!this.root.hasAttribute("uc-motion")) return;

    this.win.requestAnimationFrame(() => {
      const speed = PrefsStore.get("motion.speed");
      const easing =
        getComputedStyle(this.root).getPropertyValue("--uc-spring").trim() ||
        "ease";
      const duration = Motion.duration(260, speed);

      for (const { el, rect } of first) {
        if (!el.isConnected) continue;
        const last = el.getBoundingClientRect();
        const dx = Math.round((rect.left - last.left) * 10) / 10;
        const dy = Math.round((rect.top - last.top) * 10) / 10;
        if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
        Waapi.animate(
          el,
          [
            { transform: `translate(${dx}px, ${dy}px)` },
            { transform: "translate(0px, 0px)" },
          ],
          { duration, easing, fill: "both" }
        );
      }
    });
  }

  stop() {
    if (this._offBefore) this._offBefore();
    if (this._offAfter) this._offAfter();
    this._offBefore = null;
    this._offAfter = null;
    this._first = null;
  }
}

try {
  if (window.top === window && !window.closed) {
    const motion = new MotionController(window);
    motion.start();
  }
} catch (e) {
  console.error("uc.islands motion failed", e);
}