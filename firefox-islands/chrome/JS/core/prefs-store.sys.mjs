/* islands — prefs store.
 *
 * Global singleton built from the schema. Provides typed, validated and
 * clamped access to every `uc.islands.*` pref, registers the defaults on the
 * default branch at startup (so user changes persist in prefs.js and are
 * never re-applied), and notifies listeners when any pref in the branch
 * changes.
 *
 * Services/Ci are used as chrome globals (same convention as the loader's own
 * modules), which keeps this module importable in any privileged context.
 */

import {
  PREF_BRANCH,
  SCHEMA,
  SCHEMA_BY_KEY,
  normalize,
} from "chrome://userscripts/content/core/schema.sys.mjs";

const kPrefType = Services.prefs;

function toPrefValue(def, value) {
  if (value === null || value === undefined) {
    return def.nullable ? def.sentinel : def.type === "string" ? "" : def.type === "boolean" ? false : 0;
  }
  return value;
}

function fromPrefValue(def, raw) {
  if (def.nullable && raw === def.sentinel) return null;
  return raw;
}

/** Register every schema default on the default branch, once. */
function registerDefaults() {
  const branch = kPrefType.getDefaultBranch(PREF_BRANCH);
  for (const def of SCHEMA) {
    const value = toPrefValue(def, def.default);
    switch (def.type) {
      case "boolean":
        branch.setBoolPref(def.key, value);
        break;
      case "number":
        branch.setCharPref(def.key, String(value));
        break;
      case "string":
        branch.setStringPref(def.key, value);
        break;
    }
  }
}

class PrefsStore {
  constructor() {
    registerDefaults();
    this._listeners = new Set();
    this._branchObserver = null;
    this._started = false;
  }

  /** Start observing pref changes. Safe to call multiple times. */
  start() {
    if (this._started) return;
    this._started = true;
    this._branchObserver = {
      observe(subject, topic, data) {
        // `data` is the full pref name, e.g. "uc.islands.gap"
        if (!data.startsWith(PREF_BRANCH)) return;
        const key = data.slice(PREF_BRANCH.length);
        const def = SCHEMA_BY_KEY[key];
        if (!def) return;
        const raw = store._readRaw(def);
        const value = fromPrefValue(def, normalize(def, raw));
        for (const cb of store._listeners) {
          try {
            cb(key, value, def);
          } catch (e) {
            console.error("uc.islands pref listener error", e);
          }
        }
      },
    };
    kPrefType.addObserver(PREF_BRANCH, this._branchObserver, true);
  }

  stop() {
    if (!this._started) return;
    this._started = false;
    kPrefType.removeObserver(PREF_BRANCH, this._branchObserver);
    this._branchObserver = null;
  }

  _readRaw(def) {
    try {
      switch (def.type) {
        case "boolean":
          return kPrefType.getBoolPref(PREF_BRANCH + def.key);
        case "number": {
          const raw = kPrefType.getCharPref(PREF_BRANCH + def.key, String(def.default));
          return Number(raw);
        }
        case "string":
          return kPrefType.getStringPref(PREF_BRANCH + def.key, def.default);
      }
    } catch (e) {
      return def.default;
    }
    return def.default;
  }

  /** Typed read of one setting. */
  get(key) {
    const def = SCHEMA_BY_KEY[key];
    if (!def) throw new Error(`Unknown setting: ${key}`);
    const raw = this._readRaw(def);
    const value = normalize(def, raw);
    if (value === null) return fromPrefValue(def, def.default);
    return fromPrefValue(def, value);
  }

  /** All settings as a plain object (used by presets and diagnostics). */
  getAll() {
    const out = {};
    for (const def of SCHEMA) out[def.key] = this.get(def.key);
    return out;
  }

  /**
   * Typed, validated, clamped write. Persists through the normal prefs
   * machinery (prefs.js) and fires the branch observer. Returns the final
   * stored value, or null when the input was rejected.
   */
  set(key, raw) {
    const def = SCHEMA_BY_KEY[key];
    if (!def) throw new Error(`Unknown setting: ${key}`);
    const value = normalize(def, raw);
    if (value === null) return null;
    const stored = toPrefValue(def, value);
    try {
      switch (def.type) {
        case "boolean":
          kPrefType.setBoolPref(PREF_BRANCH + key, stored);
          break;
        case "number":
          kPrefType.setCharPref(PREF_BRANCH + key, String(stored));
          break;
        case "string":
          kPrefType.setStringPref(PREF_BRANCH + key, stored);
          break;
      }
    } catch (e) {
      console.error("uc.islands failed to write pref", key, e);
      return null;
    }
    return fromPrefValue(def, value);
  }

  /** Reset a single setting to its default (clears the user value). */
  reset(key) {
    const def = SCHEMA_BY_KEY[key];
    if (!def) throw new Error(`Unknown setting: ${key}`);
    kPrefType.clearUserPref(PREF_BRANCH + key);
  }

  /** Reset every setting to its default. */
  resetAll() {
    for (const def of SCHEMA) kPrefType.clearUserPref(PREF_BRANCH + def.key);
  }

  /**
   * Subscribe to changes. The callback receives (key, value, def) for every
   * changed pref in the branch. Returns an unsubscribe function.
   */
  observe(cb) {
    this._listeners.add(cb);
    return () => this._listeners.delete(cb);
  }

  get schema() {
    return SCHEMA;
  }
}

const store = new PrefsStore();
export { store as PrefsStore };