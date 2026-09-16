/* islands — settings page logic.
 *
 * Renders every group/control from the schema, applies changes live through
 * the prefs store, and provides the presets and diagnostics sections. The
 * page runs in the same privileged process as the loader, so the shared
 * singletons are the same instances the windows use.
 */

const { SCHEMA, SCHEMA_BY_KEY } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/schema.sys.mjs"
);
const { PrefsStore } = ChromeUtils.importESModule(
  "chrome://userscripts/content/core/prefs-store.sys.mjs"
);
const { SelectorHealth } = ChromeUtils.importESModule(
  "chrome://userscripts/content/libs/dom.sys.mjs"
);
const { STRINGS } = ChromeUtils.importESModule(
  "chrome://userchrome/content/strings/es-MX.js"
);

const GROUPS = [
  "system",
  "layout",
  "frame",
  "urlbar",
  "tabs",
  "motion",
  "dynamicNotch",
  "colors",
  "behavior",
  "presets",
  "diagnostics",
];

const s = key => STRINGS[key] || key;

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "text") node.textContent = v;
    else if (k === "class") node.className = v;
    else if (k.startsWith("on") && typeof v === "function") {
      node.addEventListener(k.slice(2).toLowerCase(), v);
    } else node.setAttribute(k, v);
  }
  for (const child of children) node.appendChild(child);
  return node;
}

function optionValue(key) {
  const map = {
    variant: ["islands", "notch"],
    "tabs-layout": ["horizontal", "vertical"],
    "colors.source": ["theme", "manual", "file"],
  };
  return map[key] || [];
}

/** Build the control for one schema entry; returns {row, setValue, value}. */
function buildControl(def, apply) {
  let control;
  let read;
  let set;
  let rowExtra = null;

  if (def.type === "boolean") {
    const cb = el("input", { type: "checkbox", id: "uc-" + def.key.replace(/\./g, "-") });
    cb.addEventListener("change", () => apply(cb.checked));
    control = cb;
    read = () => cb.checked;
    set = v => (cb.checked = !!v);
  } else if (def.type === "number") {
    const range = el("input", {
      type: "range",
      min: def.min,
      max: def.max,
      step: def.step ?? 1,
      id: "uc-" + def.key.replace(/\./g, "-"),
    });
    const valueLabel = el("span", { class: "uc-value" });
    const box = el("div", { class: "uc-control" }, [range, valueLabel]);
    const updateLabel = () => {
      valueLabel.textContent =
        def.key === "frame.radius-override" && !range.value
          ? "auto"
          : String(range.value);
    };
    range.addEventListener("input", () => {
      updateLabel();
      apply(Number(range.value));
    });
    control = box;
    read = () => (range.value === "" ? null : Number(range.value));
    set = v => {
      range.value = v === null || v === undefined ? "" : String(v);
      updateLabel();
    };
  } else {
    // string: select for enums, text input otherwise
    const opts = optionValue(def.key);
    if (opts.length) {
      const select = el("select", { id: "uc-" + def.key.replace(/\./g, "-") });
      for (const o of opts) {
        select.appendChild(el("option", { value: o, text: s(`opt.${def.key}.${o}`) }));
      }
      select.addEventListener("change", () => apply(select.value));
      control = select;
      read = () => select.value;
      set = v => (select.value = v);
    } else {
      const input = el("input", {
        type: "text",
        id: "uc-" + def.key.replace(/\./g, "-"),
        placeholder: def.key === "colors.palette-file" ? "/ruta/a/paleta.json" : "",
      });
      input.addEventListener("change", () => apply(input.value));
      control = input;
      read = () => input.value;
      set = v => (input.value = v ?? "");
    }
  }

  return { def, control, read, set };
}

function dependsOn(def, values) {
  if (!def.depends) return true;
  const current = values[def.depends.key];
  return current === def.depends.equals;
}

class SettingsPage {
  constructor() {
    this._controls = [];
    this._values = {};
    this._rows = new Map();
  }

  init() {
    for (const def of SCHEMA) this._values[def.key] = PrefsStore.get(def.key);

    const host = document.getElementById("uc-groups");
    for (const group of GROUPS) {
      host.appendChild(this._buildGroup(group));
    }
    this._updateVisibility();
  }

  _buildGroup(group) {
    const section = el("section", { class: "uc-group", "data-group": group });
    section.appendChild(el("h2", { text: s(`group.${group}`) }));

    if (group === "presets") {
      section.appendChild(this._buildPresets());
      return section;
    }
    if (group === "diagnostics") {
      section.appendChild(this._buildDiagnostics());
      return section;
    }

    for (const def of SCHEMA) {
      if (def.group !== group) continue;
      const c = buildControl(def, value => this._apply(def.key, value));
      this._controls.push(c);
      this._rows.set(def.key, c);

      const hint = el("span", { class: "hint", text: def.doc || "" });
      const label = el("label", { for: c.control.id }, [
        el("span", { text: s(def.stringKey) }),
        hint,
      ]);
      const row = el("div", { class: "uc-row", "data-key": def.key }, [label, c.control]);
      section.appendChild(row);
    }
    return section;
  }

