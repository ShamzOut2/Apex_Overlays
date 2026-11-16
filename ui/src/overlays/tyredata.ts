// Your tyres and brakes: temperatures across each tyre, pressures and tread left, and brake
// temperatures. Live in Le Mans Ultimate; iRacing only measures tyres at a pit stop, so there
// it shows the readings from your last stop.

import { h, show, style, text, toggle } from '../lib/dom';
import type { Units } from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Status, TyreData, WheelData } from '../lib/types';

const w = (t: [number, number, number], carcass: number, pressure: number, wear: number, brake: number): WheelData => ({ temps: t, carcass, pressure, wear, brake, flat: false });

/** Shown while placing the overlay: a few laps into a stint, braking into a right-hander. */
const SAMPLE: TyreData = {
  game: 'lmu',
  driving: true,
  live: true,
  optimal: 88,
  wheels: [w([86, 91, 97], 84, 178.2, 91.4, 640), w([99, 93, 88], 87, 179.6, 89.8, 655), w([81, 85, 90], 80, 174.1, 93.0, 430), w([92, 87, 83], 82, 175.0, 92.1, 445)],
};

const CORNERS = ['FL', 'FR', 'RL', 'RR'];

function temp(c: number, u: Units): string {
  return `${Math.round(u === 'imperial' ? (c * 9) / 5 + 32 : c)}°`;
}

function pressure(kpa: number, unit: string, u: Units): string {
  const p = unit === 'auto' ? (u === 'imperial' ? 'psi' : 'kpa') : unit;
  if (p === 'psi') return `${(kpa * 0.145038).toFixed(1)} psi`;
  if (p === 'bar') return `${(kpa / 100).toFixed(2)} bar`;
  return `${kpa.toFixed(0)} kPa`;
}

