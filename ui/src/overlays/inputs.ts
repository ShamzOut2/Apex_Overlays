import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { CarInfo, Player } from '../lib/types';
import { inputTrace, TRACE_SETTINGS } from './inputTrace';

const WHEEL_SVG = `<svg viewBox="-50 -50 100 100"><circle r="42" fill="none" stroke="currentColor" stroke-width="9"/><path d="M-40 4 H-12 L-6 14 H6 L12 4 H40" fill="none" stroke="currentColor" stroke-width="8" stroke-linejoin="round"/><path d="M0 14 V40" stroke="currentColor" stroke-width="8"/><rect x="-3" y="-50" width="6" height="12" fill="var(--accent)"/></svg>`;

export const inputs: OverlayDef = {
  kind: 'inputs',
  name: 'Driver inputs',
  description: 'Throttle, brake and clutch bars with a scrolling input trace, steering wheel, gear and speed. Updates every telemetry tick.',
  topics: ['player', 'session', 'car'],
  size: [420, 140],
  settings: [
    { key: 'showShiftLights', label: 'Shift lights', type: 'bool', default: true, group: 'Shift lights', help: 'Per-gear shift points from Lovely Car Data when the car is listed, otherwise iRacing’s own' },
    {
      key: 'shiftLayout',
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
      group: 'Shift lights',
      help: 'The order the lights come on. The shift points and colours stay those of the car',
    },
    { key: 'shiftLightCount', label: 'Number of lights (cars without Lovely data)', type: 'number', default: 10, min: 3, max: 20, group: 'Shift lights' },
    { key: 'shiftLightSize', label: 'Light size', type: 'number', default: 0.9, min: 0.4, max: 3, step: 0.1, unit: 'em', group: 'Shift lights' },
    { key: 'shiftFlashMs', label: 'Flash speed at the shift point', type: 'number', default: 120, min: 40, max: 1000, step: 10, unit: 'ms', group: 'Shift lights' },
    { key: 'shiftLowColor', label: 'First lights', type: 'color', default: '#2fd36b', group: 'Shift lights' },
    { key: 'shiftMidColor', label: 'Middle lights', type: 'color', default: '#ffd400', group: 'Shift lights' },
    { key: 'shiftHighColor', label: 'Last lights', type: 'color', default: '#ff3b3b', group: 'Shift lights' },
    { key: 'shiftFlashColor', label: 'Shift now (flash)', type: 'color', default: '#4da3ff', group: 'Shift lights' },
    {
      key: 'showTrace',
      label: 'Input trace graph',
      type: 'bool',
      default: true,
      group: 'Elements',
      help: 'To place the graph on its own, switch this off and turn on the Input graph overlay',
    },
    { key: 'showBars', label: 'Pedal bars', type: 'bool', default: true, group: 'Elements' },
    { key: 'showClutch', label: 'Clutch', type: 'bool', default: true, group: 'Elements' },
    { key: 'showWheel', label: 'Steering wheel', type: 'bool', default: true, group: 'Elements' },
    { key: 'showGear', label: 'Gear', type: 'bool', default: true, group: 'Elements' },
    { key: 'showSpeed', label: 'Speed', type: 'bool', default: true, group: 'Elements' },
    {
      key: 'valuePosition',
      label: 'Percent values',
      type: 'select',
      default: 'top',
      options: [['top', 'Above each bar'], ['inside', 'Inside the bar'], ['off', 'Off']],
      group: 'Elements',
    },
    { key: 'valueColored', label: 'Colour values like their bar', type: 'bool', default: true, group: 'Elements' },
    ...TRACE_SETTINGS,
    { key: 'throttleColor', label: 'Throttle', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'brakeColor', label: 'Brake', type: 'color', default: '#ff4d4d', group: 'Colours' },
    { key: 'clutchColor', label: 'Clutch', type: 'color', default: '#4da3ff', group: 'Colours' },
    { key: 'absColor', label: 'ABS', type: 'color', default: '#ffd84d', group: 'Colours' },
    { key: 'steeringColor', label: 'Steering', type: 'color', default: '#e6e9ef', group: 'Colours' },
    { key: 'wheelColor', label: 'Wheel', type: 'color', default: '#e6e9ef', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('inputs');
    root.style.setProperty('--abs-color', s.absColor as string);
    const lights = h('div', 'shift-lights');
    lights.style.setProperty('--led', `${s.shiftLightSize}em`);
    const main = h('div', 'inputs-main');
    let leds: HTMLElement[] = [];
    let car: CarInfo | null = null;
    const canvas = h('canvas', 'trace');
    const bars = h('div', 'bars');
    const valuePos = s.valuePosition as string;
    const mk = (cls: string, color: string) => {
      const wrap = h('div', `barcol ${cls}`);
      const bar = h('div', 'bar');
      const fill = h('div', 'fill');
      const val = h('div', `val ${valuePos}`);
      fill.style.background = color;
      if (s.valueColored && valuePos === 'top') val.style.color = color;
      bar.append(fill);
      if (valuePos === 'top') wrap.append(val, bar);
      else if (valuePos === 'inside') {
        bar.append(val);
        wrap.append(bar);
      } else wrap.append(bar);
      bars.append(wrap);
      return { wrap, fill, val };
    };
    const clutch = mk('clutch', s.clutchColor as string);
    const brake = mk('brake', s.brakeColor as string);
    const throttle = mk('throttle', s.throttleColor as string);
    show(clutch.wrap, s.showClutch as boolean);
    const side = h('div', 'side');
    const wheel = h('div', 'wheel');
    wheel.innerHTML = WHEEL_SVG;
    wheel.style.color = s.wheelColor as string;
    const gear = h('div', 'gear');
    const spd = h('div', 'speed');
    side.append(wheel, gear, spd);
    show(canvas, s.showTrace as boolean);
    show(bars, s.showBars as boolean);
    show(wheel, s.showWheel as boolean);
    show(gear, s.showGear as boolean);
    show(spd, s.showSpeed as boolean);
    show(side, (s.showWheel || s.showGear || s.showSpeed) as boolean);
    main.append(canvas, bars, side);
    root.append(lights, main);

    /** Lights for the current car: [rpm on, colour] per light plus the shift-now rpm. */
    type Plan = { on: number[]; colors: string[]; flashAt: number; flashColor: string };
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
        .map((rpm, i) => ({ rpm, color: p.colors[i] }))
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
      const lights = Array.from({ length: n }, (_, i) => {
        const fromEnd = Math.min(i, n - 1 - i);
        return pick(layout === 'outIn' ? fromEnd : m - 1 - fromEnd);
      });
      return { ...p, on: lights.map((x) => x.rpm), colors: lights.map((x) => x.color) };
    };
    let plans = new Map<string, Plan | null>();
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
        const on = g.slice(1, n + 1).map((rpm, i) => (blank(i) ? Infinity : rpm));
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
    let shown: Plan | null = null;
    const drawLights = (p: Player) => {
      const pl = s.showShiftLights ? plan(p.gear) : null;
      show(lights, !!pl);
      if (!pl) return;
      if (pl !== shown) {
        shown = pl;
        leds = pl.on.map((rpm) => h('span', Number.isFinite(rpm) ? 'led' : 'led blank'));
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
    };

    const trace = inputTrace(canvas, s, () => !!s.showClutch);

    return {
      update(topic, data) {
        if (topic === 'car') {
          car = data as CarInfo;
          plans = new Map();
          shown = null;
          return;
        }
        if (topic !== 'player') return;
        const p = data as Player;
        drawLights(p);

        const bar = (b: { fill: HTMLElement; val: HTMLElement }, v: number) => {
          style(b.fill, 'transform', `scaleY(${v.toFixed(3)})`);
          if (valuePos !== 'off') text(b.val, String(Math.round(v * 100)));
        };
        bar(throttle, p.throttle);
        bar(brake, p.brake);
        bar(clutch, p.clutch);
        toggle(brake.wrap, 'abs', !!s.absHighlight && p.abs);
        style(wheel, 'transform', `rotate(${(-p.steering * 180) / Math.PI}deg)`);
        text(gear, p.gear === -1 ? 'R' : p.gear === 0 ? 'N' : String(p.gear));
        text(spd, `${Math.round(f.speed(p.speed, ctx.units))}`);
        spd.dataset.unit = f.speedUnit(ctx.units);

        if (s.showTrace) trace.push(p);
      },
    };
  },
};
