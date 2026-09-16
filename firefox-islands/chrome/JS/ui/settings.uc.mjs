/* islands — quick settings panel.
 *
 * A CustomizableUI toolbar button (movable in Customize mode) that opens a
 * native panel with the most-used controls, plus a keyboard shortcut and the
 * full settings page opener.
 */

import { STRINGS } from "chrome://userchrome/content/strings/es-MX.js";

const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);
const { createElement } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/dom.sys.mjs"
);

const s = key => STRINGS[key] || key;
const WIDGET_ID = "islands-settings-button";
const PANEL_ID = "uc-islands-quick-panel";
const SETTINGS_URL = "chrome://userchrome/content/settings.xhtml";
const QUICK_KEYS = ["gap", "window-radius", "bar-height"];

class QuickSettings {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this._panel = null;
    this._controls = {};
  }

  start() {
    this._registerWidget();
    this._registerShortcut();
  }

  _registerWidget() {
    const { CustomizableUI } = this.win;
    if (!CustomizableUI || CustomizableUI.getWidget(WIDGET_ID)) return;

    CustomizableUI.createWidget({
      id: WIDGET_ID,
      type: "button",
      label: "islands",
      tooltiptext: s("quick.open-settings"),
      defaultArea: CustomizableUI.AREA_NAVBAR,
      onCommand: event => this._togglePanel(event.target),
    });
  }

  _registerShortcut() {
    const { Hotkeys } = ChromeUtils.importESModule(
      "chrome://userchromejs/content/uc_api.sys.mjs"
    );
    Hotkeys.define({
      id: "islands-open-settings",
      modifiers: "ctrl shift",
      key: "U",
      command: win => this._openSettings(win),
    }).autoAttach({ suppressOriginal: false });
  }

  _openSettings(win) {
    // chrome:// pages need a privileged window: they cannot load in a
    // content tab, so the full settings open as a chrome dialog window.
    const w = win || this.win;
    if (w.closed) return;
    w.openDialog(SETTINGS_URL, "islands-settings", "chrome,resizable,centerscreen");
  }

  _togglePanel(anchor) {
    if (!this._panel) this._buildPanel();
    if (this._panel.state === "open") {
      this._panel.hidePopup();
    } else {
      this._panel.openPopup(anchor, "bottomleft topleft");
    }
  }

  _buildPanel() {
    this._panel = createElement(this.doc, "panel", {
      id: PANEL_ID,
      type: "arrow",
      position: "bottomleft topleft",
      onpopupshown: () => this._sync(),
    });
    this.doc.documentElement.appendChild(this._panel);

    this._panel.appendChild(
      createElement(this.doc, "label", {
        class: "uc-qp-title",
        textContent: s("quick.title"),
      })
    );

    this._controls.variant = this._addSelect("variant", ["islands", "notch"]);
    for (const key of QUICK_KEYS) this._controls[key] = this._addRange(key);
    this._controls.animations = this._addCheckbox("motion.enabled");

    const actions = createElement(this.doc, "hbox", {
      class: "uc-qp-actions",
    });
    const openBtn = createElement(this.doc, "button", {
      textContent: s("quick.open-settings"),
    });
    openBtn.addEventListener("command", () => {
      this._panel.hidePopup();
      this._openSettings();
    });
    actions.appendChild(openBtn);
    this._panel.appendChild(actions);
  }

  _row(labelKey, control) {
    const row = createElement(this.doc, "hbox", { class: "uc-qp-row" });
    row.appendChild(createElement(this.doc, "label", { textContent: s(labelKey) }));
    row.appendChild(control);
    this._panel.appendChild(row);
    return row;
  }

  _addSelect(key, options) {
    const select = createElement(this.doc, "menulist");
    const popup = createElement(this.doc, "menupopup");
    for (const opt of options) {
      popup.appendChild(
        createElement(this.doc, "menuitem", {
          value: opt,
          label: s(`opt.${key}.${opt}`),
        })
      );
    }
    select.appendChild(popup);
    select.value = PrefsStore.get(key);
    select.addEventListener("command", () => {
      if (select.value) PrefsStore.set(key, select.value);
    });
    this._row(`setting.layout.${key}`, select);
    return select;
  }

  _addRange(key) {
    const def = PrefsStore.schema.find(x => x.key === key);
    const scale = createElement(this.doc, "scale", {
      min: def.min,
      max: def.max,
      increment: def.step,
      value: def.default,
      flex: "1",
    });
    scale.addEventListener("command", () => {
      PrefsStore.set(key, scale.value);
    });
    const row = createElement(this.doc, "hbox", { class: "uc-qp-row" });
    row.appendChild(createElement(this.doc, "label", { textContent: s(def.stringKey) }));
    row.appendChild(scale);
    this._panel.appendChild(row);
    return scale;
  }

  _addCheckbox(key) {
    const cb = createElement(this.doc, "checkbox", {
      checked: PrefsStore.get(key),
      label: s("quick.animations"),
    });
    cb.addEventListener("command", () => PrefsStore.set(key, cb.checked));
    const row = createElement(this.doc, "hbox", { class: "uc-qp-row" });
    row.appendChild(cb);
    this._panel.appendChild(row);
    return cb;
  }

  _sync() {
    try {
      if (this._controls.variant) {
        this._controls.variant.value = PrefsStore.get("variant");
      }
      for (const key of QUICK_KEYS) {
        const scale = this._controls[key];
        if (scale) scale.value = PrefsStore.get(key);
      }
      if (this._controls.animations) {
        this._controls.animations.checked = PrefsStore.get("motion.enabled");
      }
    } catch (e) {}
  }
}

try {
  if (window.top === window && !window.closed) {
    const qs = new QuickSettings(window);
    qs.start();
    window.addEventListener("unload", () => qs._panel?.remove(), { once: true });
  }
} catch (e) {
  console.error("uc.islands quick settings failed", e);
}