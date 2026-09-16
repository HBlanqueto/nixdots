/* islands — global bootstrap.
 *
 * Loaded by fx-autoconfig as a top-level .sys.mjs (background module) once at
 * startup in the shared system global. Initializes the global singletons;
 * per-window controllers reach the same instances through
 * ChromeUtils.importESModule. Everything here is best-effort: a failure must
 * not prevent the CSS-only layout from working.
 */

import { PrefsStore } from "chrome://userscripts/content/core/prefs-store.sys.mjs";
import { ThemeSource } from "chrome://userscripts/content/core/theme-source.sys.mjs";

try {
  PrefsStore.start();
  ThemeSource.ensureStarted();
  console.log("uc.islands core ready");
} catch (e) {
  console.error("uc.islands bootstrap failed", e);
}