export const tyredata: OverlayDef = {
  kind: 'tyredata',
  name: 'Tyre data',
  description:
    'Your tyres and brakes, wheel by wheel: temperatures across each tyre (coloured cold, in the window, hot), pressures and tread left, plus brake temperatures. Live in Le Mans Ultimate; iRacing only measures tyres at pit stops, so there it shows the readings from your last stop.',
  topics: ['tyredata', 'status'],
  size: [380, 230],
  commonDefaults: { autoHeight: true, hideWhenNotDriving: true },
  settings: [
    { key: 'showTemps', label: 'Temperatures across each tyre', type: 'bool', default: true, group: 'What to show' },
    { key: 'showCarcass', label: 'Carcass temperature', type: 'bool', default: false, group: 'What to show', lmu: true, help: 'The tyre’s core, slower to change than the surface' },
    { key: 'showPressure', label: 'Pressure', type: 'bool', default: true, group: 'What to show', help: 'iRacing: the cold pressure set at your last stop' },
    { key: 'showWear', label: 'Tread left', type: 'bool', default: true, group: 'What to show' },
    { key: 'showBrakes', label: 'Brake temperatures', type: 'bool', default: true, group: 'What to show', lmu: true, help: 'iRacing does not share brake temperatures' },
    { key: 'pressureUnit', when: (v) => v.showPressure !== false, label: 'Pressure in', type: 'select', default: 'auto', options: [['auto', 'kPa or psi (follows the units setting)'], ['kpa', 'kPa'], ['psi', 'psi'], ['bar', 'bar']], group: 'What to show' },
    { key: 'useGameWindow', label: 'Use the game’s ideal tyre temperature', type: 'bool', default: true, group: 'Tyre window', lmu: true, help: 'Le Mans Ultimate says each tyre’s ideal temperature; the window is that ± the margin below' },
    { key: 'windowMargin', when: (v) => v.useGameWindow !== false, label: 'Either side of the ideal', type: 'number', default: 12, min: 2, max: 40, unit: '°C', group: 'Tyre window', lmu: true },
    { key: 'coldBelow', label: 'Cold below', type: 'number', default: 70, min: 0, max: 150, unit: '°C', group: 'Tyre window', help: 'Used when the game gives no ideal temperature' },
    { key: 'hotAbove', label: 'Hot above', type: 'number', default: 105, min: 20, max: 200, unit: '°C', group: 'Tyre window', help: 'Used when the game gives no ideal temperature' },
    { key: 'brakeCold', when: (v) => v.showBrakes !== false, label: 'Brakes cold below', type: 'number', default: 250, min: 0, max: 1000, step: 10, unit: '°C', group: 'Brakes', lmu: true },
    { key: 'brakeHot', when: (v) => v.showBrakes !== false, label: 'Brakes hot above', type: 'number', default: 800, min: 100, max: 1500, step: 10, unit: '°C', group: 'Brakes', lmu: true },
    { key: 'wearWarn', when: (v) => v.showWear !== false, label: 'Tread left in amber below', type: 'number', default: 35, min: 0, max: 100, unit: '%', group: 'Tread' },
    { key: 'wearBad', when: (v) => v.showWear !== false, label: 'Tread left in red below', type: 'number', default: 15, min: 0, max: 100, unit: '%', group: 'Tread' },
    { key: 'coldColor', label: 'Cold', type: 'color', default: '#3aa0ff', group: 'Colours' },
    { key: 'okColor', label: 'In the window', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'hotColor', label: 'Hot', type: 'color', default: '#ff4d3a', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('tyredata');
    style(root, '--td-cold', s.coldColor as string);
    style(root, '--td-ok', s.okColor as string);
    style(root, '--td-hot', s.hotColor as string);
    toggle(root, 'no-brakes', s.showBrakes === false);
    const head = h('div', 'td-head');
    const tag = h('span', 'td-tag');
    const ideal = h('span', 'td-ideal');
    const headRight = h('span', 'td-head-r');
    headRight.append(ideal, tag);
    head.append(h('span', 'td-title', 'TYRES'), headRight);
    // A car from above: the wheels at its corners, each wheel's readings beside it.
    const car = h('div', 'td-car');
    const body = h('div', 'td-body');
    body.append(h('span', 'td-wing front'), h('span', 'td-cockpit'), h('span', 'td-wing rear'));
    const note = h('div', 'td-note dim');
    root.append(head, car, note);

    const wheels = CORNERS.map((name, i) => {
      const right = i % 2 === 1;
      const side = right ? 'right' : 'left';
      const wheel = h('div', `td-wheel ${side}`);
      const tyre = h('div', 'td-tyre');
      const flat = h('span', 'td-flat', 'FLAT');
      tyre.append(flat);
      const disc = h('div', 'td-disc');
      wheel.append(...(right ? [disc, tyre] : [tyre, disc]));
      const info = h('div', `td-info ${side}`);
      const corner = h('span', 'td-corner', name);
      const temps = h('div', 'td-temps');
      const cells = [0, 1, 2].map(() => temps.appendChild(h('span', 'td-t')));
      const press = h('div', 'td-press');
      const pVal = h('span', 'td-pv');
      const pUnit = h('span', 'td-pu');
      press.append(pVal, pUnit);
      const wear = h('div', 'td-wear');
      const wearBar = h('span', 'td-wearbar');
      const wearVal = h('span', 'td-wv');
      wear.append(wearBar, wearVal);
      const extra = h('div', 'td-extra');
      const carcass = h('span', 'td-x td-carcass');
      const brake = h('span', 'td-x td-brake');
      extra.append(carcass, brake);
      info.append(corner, temps, press, wear, extra);
      show(temps, s.showTemps !== false);
      show(press, s.showPressure !== false);
      show(wear, s.showWear !== false);
      return { info, wheel, tyre, flat, disc, cells, pVal, pUnit, wear, wearBar, wearVal, extra, carcass, brake };
    });
    // Grid: info | wheel | body | wheel | info, front axle then rear.
    const [fl, fr, rl, rr] = wheels;
    car.append(fl.info, fl.wheel, body, fr.wheel, fr.info, rl.info, rl.wheel, rr.wheel, rr.info);

    /** cold / ok / hot, blended near the window's edges so a tyre coming in shows as it warms. */
    const tone = (v: number, lo: number, hi: number) => {
      const band = Math.max(4, (hi - lo) * 0.2);
      const mix = (c: string, k: number) => (k >= 1 ? c : `color-mix(in srgb, ${c} ${Math.round(k * 100)}%, var(--td-ok))`);
      if (v < lo) return mix('var(--td-cold)', Math.min(1, (lo - v) / band));
      if (v > hi) return mix('var(--td-hot)', Math.min(1, (v - hi) / band));
      return 'var(--td-ok)';
    };

    let editing = false;
    let live: TyreData | null = null;
    const preview = new URLSearchParams(location.search).has('preview');
    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        else if (topic === 'tyredata') live = data as TyreData;
        else return;
        const d = (editing || preview) && !live?.wheels ? SAMPLE : live;
        if (!d) return;
        const u = ctx.units;
        text(tag, d.wheels ? (d.live ? 'LIVE' : 'LAST STOP') : '');
        toggle(tag, 'stop', !d.live);
        const opt = s.useGameWindow !== false && d.optimal != null ? d.optimal : null;
        text(ideal, opt == null ? '' : `IDEAL ${temp(opt, u)}`);
        const lo = opt != null ? opt - (s.windowMargin as number) : (s.coldBelow as number);
        const hi = opt != null ? opt + (s.windowMargin as number) : (s.hotAbove as number);
        const measured = !!d.wheels?.some((wh) => wh.temps || wh.pressure != null || wh.wear != null);
        show(car, !!d.wheels);
        show(note, !measured);
        text(note, d.game === 'iracing' ? 'iRacing measures the tyres at a pit stop: readings show after your first stop.' : 'No tyre readings yet.');
        if (!d.wheels) return;
        d.wheels.forEach((wh, i) => {
          const v = wheels[i];
          show(v.flat, wh.flat);
          toggle(v.wheel, 'flat', wh.flat);
          // The tyre shaded across its width from the three temperatures (or the carcass's).
          const t = wh.temps ?? (wh.carcass != null ? [wh.carcass, wh.carcass, wh.carcass] : null);
          toggle(v.tyre, 'unknown', !t);
          v.cells.forEach((c, k) => {
            const tv = wh.temps?.[k] ?? null;
            text(c, tv == null ? '–' : temp(tv, u));
            style(c, 'color', tv == null ? '' : tone(tv, lo, hi));
            style(v.tyre, `--t${k}`, t ? tone(t[k], lo, hi) : '');
          });
          if (wh.pressure == null) {
            text(v.pVal, '–');
            text(v.pUnit, '');
          } else {
            const [val, unit] = pressure(wh.pressure, s.pressureUnit as string, u).split(' ');
            text(v.pVal, val);
            text(v.pUnit, unit);
          }
          const wear = wh.wear;
          text(v.wearVal, wear == null ? '–' : `${Math.round(wear)}%`);
          style(v.wearBar, '--p', String(wear == null ? 0 : Math.max(0, Math.min(1, wear / 100))));
          toggle(v.wear, 'warn', wear != null && wear < (s.wearWarn as number) && wear >= (s.wearBad as number));
          toggle(v.wear, 'bad', wear != null && wear < (s.wearBad as number));
          show(v.carcass, !!s.showCarcass && wh.carcass != null);
          text(v.carcass, wh.carcass == null ? '' : `CORE ${temp(wh.carcass, u)}`);
          style(v.carcass, 'color', wh.carcass == null ? '' : tone(wh.carcass, lo, hi));
          const brakes = s.showBrakes !== false && wh.brake != null;
          show(v.brake, brakes);
          show(v.disc, brakes);
          const bc = wh.brake == null ? '' : tone(wh.brake, s.brakeCold as number, s.brakeHot as number);
          text(v.brake, wh.brake == null ? '' : `BRK ${temp(wh.brake, u)}`);
          style(v.brake, 'color', bc);
          style(v.disc, '--bc', bc);
          show(v.extra, brakes || (!!s.showCarcass && wh.carcass != null));
        });
      },
    };
  },
};
