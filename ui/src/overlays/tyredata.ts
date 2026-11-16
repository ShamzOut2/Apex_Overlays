// Your tyres and brakes, a card per wheel: the temperatures across the tyre as tall cells,
// coloured cold (blue), in the window (green), hot (amber) or critical (red-orange); the
// pressure large; tread left and brake temperature; and a pill naming the worst problem.
// Live in Le Mans Ultimate; iRacing only measures tyres at a pit stop, so there it shows the
// readings from your last stop.

import { h, show, style, text, toggle } from '../lib/dom';
import type { Units } from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Status, TyreData, WheelData } from '../lib/types';

const w = (t: [number, number, number], carcass: number, pressure: number, wear: number, brake: number): WheelData => ({ temps: t, carcass, pressure, wear, brake, flat: false });

/** Shown while placing the overlay: mid-stint, the front-right's inside edge running hot. */
const SAMPLE: TyreData = {
  game: 'lmu',
  driving: true,
  live: true,
  optimal: 88,
  wheels: [w([82, 86, 91], 84, 183, 87, 580), w([103, 98, 94], 92, 183, 87, 580), w([79, 83, 88], 80, 181, 90, 520), w([100, 95, 91], 90, 181, 90, 520)],
};

const CORNERS = ['FL', 'FR', 'RL', 'RR'];

type Tone = 'cold' | 'ok' | 'hot' | 'crit';
const RANK: Record<Tone, number> = { ok: 0, cold: 1, hot: 2, crit: 3 };

const temp = (c: number, u: Units) => String(Math.round(u === 'imperial' ? (c * 9) / 5 + 32 : c));

function pressureUnit(setting: string, u: Units): 'kpa' | 'psi' | 'bar' {
  return setting === 'auto' ? (u === 'imperial' ? 'psi' : 'kpa') : (setting as 'kpa' | 'psi' | 'bar');
}
function pressure(kpa: number, unit: 'kpa' | 'psi' | 'bar'): string {
  if (unit === 'psi') return (kpa * 0.145038).toFixed(1);
  if (unit === 'bar') return (kpa / 100).toFixed(2);
  return kpa.toFixed(0);
}

/** Where across the tyre cell `k` is (cells run left to right as seen from above). */
const edge = (corner: number, k: number) => (k === 1 ? 'MIDDLE' : (corner % 2 === 0) === (k === 0) ? 'OUTER' : 'INNER');

