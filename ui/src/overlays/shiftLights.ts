// Shift lights and the RPM readout, shared by the Driver inputs overlay and the Shift lights
// overlay: the car's own light pattern from Lovely Car Data when the car is listed (per-gear
// shift points, its colours, blank positions), otherwise iRacing's shift points.

import { h, show, text, toggle } from '../lib/dom';
import type { SettingDef, Values } from '../lib/settings';
import type { CarInfo, Player } from '../lib/types';

/** The lights' settings, under `group`; `when` limits them (e.g. to the lights being on). */
export function shiftLightSettings(group: string, when: (v: Values) => boolean = () => true): SettingDef[] {
  return [
    {
      key: 'shiftLayout', when,
      label: 'Light pattern',
      type: 'select',
      default: 'car',
      options: [
        ['car', 'As the car has it'],
        ['ltr', 'Left to right'],
        ['rtl', 'Right to left'],
        ['outIn', 'Outside in (both ends to the middle)'],
        ['inOut', 'Inside out (middle to both ends)'],
      ],
      group,
      help: 'The order the lights come on. The shift points and colours stay those of the car',
    },
    { key: 'shiftLightCount', when, label: 'Number of lights', type: 'number', default: 10, min: 3, max: 20, group, help: 'For cars without their own light pattern' },
    { key: 'shiftLightSize', when, label: 'Light size', type: 'number', default: 0.9, min: 0.4, max: 3, step: 0.1, unit: 'em', group },
    { key: 'shiftFlashMs', when, label: 'Flash speed at the shift point', type: 'number', default: 120, min: 40, max: 1000, step: 10, unit: 'ms', group },
    { key: 'shiftLowColor', when, label: 'First lights', type: 'color', default: '#2fd36b', group },
    { key: 'shiftMidColor', when, label: 'Middle lights', type: 'color', default: '#ffd400', group },
    { key: 'shiftHighColor', when, label: 'Last lights', type: 'color', default: '#ff3b3b', group },
    { key: 'shiftFlashColor', when, label: 'Shift now (flash)', type: 'color', default: '#4da3ff', group },
  ];
}

/** Lights for the current car: [rpm on, colour] per light plus the shift-now rpm. */
type Plan = { on: number[]; colors: string[]; flashAt: number; flashColor: string };

/**
 * The lights (`lights`) and the RPM readout (`rpm`, turning the shift-now colour at the shift
 * point). `showLights` / `showRpm` say which are in use.
 */
