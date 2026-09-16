/* islands — motion helpers.
 *
 * Turns spring parameters (stiffness, damping) into a CSS `linear()`
 * easing string so CSS transitions and Web Animations share one curve, and
 * exposes a few timing helpers. All durations are scaled by the global speed
 * setting through --uc-motion-speed (CSS) or Motion.duration (JS).
 */

export function prefersReducedMotion(win) {
  return win.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const STEP_MS = 1 / 240;

/**
 * Integrate a damped spring from 0 toward 1 (v0 = 0) with RK4.
 * Returns { settle, samples } where samples are {t, x} pairs.
 */
function integrate(stiffness, damping) {
  const m = 1;
  const k = stiffness / m;
  const c = damping / m;
  const dt = STEP_MS;
  let t = 0;
  let x = 0;
  let v = 0;
  const target = 1;
  const samples = [];
  while (t < 3) {
    if (t === 0) samples.push({ t: 0, x: 0 });
    else samples.push({ t, x });
    const a = (-c * v - k * (x - target)) / m;
    const v1 = v + (a * dt) / 2;
    const x1 = x + (v * dt) / 2;
    const a1 = (-c * v1 - k * (x1 - target)) / m;
    const v2 = v + (a1 * dt) / 2;
    const x2 = x + (v1 * dt) / 2;
    const a2 = (-c * v2 - k * (x2 - target)) / m;
    const v3 = v + a2 * dt;
    const x3 = x + v2 * dt;
    const a3 = (-c * v3 - k * (x3 - target)) / m;
    x += (v + 2 * v1 + 2 * v2 + v3) * (dt / 6);
    v += (a + 2 * a1 + 2 * a2 + a3) * (dt / 6);
    t += dt;
    if (t > 0.05 && Math.abs(x - target) < 0.0005 && Math.abs(v) < 0.0005) {
      break;
    }
  }
  return { settle: t, samples };
}

const POINT_COUNT = 17;

/**
 * Build a CSS `linear()` easing string from the spring response, sampled at
 * POINT_COUNT evenly spaced inputs. Overshoot is preserved (outputs > 1 are
 * legal inside linear()). Returns a string like
 * "linear(0 0%, 0.2039 6.25%, ...)".
 */
export function springLinear(stiffness, damping) {
  const { settle, samples } = integrate(stiffness, damping);
  let peak = 1;
  for (const s of samples) if (s.x > peak) peak = s.x;
  const scale = peak > 1.001 ? peak : 1;

  const stops = [];
  for (let i = 0; i < POINT_COUNT; i++) {
    const targetT = (settle * i) / (POINT_COUNT - 1);
    let j = 0;
    while (j < samples.length - 1 && samples[j + 1].t < targetT) j++;
    const a = samples[j];
    const b = samples[Math.min(j + 1, samples.length - 1)];
    const f = b.t - a.t > 0 ? (targetT - a.t) / (b.t - a.t) : 0;
    const x = a.x + (b.x - a.x) * f;
    const out = Math.round((x / scale) * 1e4) / 1e4;
    const input = Math.round((targetT / settle) * 100 * 100) / 100;
    stops.push(`${out} ${input}%`);
  }
  return `linear(${stops.join(", ")})`;
}

/** Duration helper honoring the global speed multiplier. */
export function duration(baseMs, speed) {
  return Math.round(baseMs * speed);
}

export const Motion = Object.freeze({
  springLinear,
  prefersReducedMotion,
  duration,
});