export const tyredata: OverlayDef = {
  kind: 'tyredata',
  name: 'Tyre data',
  description:
    'A card per wheel: the temperatures across each tyre (blue when cold, green in the window, amber when hot, red-orange when critical), the pressure, tread left and brake temperature, with a pill naming the worst problem. Live in Le Mans Ultimate; iRacing only measures tyres at pit stops, so there it shows the readings from your last stop.',
  topics: ['tyredata', 'status'],
  size: [470, 280],
  commonDefaults: { autoHeight: true, hideWhenNotDriving: true },
  settings: [
    { key: 'showPressure', label: 'Pressure', type: 'bool', default: true, group: 'What to show', help: 'iRacing: the cold pressure set at your last stop' },
    { key: 'showWear', label: 'Tread left', type: 'bool', default: true, group: 'What to show' },
    { key: 'showWearBar', label: 'Tread bar under the tyre', type: 'bool', default: true, group: 'What to show' },
    { key: 'showBrakes', label: 'Brake temperatures', type: 'bool', default: true, group: 'What to show', lmu: true, help: 'iRacing does not share brake temperatures' },
    { key: 'showCarcass', label: 'Carcass temperature', type: 'bool', default: false, group: 'What to show', lmu: true, help: 'The tyre’s core, slower to change than the surface' },
    { key: 'showAlert', label: 'Pill naming the worst problem', type: 'bool', default: true, group: 'What to show', help: 'e.g. FR INNER 103°, TYRES COLD, RL TREAD 14%' },
    { key: 'showUnits', label: 'Units in the header', type: 'bool', default: true, group: 'What to show' },
    { key: 'pressureUnit', when: (v) => v.showPressure !== false, label: 'Pressure in', type: 'select', default: 'auto', options: [['auto', 'kPa or psi (follows the units setting)'], ['kpa', 'kPa'], ['psi', 'psi'], ['bar', 'bar']], group: 'What to show' },
    { key: 'condensed', label: 'Numbers in a condensed font (Barlow Condensed)', type: 'bool', default: true, group: 'Look', help: 'Off: the theme’s overlay font' },
    { key: 'useGameWindow', label: 'Use the game’s ideal tyre temperature', type: 'bool', default: true, group: 'Tyre window', lmu: true, help: 'Le Mans Ultimate says each tyre’s ideal temperature; green is that ± the margin below' },
    { key: 'windowMargin', when: (v) => v.useGameWindow !== false, label: 'Either side of the ideal', type: 'number', default: 12, min: 2, max: 40, unit: '°C', group: 'Tyre window', lmu: true },
    { key: 'coldBelow', label: 'Cold below', type: 'number', default: 70, min: 0, max: 150, unit: '°C', group: 'Tyre window', help: 'Used when the game gives no ideal temperature' },
    { key: 'hotAbove', label: 'Hot above', type: 'number', default: 100, min: 20, max: 200, unit: '°C', group: 'Tyre window', help: 'Used when the game gives no ideal temperature' },
    { key: 'critMargin', label: 'Critical when hotter than the window by', type: 'number', default: 12, min: 1, max: 60, unit: '°C', group: 'Tyre window' },
    { key: 'brakeCold', when: (v) => v.showBrakes !== false, label: 'Brakes cold below', type: 'number', default: 300, min: 0, max: 1000, step: 10, unit: '°C', group: 'Brakes', lmu: true },
    { key: 'brakeHot', when: (v) => v.showBrakes !== false, label: 'Brakes hot above', type: 'number', default: 700, min: 100, max: 1500, step: 10, unit: '°C', group: 'Brakes', lmu: true },
    { key: 'brakeCrit', when: (v) => v.showBrakes !== false, label: 'Brakes critical above', type: 'number', default: 850, min: 100, max: 1600, step: 10, unit: '°C', group: 'Brakes', lmu: true },
    { key: 'wearWarn', when: (v) => v.showWear !== false || v.showWearBar !== false, label: 'Tread left in amber below', type: 'number', default: 40, min: 0, max: 100, unit: '%', group: 'Tread' },
    { key: 'wearBad', when: (v) => v.showWear !== false || v.showWearBar !== false, label: 'Tread left in red below', type: 'number', default: 20, min: 0, max: 100, unit: '%', group: 'Tread' },
    { key: 'coldColor', label: 'Cold', type: 'color', default: '#6e9dff', group: 'Colours' },
    { key: 'okColor', label: 'In the window', type: 'color', default: '#3fd47f', group: 'Colours' },
    { key: 'hotColor', label: 'Hot', type: 'color', default: '#ffb224', group: 'Colours' },
    { key: 'critColor', label: 'Critical', type: 'color', default: '#ff6a4d', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('tyredata');
    toggle(root, 'td-condensed', s.condensed !== false);
    for (const t of ['cold', 'ok', 'hot', 'crit']) style(root, `--td-${t}`, s[`${t}Color`] as string);

    const head = h('div', 'td-head');
    const units = h('span', 'td-units');
    const left = h('span', 'td-head-l');
    left.append(h('span', 'td-title', 'TYRES'), units);
    const pill = h('span', 'td-pill');
    head.append(left, pill);
    const grid = h('div', 'td-grid');
    const note = h('div', 'td-note dim');
    root.append(head, grid, note);
    show(units, s.showUnits !== false);

    const row = (label: string) => {
      const el = h('div', 'td-row');
      const v = h('span', 'td-rv');
      el.append(h('span', 'td-rk', label), v);
      return { el, v };
    };
    const wheels = CORNERS.map((name, i) => {
      const right = i % 2 === 1;
      const card = h('div', `td-card ${right ? 'right' : 'left'}`);
      const tyre = h('div', 'td-tyre');
      const strip = h('div', 'td-strip');
      const cells = [0, 1, 2].map(() => strip.appendChild(h('span', 'td-cell')));
      const bar = h('div', 'td-bar');
      const barFill = h('span', 'td-bar-fill');
      bar.append(barFill);
      tyre.append(strip, bar);
      const info = h('div', 'td-info');
      const flat = h('span', 'td-flat', 'FLAT');
      const top = h('div', 'td-top');
      top.append(h('span', 'td-corner', name), flat);
      const press = h('span', 'td-press');
      const tread = row('TREAD');
      const brake = row('BRAKE');
      const core = row('CORE');
      info.append(top, press, tread.el, brake.el, core.el);
      card.append(...(right ? [info, tyre] : [tyre, info]));
      grid.append(card);
      show(press, s.showPressure !== false);
      show(tread.el, s.showWear !== false);
      show(bar, s.showWearBar !== false);
      return { card, cells, barFill, flat, press, tread, brake, core };
    });
    // Nothing beside the tyres: just the tyres.
    toggle(root, 'td-compact', s.showPressure === false && s.showWear === false && s.showBrakes === false && !s.showCarcass);

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
        const pu = pressureUnit(s.pressureUnit as string, u);
        text(units, `${u === 'imperial' ? '°F' : '°C'}${s.showPressure !== false ? ` · ${pu === 'kpa' ? 'kPa' : pu}` : ''}${d.wheels && !d.live ? ' · LAST STOP' : ''}`);
        const opt = s.useGameWindow !== false && d.optimal != null ? d.optimal : null;
        const lo = opt != null ? opt - (s.windowMargin as number) : (s.coldBelow as number);
        const hi = opt != null ? opt + (s.windowMargin as number) : (s.hotAbove as number);
        const crit = hi + (s.critMargin as number);
        const tyreTone = (v: number): Tone => (v > crit ? 'crit' : v > hi ? 'hot' : v < lo ? 'cold' : 'ok');
        const brakeTone = (v: number): Tone => (v > (s.brakeCrit as number) ? 'crit' : v > (s.brakeHot as number) ? 'hot' : v < (s.brakeCold as number) ? 'cold' : 'ok');
        const measured = !!d.wheels?.some((wh) => wh.temps || wh.pressure != null || wh.wear != null);
        show(grid, !!d.wheels);
        show(note, !measured);
        text(note, d.game === 'iracing' ? 'iRacing measures the tyres at a pit stop: readings show after your first stop.' : 'No tyre readings yet.');
        if (!d.wheels) {
          show(pill, false);
          return;
        }

        // The worst problem, for the pill: a flat, then the hottest critical or hot edge,
        // brake or low tread, then cold tyres.
        let alert: { rank: number; tone: Tone; label: string } | null = null;
        const consider = (rank: number, tone: Tone, label: string) => {
          if (!alert || rank > alert.rank) alert = { rank, tone, label };
        };
        let coldTyres = 0;
        let coldOne = -1;
        d.wheels.forEach((wh, i) => {
          const v = wheels[i];
          show(v.flat, wh.flat);
          toggle(v.card, 'flat', wh.flat);
          if (wh.flat) consider(100, 'crit', `${CORNERS[i]} FLAT`);
          let allCold = !!wh.temps;
          v.cells.forEach((c, k) => {
            const t = wh.temps?.[k] ?? null;
            text(c, t == null ? '–' : temp(t, u));
            const tone = t == null ? null : tyreTone(t);
            c.className = `td-cell t-${tone ?? 'none'}`;
            if (tone !== 'cold') allCold = false;
            if (t != null && (tone === 'hot' || tone === 'crit')) consider(RANK[tone] * 10 + t / 1000, tone, `${CORNERS[i]} ${edge(i, k)} ${temp(t, u)}°`);
          });
          if (allCold) {
            coldTyres++;
            if (coldOne < 0) coldOne = i;
          }
          text(v.press, wh.pressure == null ? '–' : pressure(wh.pressure, pu));
          const wear = wh.wear;
          const wTone: Tone = wear == null ? 'ok' : wear < (s.wearBad as number) ? 'crit' : wear < (s.wearWarn as number) ? 'hot' : 'ok';
          text(v.tread.v, wear == null ? '–' : `${Math.round(wear)}%`);
          v.tread.v.className = `td-rv w-${wTone}`;
          style(v.barFill, 'width', `${wear == null ? 0 : Math.max(0, Math.min(100, wear))}%`);
          v.barFill.className = `td-bar-fill w-${wTone}`;
          if (wear != null && wTone !== 'ok') consider(RANK[wTone] * 10 - 5 + (100 - wear) / 1000, wTone, `${CORNERS[i]} TREAD ${Math.round(wear)}%`);
          show(v.brake.el, s.showBrakes !== false && wh.brake != null);
          if (wh.brake != null) {
            const bt = brakeTone(wh.brake);
            text(v.brake.v, `${temp(wh.brake, u)}°`);
            v.brake.v.className = `td-rv b-${bt}`;
            if (s.showBrakes !== false && (bt === 'hot' || bt === 'crit')) consider(RANK[bt] * 10 - 2 + wh.brake / 10000, bt, `${CORNERS[i]} BRAKE ${temp(wh.brake, u)}°`);
          }
          show(v.core.el, !!s.showCarcass && wh.carcass != null);
          if (wh.carcass != null) {
            text(v.core.v, `${temp(wh.carcass, u)}°`);
            v.core.v.className = `td-rv b-${tyreTone(wh.carcass)}`;
          }
        });
        if (coldTyres >= 3) consider(RANK.cold * 10, 'cold', 'TYRES COLD');
        else if (coldOne >= 0) consider(RANK.cold * 10 - 1, 'cold', `${CORNERS[coldOne]} COLD`);
        const a = alert as { tone: Tone; label: string } | null;
        show(pill, s.showAlert !== false && !!a);
        if (a) {
          text(pill, a.label);
          pill.className = `td-pill p-${a.tone}`;
        }
      },
    };
  },
};
