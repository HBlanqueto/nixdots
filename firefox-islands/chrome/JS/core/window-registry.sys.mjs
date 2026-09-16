/* islands — window registry.
 *
 * Tracks the open browser windows that have an active controller so the
 * global modules (prefs store, theme source) can reach them, and so the
 * settings page can open windows / trigger actions. Entries are removed on
 * window unload to avoid leaks.
 */

const windows = new Set();

export function registerWindow(win) {
  windows.add(win);
  return () => windows.delete(win);
}

export function getWindows() {
  return [...windows];
}

export function forEachWindow(cb) {
  for (const win of windows) {
    try {
      cb(win);
    } catch (e) {
      console.error("uc.islands window callback error", e);
    }
  }
}

export const WindowRegistry = Object.freeze({
  register: registerWindow,
  getWindows,
  forEach: forEachWindow,
  get size() {
    return windows.size;
  },
});