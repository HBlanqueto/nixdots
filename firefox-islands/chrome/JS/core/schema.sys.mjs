/* islands — declarative settings schema.
 *
 * Single source of truth for every setting. It drives:
 *   - the default-branch prefs registered at startup (prefs-store.sys.mjs),
 *   - the key -> root-attribute / CSS-custom-property mapping each window
 *     controller applies,
 *   - every control on the settings page (phase 7),
 *   - the quick panel.
 *
 * Each entry describes one pref `uc.islands.<key>`:
 *   type       "boolean" | "number" | "string"
 *   default    value when the user has not changed anything
 *   min/max/step  numeric bounds (the store clamps)
 *   enum       allowed strings (the store validates)
 *   group      settings-page group id (layout | frame | urlbar | tabs |
 *              motion | dynamicNotch | colors | behavior | system | presets)
 *   stringKey  key into resources/strings/es-MX.js for the label
 *   attr       root attribute mirrored by the controller (discrete options)
 *   attrWhen   value written to the attribute; when absent the attribute is
 *              toggled on/off instead
 *   cssVar     CSS custom property set on :root (numeric tokens and colors)
 *   cssUnit    unit appended to the cssVar value (px, etc.)
 *   depends    condition under which the setting is shown/effective, e.g.
 *              { key: "variant", equals: "notch" }
 */

export const PREF_BRANCH = "uc.islands.";