  _apply(key, value) {
    const stored = PrefsStore.set(key, value);
    if (stored !== null) this._values[key] = stored;
    this._updateVisibility();
  }

  _updateVisibility() {
    for (const c of this._controls) {
      const visible = dependsOn(c.def, this._values);
      const row = document.querySelector(`.uc-row[data-key="${c.def.key}"]`);
      if (row) row.hidden = !visible;
    }
  }

  // ---- presets ------------------------------------------------------------

  _buildPresets() {
    const box = el("div");
    const buttons = el("div", { class: "uc-buttons" });
    buttons.appendChild(el("button", { text: s("settings.presets.save"), onclick: () => this._savePreset() }));
    buttons.appendChild(el("button", { text: s("settings.presets.load"), onclick: () => this._loadPreset() }));
    buttons.appendChild(el("button", { text: s("settings.presets.export"), onclick: () => this._exportPreset() }));
    buttons.appendChild(el("button", { text: s("settings.presets.import"), onclick: () => this._importPreset() }));
    buttons.appendChild(el("button", { "data-kind": "danger", text: s("settings.presets.reset"), onclick: () => this._resetAll() }));
    const status = el("p", { class: "hint", id: "uc-preset-status" });
    box.appendChild(buttons);
    box.appendChild(status);
    return box;
  }

  _status(text) {
    const node = document.getElementById("uc-preset-status");
    if (node) node.textContent = text;
  }

  _savePreset() {
    PrefsStore.set("presets.current", JSON.stringify(PrefsStore.getAll()));
    this._status(s("settings.presets.saved"));
  }

  _loadPreset() {
    const raw = PrefsStore.get("presets.current");
    if (!raw) return;
    this._applySnapshot(JSON.parse(raw));
    this._status(s("settings.presets.loaded"));
  }

  async _exportPreset() {
    const picker = Cc["@mozilla.org/filepicker;1"].createInstance(Ci.nsIFilePicker);
    picker.init(window, s("settings.presets.export"), picker.modeSave);
    picker.defaultString = "islands-preset.json";
    if (picker.show() !== picker.returnOK) return;
    await IOUtils.writeUTF8(picker.file.path, JSON.stringify(PrefsStore.getAll(), null, 2));
    this._status(s("settings.presets.exported"));
  }

  async _importPreset() {
    const picker = Cc["@mozilla.org/filepicker;1"].createInstance(Ci.nsIFilePicker);
    picker.init(window, s("settings.presets.import"), picker.modeOpen);
    if (picker.show() !== picker.returnOK) return;
    try {
      const snapshot = JSON.parse(await IOUtils.readUTF8(picker.file.path));
      this._applySnapshot(snapshot);
      this._status(s("settings.presets.imported"));
    } catch (e) {
      this._status(s("settings.presets.invalid"));
    }
  }

  _applySnapshot(snapshot) {
    let applied = 0;
    for (const [key, value] of Object.entries(snapshot)) {
      if (!SCHEMA_BY_KEY[key] || key === "presets.current") continue;
      if (PrefsStore.set(key, value) !== null) applied++;
    }
    if (applied) this._updateValues();
  }

  _resetAll() {
    if (!confirm(s("settings.presets.confirm-reset"))) return;
    PrefsStore.resetAll();
    this._updateValues();
    this._status(s("settings.presets.reset-done"));
  }

  _updateValues() {
    for (const c of this._controls) {
      this._values[c.def.key] = PrefsStore.get(c.def.key);
      c.set(this._values[c.def.key]);
    }
    this._updateVisibility();
  }

  // ---- diagnostics ---------------------------------------------------------

  _buildDiagnostics() {
    const box = el("div", { class: "uc-diagnostics" });
    const lines = [];

    const version = Services.appinfo.version;
    const nova = Services.prefs.getBoolPref("browser.nova.enabled", false);
    let loaderVersion = "desconocido";
    try {
      const { UC_API } = ChromeUtils.importESModule(
        "chrome://userchromejs/content/uc_api.sys.mjs"
      );
      loaderVersion = UC_API.Runtime?.loaderVersion || loaderVersion;
    } catch (e) {}

    lines.push(`${s("settings.diagnostics.version")}: ${version}`);
    lines.push(`${s("settings.diagnostics.nova")}: ${nova ? "sí" : "no"}`);
    lines.push(`${s("settings.diagnostics.loader")}: ${loaderVersion}`);

    const report = SelectorHealth.getReport();
    if (report) {
      const missing = [...(report.missing || []), ...(report.novaMissing || [])];
      if (missing.length) {
        lines.push(`${s("settings.diagnostics.selector-health")}: ${s("settings.diagnostics.missing")} ${missing.join(", ")}`);
      } else {
        lines.push(`${s("settings.diagnostics.selector-health")}: ${s("settings.diagnostics.ok")}`);
      }
    }
    box.appendChild(el("pre", { text: lines.join("\n") }));
    return box;
  }
}

try {
  const page = new SettingsPage();
  page.init();
} catch (e) {
  console.error("uc.islands settings page failed", e);
  const groups = document.getElementById("uc-groups");
  if (groups) groups.textContent = "islands: " + e;
}