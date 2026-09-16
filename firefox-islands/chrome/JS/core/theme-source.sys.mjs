/* islands — theme source.
 *
 * Pluggable adapter that produces a palette for the design. Sources:
 *   "theme"   follow the active Firefox theme (default) — derives the
 *             palette from a window's computed --lwt-* variables, so it
 *             tracks any WebExtension theme and the default theme.
 *   "manual"  accent and private-window tint from the settings; the rest
 *             keeps the design defaults.
 *   "file"    an external palette JSON file, reloaded on mtime change by a
 *             single global low-frequency watcher.
 *
 * The interface is intentionally tiny (apply() + last palette) so it can be
 * swapped for a desktop-wide settings bus (SomeWM / Quickshell) later.
 */

import { PrefsStore } from "chrome://userscripts/content/core/prefs-store.sys.mjs";
import { EventBus } from "chrome://userscripts/content/core/event-bus.sys.mjs";
import { WindowRegistry } from "chrome://userscripts/content/core/window-registry.sys.mjs";

// Services and IOUtils are chrome globals (loader convention).

const kObsThemeSet = "LightweightTheme:Set";
const FILE_POLL_MS = 15000;

/** Best-effort luminance of a resolved CSS color string (0..1), or null. */
function luminance(cssColor) {
  if (!cssColor) return null;
  const m = cssColor.match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const parts = m[1].split(",").map(s => Number(s.trim()));
  if (parts.length < 3 || parts.some(n => !Number.isFinite(n))) return null;
  const [r, g, b] = parts;
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

function isDarkColor(cssColor) {
  const l = luminance(cssColor);
  return l === null ? null : l < 0.5;
}

function readVar(win, name) {
  const root = win.document.documentElement;
  return getComputedStyle(root).getPropertyValue(name).trim();
}

function pick(first, ...rest) {
  for (const v of [first, ...rest]) {
    if (v) return v;
  }
  return null;
}

/**
 * Derive the palette from a window's computed theme variables. Returns an
 * object with CSS color strings; null entries mean "keep the design default".
 */
function fromThemeWindow(win) {
  const root = win.document.documentElement;
  const cs = getComputedStyle(root);
  const accent =
    pick(
      cs.getPropertyValue("--lwt-accent-color").trim(),
      cs.getPropertyValue("--color-accent-primary").trim()
    ) || null;
  const text =
    pick(
      cs.getPropertyValue("--lwt-text-color").trim(),
      cs.getPropertyValue("--toolbox-text-color").trim()
    ) || null;
  const toolbox =
    cs.getPropertyValue("--toolbox-background-color").trim() || null;
  const toolbar =
    cs.getPropertyValue("--toolbar-background-color").trim() || null;
  const separator =
    cs.getPropertyValue("--chrome-content-separator-color").trim() || null;

  // canvas follows the window frame/toolbox color; fall back to the accent.
  const canvasBase = pick(toolbox, accent);
  const isDark = isDarkColor(canvasBase) ?? isDarkColor(accent) ?? false;

  return {
    canvasBase,
    islandBg: toolbar,
    islandBgHover: toolbar,
    fg: text,
    fgMuted: text,
    accent,
    frameBorder: separator,
    privateTint: null,
    isDark,
  };
}

/** Merge a file palette (JSON) into a palette object. */
function fromFilePalette(obj) {
  const map = {
    canvas: "canvasBase",
    island: "islandBg",
    "island-hover": "islandBgHover",
    fg: "fg",
    "fg-muted": "fgMuted",
    accent: "accent",
    "frame-border": "frameBorder",
    "private-tint": "privateTint",
  };
  const out = {};
  for (const [k, v] of Object.entries(map)) {
    const raw = obj?.[k];
    out[v] = typeof raw === "string" && raw ? raw : null;
  }
  out.isDark = isDarkColor(out.canvasBase) ?? false;
  return out;
}

class ThemeSourceImpl {
  constructor() {
    this._palette = null;
    this._probeWindow = null;
    this._fileWatcher = null;
    this._lastMtime = null;
    this._applying = false;
  }

  /** The window used to sample theme colors (most recently focused). */
  setProbeWindow(win) {
    if (win && !win.closed) this._probeWindow = win;
  }

  get palette() {
    return this._palette;
  }

  async ensureStarted() {
    Services.obs.addObserver(this, kObsThemeSet);
    PrefsStore.observe((key) => {
      if (key.startsWith("colors.") || key === "enabled") this.apply();
    });
    this._manageFileWatcher();
    await this.apply();
  }

  stop() {
    Services.obs.removeObserver(this, kObsThemeSet);
    this._stopFileWatcher();
  }

  observe() {
    // Theme set or changed anywhere: re-derive for every window.
    this.apply();
  }

  async apply() {
    if (this._applying) return;
    this._applying = true;
    try {
      const source = PrefsStore.get("colors.source");
      let palette = null;
      if (source === "theme") {
        const win = this._probeWindow && !this._probeWindow.closed
          ? this._probeWindow
          : WindowRegistry.getWindows()[0];
        palette = win
          ? fromThemeWindow(win)
          : { canvasBase: null, islandBg: null, islandBgHover: null, fg: null, fgMuted: null, accent: null, frameBorder: null, privateTint: null, isDark: false };
      } else if (source === "manual") {
        palette = {
          canvasBase: null,
          islandBg: null,
          islandBgHover: null,
          fg: null,
          fgMuted: null,
          accent: PrefsStore.get("colors.accent") || null,
          frameBorder: null,
          privateTint: PrefsStore.get("colors.private-tint") || null,
          isDark: false,
        };
      } else {
        palette = await this._readPaletteFile();
      }
      this._palette = palette;
      this._manageFileWatcher();
      EventBus.emit("theme", { palette });
    } finally {
      this._applying = false;
    }
  }

  async _readPaletteFile() {
    const path = PrefsStore.get("colors.palette-file");
    if (!path) return this._emptyPalette();
    try {
      const data = JSON.parse(await IOUtils.readUTF8(path));
      return fromFilePalette(data);
    } catch (e) {
      console.error("uc.islands palette file unreadable:", path, e);
      return this._emptyPalette();
    }
  }

  _emptyPalette() {
    return {
      canvasBase: null,
      islandBg: null,
      islandBgHover: null,
      fg: null,
      fgMuted: null,
      accent: null,
      frameBorder: null,
      privateTint: null,
      isDark: false,
    };
  }

  /** Single global watcher: only while source=file and there are windows. */
  _manageFileWatcher() {
    const source = PrefsStore.get("colors.source");
    const shouldWatch =
      source === "file" && WindowRegistry.getWindows().length > 0;
    if (shouldWatch && !this._fileWatcher) {
      this._fileWatcher = setInterval(() => this._checkFile(), FILE_POLL_MS);
    } else if (!shouldWatch && this._fileWatcher) {
      this._stopFileWatcher();
    }
  }

  _stopFileWatcher() {
    if (this._fileWatcher) {
      clearInterval(this._fileWatcher);
      this._fileWatcher = null;
    }
    this._lastMtime = null;
  }

  async _checkFile() {
    const path = PrefsStore.get("colors.palette-file");
    if (!path) return;
    try {
      const stat = await IOUtils.stat(path);
      const mtime = stat.lastModified;
      if (mtime !== this._lastMtime) {
        this._lastMtime = mtime;
        await this.apply();
      }
    } catch (e) {
      // File disappeared; ignore until it returns.
    }
  }
}

const themeSource = new ThemeSourceImpl();
export { themeSource as ThemeSource };