export const SCHEMA = Object.freeze([
  // ---- system ------------------------------------------------------------
  {
    key: "enabled",
    type: "boolean",
    default: true,
    group: "system",
    stringKey: "setting.system.enabled",
    doc: "Global kill switch: when false no JS behavior runs and the CSS-only islands layout is used.",
  },
  // ---- layout ------------------------------------------------------------
  {
    key: "variant",
    type: "string",
    default: "islands",
    enum: ["islands", "notch"],
    group: "layout",
    stringKey: "setting.layout.variant",
    attr: "uc-variant",
    doc: "islands: every group is a floating pill. notch: only the URL hangs from the top edge.",
  },
  {
    key: "gap",
    type: "number",
    default: 8,
    min: 0,
    max: 24,
    step: 1,
    group: "layout",
    stringKey: "setting.layout.gap",
    cssVar: "--uc-gap",
    cssUnit: "px",
  },
  {
    key: "bar-height",
    type: "number",
    default: 34,
    min: 24,
    max: 48,
    step: 1,
    group: "layout",
    stringKey: "setting.layout.bar-height",
    cssVar: "--uc-bar-height",
    cssUnit: "px",
  },
  {
    key: "window-radius",
    type: "number",
    default: 16,
    min: 0,
    max: 32,
    step: 1,
    group: "layout",
    stringKey: "setting.layout.window-radius",
    cssVar: "--uc-window-radius",
    cssUnit: "px",
  },
  {
    key: "notch-width",
    type: "number",
    default: 420,
    min: 240,
    max: 640,
    step: 10,
    group: "layout",
    stringKey: "setting.layout.notch-width",
    cssVar: "--uc-notch-width",
    cssUnit: "px",
    depends: { key: "variant", equals: "notch" },
  },
  {
    key: "tabs-layout",
    type: "string",
    default: "horizontal",
    enum: ["horizontal", "vertical"],
    group: "layout",
    stringKey: "setting.layout.tabs-layout",
    doc: "Switches the native sidebar.verticalTabs pref. Vertical tabs move the strip into the sidebar and the card frames it.",
  },
  // ---- content frame ------------------------------------------------------
  {
    key: "frame.enabled",
    type: "boolean",
    default: true,
    group: "frame",
    stringKey: "setting.frame.enabled",
    attr: "uc-frame",
    attrWhen: "off",
    doc: "Padded rounded card around the web content.",
  },
  {
    key: "frame.border",
    type: "boolean",
    default: true,
    group: "frame",
    stringKey: "setting.frame.border",
    attr: "uc-frame-border",
    attrWhen: "off",
    depends: { key: "frame.enabled", equals: true },
  },
  {
    key: "frame.radius-override",
    type: "number",
    default: null,
    nullable: true,
    sentinel: -1,
    min: 0,
    max: 32,
    step: 1,
    group: "frame",
    stringKey: "setting.frame.radius-override",
    cssVar: "--uc-frame-radius",
    cssUnit: "px",
    depends: { key: "frame.enabled", equals: true },
    doc: "null follows the derived inner radius.",
  },
  // ---- url bar ------------------------------------------------------------
  {
    key: "urlbar.host-only",
    type: "boolean",
    default: false,
    group: "urlbar",
    stringKey: "setting.urlbar.host-only",
    attr: "uc-host-only",
    doc: "Idle address bar shows only the host, centered; the full URL returns on focus.",
  },
  {
    key: "urlbar.spotlight",
    type: "boolean",
    default: false,
    group: "urlbar",
    stringKey: "setting.urlbar.spotlight",
    attr: "uc-spotlight",
    doc: "On focus the address bar expands into a centered floating panel over the page.",
  },
  {
    key: "urlbar.spotlight-width",
    type: "number",
    default: 640,
    min: 320,
    max: 1200,
    step: 20,
    group: "urlbar",
    stringKey: "setting.urlbar.spotlight-width",
    cssVar: "--uc-spotlight-width",
    cssUnit: "px",
    depends: { key: "urlbar.spotlight", equals: true },
  },
  // ---- tabs ---------------------------------------------------------------
  {
    key: "tabs.favicon-only-threshold",
    type: "number",
    default: 80,
    min: 40,
    max: 200,
    step: 5,
    group: "tabs",
    stringKey: "setting.tabs.favicon-only-threshold",
    cssVar: "--uc-tab-favicon-only",
    cssUnit: "px",
    doc: "Tabs narrower than this show only the favicon.",
  },
  {
    key: "tabs.min-width",
    type: "number",
    default: 76,
    min: 50,
    max: 200,
    step: 2,
    group: "tabs",
    stringKey: "setting.tabs.min-width",
    cssVar: "--uc-tab-min-width",
    cssUnit: "px",
  },
  {
    key: "tabs.close-on-hover",
    type: "boolean",
    default: false,
    group: "tabs",
    stringKey: "setting.tabs.close-on-hover",
    attr: "uc-tab-close-hover",
  },
  {
    key: "tabs.active-accent",
    type: "boolean",
    default: false,
    group: "tabs",
    stringKey: "setting.tabs.active-accent",
    attr: "uc-tab-active-accent",
    doc: "Tints the active tab with the accent color.",
  },
  // ---- motion ---------------------------------------------------------------
  {
    key: "motion.enabled",
    type: "boolean",
    default: true,
    group: "motion",
    stringKey: "setting.motion.enabled",
    attr: "uc-motion",
    doc: "Global motion toggle; combined with respect-reduced-motion.",
  },
  {
    key: "motion.speed",
    type: "number",
    default: 1,
    min: 0.5,
    max: 2,
    step: 0.1,
    group: "motion",
    stringKey: "setting.motion.speed",
    cssVar: "--uc-motion-speed",
  },
  {
    key: "motion.spring-stiffness",
    type: "number",
    default: 300,
    min: 100,
    max: 800,
    step: 10,
    group: "motion",
    stringKey: "setting.motion.spring-stiffness",
    doc: "Regenerates the --uc-spring easing.",
  },
  {
    key: "motion.spring-damping",
    type: "number",
    default: 30,
    min: 10,
    max: 60,
    step: 1,
    group: "motion",
    stringKey: "setting.motion.spring-damping",
    doc: "Regenerates the --uc-spring easing.",
  },
  {
    key: "motion.respect-reduced-motion",
    type: "boolean",
    default: true,
    group: "motion",
    stringKey: "setting.motion.respect-reduced-motion",
    doc: "With the OS reduced-motion preference on, motion is disabled entirely.",
  },
  // ---- dynamic notch ---------------------------------------------------------
  {
    key: "notch.loading",
    type: "boolean",
    default: true,
    group: "dynamicNotch",
    stringKey: "setting.notch.loading",
    attr: "uc-dn-loading",
    depends: { key: "variant", equals: "notch" },
  },
  {
    key: "notch.downloads",
    type: "boolean",
    default: false,
    group: "dynamicNotch",
    stringKey: "setting.notch.downloads",
    attr: "uc-dn-downloads",
    depends: { key: "variant", equals: "notch" },
  },
  {
    key: "notch.media",
    type: "boolean",
    default: true,
    group: "dynamicNotch",
    stringKey: "setting.notch.media",
    attr: "uc-dn-media",
    depends: { key: "variant", equals: "notch" },
  },
  // ---- colors ---------------------------------------------------------------
  {
    key: "colors.source",
    type: "string",
    default: "theme",
    enum: ["theme", "manual", "file"],
    group: "colors",
    stringKey: "setting.colors.source",
    attr: "uc-color-source",
    doc: "theme: follow the active Firefox theme. manual: colors below. file: external palette JSON.",
  },
  {
    key: "colors.palette-file",
    type: "string",
    default: "",
    group: "colors",
    stringKey: "setting.colors.palette-file",
    depends: { key: "colors.source", equals: "file" },
    doc: "Absolute path to a JSON palette file, reloaded when its mtime changes.",
  },
  {
    key: "colors.accent",
    type: "string",
    default: "",
    group: "colors",
    stringKey: "setting.colors.accent",
    cssVar: "--uc-accent",
    depends: { key: "colors.source", equals: "manual" },
    doc: "CSS color for the accent used by the active tab and focus rings.",
  },
  {
    key: "colors.private-tint",
    type: "string",
    default: "",
    group: "colors",
    stringKey: "setting.colors.private-tint",
    cssVar: "--uc-private-tint",
    depends: { key: "colors.source", equals: "manual" },
    doc: "CSS color mixed into the canvas in private windows.",
  },
  {
    key: "colors.blur",
    type: "boolean",
    default: false,
    group: "colors",
    stringKey: "setting.colors.blur",
    attr: "uc-blur",
    doc: "Backdrop blur behind the islands. Off by default for performance.",
  },
  // ---- behavior (experimental) ------------------------------------------------
  {
    key: "behavior.safari-merge",
    type: "boolean",
    default: false,
    group: "behavior",
    stringKey: "setting.behavior.safari-merge",
    attr: "uc-safari-merge",
    doc: "Experimental: place the address field over the active tab, kept in sync on select/scroll/resize/reorder.",
  },
  // ---- presets ------------------------------------------------------------------
  {
    key: "presets.current",
    type: "string",
    default: "",
    group: "presets",
    stringKey: "setting.presets.current",
    doc: "Serialized snapshot of all user settings; export/import payload.",
  },
]);

export const SCHEMA_BY_KEY = Object.freeze(
  Object.fromEntries(SCHEMA.map(s => [s.key, s]))
);

export const PREF = Object.freeze(
  Object.fromEntries(SCHEMA.map(s => [s.key, PREF_BRANCH + s.key]))
);

/** Clamp/validate a raw value for a schema entry; returns the value or null. */
export function normalize(def, raw) {
  switch (def.type) {
    case "boolean": {
      if (typeof raw === "boolean") return raw;
      if (raw === "true" || raw === "false") return raw === "true";
      return null;
    }
    case "number": {
      if (raw === def.sentinel) return def.nullable ? null : def.sentinel;
      let n = typeof raw === "number" ? raw : Number(raw);
      if (!Number.isFinite(n)) return null;
      if (def.min != null) n = Math.max(def.min, n);
      if (def.max != null) n = Math.min(def.max, n);
      if (def.step) {
        n = Math.round(n / def.step) * def.step;
        n = Math.round(n * 1e6) / 1e6;
      }
      return n;
    }
    case "string": {
      if (typeof raw !== "string") return null;
      if (def.enum && !def.enum.includes(raw)) return null;
      return raw;
    }
    default:
      return null;
  }
}