export function shiftLights(s: Values, showLights: () => boolean, showRpm: () => boolean) {
  const lights = h('div', 'shift-lights');
  lights.style.setProperty('--led', `${s.shiftLightSize}em`);
  const rpm = h('div', 'rpm');
  rpm.style.setProperty('--rpm-shift', s.shiftFlashColor as string);
  let leds: HTMLElement[] = [];
  let car: CarInfo | null = null;
  let plans = new Map<string, Plan | null>();
  let shown: Plan | null = null;

  /**
   * Re-orders the lights for the chosen pattern. The car's lights in the order they come on
   * (lowest rpm first) are the stages; left/right patterns use one light per stage, outside-in
   * and inside-out light pairs, so they use every other stage.
   */
  const arrange = (p: Plan): Plan => {
    const layout = s.shiftLayout as string;
    if (layout === 'car' || p.on.length < 2) return p;
    // Blank positions of the car's light bar are left out of the other patterns.
    const stages = p.on
      .map((r, i) => ({ rpm: r, color: p.colors[i] }))
      .filter((x) => Number.isFinite(x.rpm))
      .sort((a, b) => a.rpm - b.rpm);
    if (stages.length < 2) return p;
    const n = stages.length;
    if (layout === 'ltr' || layout === 'rtl') {
      const order = layout === 'ltr' ? stages : [...stages].reverse();
      return { ...p, on: order.map((x) => x.rpm), colors: order.map((x) => x.color) };
    }
    // Pairs from the ends (outside in) or from the middle (inside out).
    const m = Math.ceil(n / 2);
    const pick = (k: number) => stages[Math.round((k * (n - 1)) / Math.max(1, m - 1))];
    const ls = Array.from({ length: n }, (_, i) => {
      const fromEnd = Math.min(i, n - 1 - i);
      return pick(layout === 'outIn' ? fromEnd : m - 1 - fromEnd);
    });
    return { ...p, on: ls.map((x) => x.rpm), colors: ls.map((x) => x.color) };
  };
  const ramp = (i: number, n: number) => (i < n * 0.4 ? (s.shiftLowColor as string) : i < n * 0.7 ? (s.shiftMidColor as string) : (s.shiftHighColor as string));
  const plan = (gear: number): Plan | null => {
    const key = gear === -1 ? 'R' : gear === 0 ? 'N' : String(gear);
    if (plans.has(key)) return plans.get(key)!;
    let p: Plan | null = null;
    const lv = car?.lovely;
    const g = lv && (lv.rpm[key] ?? lv.rpm[String(Math.max(1, gear))]);
    if (lv && g && g.length > 1) {
      const n = Math.min(lv.leds, g.length - 1);
      // A light at 0 rpm or with a transparent colour ('') is a blank spot in the car's
      // light bar (e.g. GT3 wheels with 12 positions and 10 lights): it never lights.
      const blank = (i: number) => g[i + 1] <= 0 || lv.colors[i + 1] === '';
      const on = g.slice(1, n + 1).map((r, i) => (blank(i) ? Infinity : r));
      p = { on, colors: on.map((_, i) => (blank(i) ? '' : lv.colors[i + 1] || ramp(i, n))), flashAt: g[0], flashColor: lv.colors[0] || (s.shiftFlashColor as string) };
    } else if (car && car.slShift > 0) {
      const n = s.shiftLightCount as number;
      const first = car.slFirst > 0 ? car.slFirst : car.slShift * 0.85;
      const last = car.slLast > first ? car.slLast : car.slShift;
      const on = Array.from({ length: n }, (_, i) => first + ((last - first) * i) / Math.max(1, n - 1));
      p = { on, colors: on.map((_, i) => ramp(i, n)), flashAt: car.slBlink > 0 ? car.slBlink : car.slShift, flashColor: s.shiftFlashColor as string };
    }
    if (p) p = arrange(p);
    plans.set(key, p);
    return p;
  };

  return {
    lights,
    rpm,
    /** The car changed: its light pattern is worked out again. */
    setCar(c: CarInfo) {
      car = c;
      plans = new Map();
      shown = null;
    },
    /** Whether there is a light pattern for this car (none until the car is known). */
    hasPlan: (gear: number) => !!plan(gear),
    draw(p: Player) {
      const pl = showLights() ? plan(p.gear) : null;
      show(lights, !!pl);
      if (pl) {
        if (pl !== shown) {
          shown = pl;
          leds = pl.on.map((r) => h('span', Number.isFinite(r) ? 'led' : 'led blank'));
          lights.replaceChildren(...leds);
        }
        const flash = p.rpm >= pl.flashAt && pl.flashAt > 0;
        const blinkOn = !flash || Math.floor(performance.now() / (s.shiftFlashMs as number)) % 2 === 0;
        leds.forEach((el, i) => {
          const blank = !Number.isFinite(pl.on[i]);
          const lit = !blank && (flash ? blinkOn : p.rpm >= pl.on[i]);
          const color = lit ? (flash ? pl.flashColor : pl.colors[i]) : '';
          if (el.style.getPropertyValue('--c') !== color) el.style.setProperty('--c', color);
          toggle(el, 'on', lit);
        });
      }
      if (showRpm()) {
        text(rpm, String(Math.max(0, Math.round(p.rpm))));
        // At the shift point: the shift lights' "shift now" point when known, else iRacing's.
        const at = plan(p.gear)?.flashAt || p.shiftRpm;
        toggle(rpm, 'shift', at > 0 && p.rpm >= at);
      }
    },
  };
}
