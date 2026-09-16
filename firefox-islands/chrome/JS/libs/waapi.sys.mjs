/* islands — Web Animations API helpers.
 *
 * Cancellation-safe animation runner plus a FLIP utility for layout
 * transitions. Everything animates transform/opacity only; no synchronous
 * layout is forced inside animation frames (reads are batched before
 * writes).
 */

import { Motion } from "chrome://userscripts/content/libs/motion.sys.mjs";

/**
 * Animate `element` with the Web Animations API. Any prior animation we own
 * on the element is cancelled first.
 *
 * @param {Element} element
 * @param {Object[]} frames   keyframes for element.animate()
 * @param {Object} options    options (duration, easing, fill, ...)
 * @returns {Animation}
 */
export function animate(element, frames, options) {
  if (element._ucAnimation) element._ucAnimation.cancel();
  const anim = element.animate(frames, options);
  element._ucAnimation = anim;
  anim.addEventListener("finish", () => (element._ucAnimation = null), {
    once: true,
  });
  return anim;
}

/**
 * FLIP: First / Last / Invert / Play a layout change. Reads positions in
 * both states, inverts the delta, then plays the transform back to zero.
 * Falls back to a no-op when the element is not laid out.
 *
 * @param {Element} element
 * @param {() => void} change   performs the layout change between measurements
 * @param {Object} [opts]       { durationMs, easing, speed, transformOrigin }
 */
export function flip(element, change, opts = {}) {
  const first = element.getBoundingClientRect();
  change();
  const last = element.getBoundingClientRect();
  if (!first.width || !first.height) return;

  const dx = first.left - last.left;
  const dy = first.top - last.top;
  if (dx === 0 && dy === 0) return;

  const speed = opts.speed ?? 1;
  const durationMs = Motion.duration(opts.durationMs ?? 260, speed);
  const easing = opts.easing ?? "var(--uc-spring)";

  element.style.transformOrigin = opts.transformOrigin ?? "center";
  animate(
    element,
    [
      { transform: `translate(${dx}px, ${dy}px)` },
      { transform: "translate(0, 0)" },
    ],
    { duration: durationMs, easing, fill: "both" }
  );
}

export const Waapi = Object.freeze({
  animate,
  flip,
});