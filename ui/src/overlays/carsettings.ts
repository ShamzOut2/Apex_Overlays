// The settings you change from the wheel (brake bias, ABS, traction control, engine map...),
// with a big pop-up whenever one changes, so a click of the rotary is easy to check mid-corner.

import { h, show, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns } from '../lib/settings';
import type { CarSetting, CarSettings, Status } from '../lib/types';

const LMU_ONLY = 'Le Mans Ultimate only';

/** Every setting either game reports: short label on the strip, full name on the pop-up. */
const ITEMS: { id: string; short: string; long: string; lmu?: string }[] = [
  { id: 'brakeBias', short: 'BB', long: 'Brake bias' },
  { id: 'peakBias', short: 'PEAK BB', long: 'Peak brake bias' },
  { id: 'migration', short: 'MIGR', long: 'Brake migration', lmu: LMU_ONLY },
  { id: 'abs', short: 'ABS', long: 'ABS' },
  { id: 'tc', short: 'TC', long: 'Traction control' },
  { id: 'tc2', short: 'TC2', long: 'Traction control 2' },
  { id: 'tcSlip', short: 'TC SLIP', long: 'TC slip', lmu: LMU_ONLY },
  { id: 'tcCut', short: 'TC CUT', long: 'TC power cut', lmu: LMU_ONLY },
  { id: 'engineMap', short: 'MAP', long: 'Engine map', lmu: LMU_ONLY },
  { id: 'fuelMix', short: 'MIX', long: 'Fuel mixture' },
  { id: 'throttleMap', short: 'THR', long: 'Throttle map' },
  { id: 'enginePower', short: 'PWR', long: 'Engine power' },
  { id: 'engineBraking', short: 'EB', long: 'Engine braking' },
  { id: 'boost', short: 'BOOST', long: 'Boost' },
  { id: 'deployMode', short: 'DEPLOY', long: 'Hybrid deploy' },
  { id: 'regenGain', short: 'REGEN', long: 'Hybrid regen' },
  { id: 'arbFront', short: 'ARB F', long: 'Front anti-roll bar' },
  { id: 'arbRear', short: 'ARB R', long: 'Rear anti-roll bar' },
  { id: 'weightJacker', short: 'JACK', long: 'Weight jacker' },
  { id: 'diffEntry', short: 'DIFF EN', long: 'Diff entry' },
  { id: 'diffMiddle', short: 'DIFF MID', long: 'Diff middle' },
  { id: 'diffExit', short: 'DIFF EX', long: 'Diff exit' },
];
const byId = new Map(ITEMS.map((i) => [i.id, i]));

/** Shown while placing the overlay: a GT3 car. */
const SAMPLE: CarSettings = {
  game: 'lmu',
  driving: true,
  items: [
    { id: 'brakeBias', value: 54.6, max: null },
    { id: 'abs', value: 3, max: 11 },
    { id: 'tc', value: 4, max: 11 },
    { id: 'tcCut', value: 2, max: 8 },
    { id: 'engineMap', value: 1, max: 5 },
  ],
};

const isBias = (id: string) => id === 'brakeBias' || id === 'peakBias';
function value(s: CarSetting): string {
  if (isBias(s.id)) return `${(s.value < 1 ? s.value * 100 : s.value).toFixed(1)}%`;
  return Number.isInteger(s.value) ? String(s.value) : s.value.toFixed(2).replace(/0$/, '');
}

export const carsettings: OverlayDef = {
  kind: 'carsettings',
  name: 'Car settings',
  description:
    'The settings you change from the wheel: brake bias, ABS, traction control, engine map and whatever else your car has. A big pop-up shows the new value whenever you change one, or use it as pop-ups only. Shows the settings each car actually has.',
  topics: ['carsettings', 'status'],
  size: [400, 64],
  commonDefaults: { autoWidth: true, autoHeight: true, hideWhenNotDriving: true },
  settings: [
    {
      key: 'items',
      label: 'Settings shown, in order',
      type: 'columns',
      ...columns(
        ITEMS.map((i) => ({ id: i.id, label: `${i.long} — ${i.short}`, lmu: i.lmu })),
        ['brakeBias', 'abs', 'tc', 'tc2', 'tcSlip', 'tcCut', 'engineMap', 'fuelMix', 'deployMode'],
      ),
      group: 'What to show',
      help: 'Only those your car has are shown',
    },
    { key: 'showMax', label: 'Number of steps (4/11)', type: 'bool', default: false, group: 'What to show', lmu: true, help: 'Le Mans Ultimate says how many steps each setting has' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'row', options: [['row', 'In a row'], ['column', 'In a column']], group: 'Layout' },
    {
      key: 'popup',
      label: 'When a setting changes',
      type: 'select',
      default: 'over',
      options: [
        ['over', 'Pop up over the settings'],
        ['only', 'Pop-ups only (hidden until something changes)'],
        ['flash', 'Just highlight it'],
      ],
      group: 'Pop-up',
    },
    { key: 'popupSeconds', when: (v) => v.popup !== 'flash', label: 'Pop-up shows for', type: 'number', default: 1.8, min: 0.5, max: 6, step: 0.1, unit: 's', group: 'Pop-up' },
    { key: 'highlight', label: 'Changed setting', type: 'color', default: '#ffd84d', group: 'Pop-up' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('carsettings', `cs-${s.layout === 'column' ? 'column' : 'row'}`, `cs-pop-${(s.popup as string) || 'over'}`);
    style(root, '--cs-hi', s.highlight as string);
    const popupMode = (s.popup as string) || 'over';
    const strip = h('div', 'cs-strip');
    const pop = h('div', 'cs-pop');
    const popName = h('span', 'cs-pop-name');
    const popVal = h('span', 'cs-pop-val');
    const popMax = h('span', 'cs-pop-max');
    pop.append(popName, popVal, popMax);
    const empty = h('div', 'cs-empty dim', 'This car has none of the chosen settings');
    root.append(strip, pop, empty);
    show(pop, false);
    show(empty, false);
    if (popupMode === 'only') show(strip, false);

    const order = enabledColumns(s.items);
    const cells = new Map<string, { el: HTMLElement; val: HTMLElement; max: HTMLElement }>();
    for (const id of order) {
      const def = byId.get(id);
      if (!def) continue;
      const el = h('div', 'cs-item');
      const val = h('span', 'cs-val');
      const max = h('span', 'cs-max');
      el.append(h('span', 'cs-label', def.short), val, max);
      show(el, false);
      strip.append(el);
      cells.set(id, { el, val, max });
    }

    let editing = false;
    let live: CarSettings | null = null;
    let prev = new Map<string, number>();
    let carKey = '';
    let popTimer = 0;
    let popping = false;
    const flashTimers = new Map<string, number>();
    const preview = new URLSearchParams(location.search).has('preview');

    const visibility = () => {
      const sample = editing || preview;
      const any = [...cells.values()].some((c) => !c.el.hidden);
      show(empty, !any && sample);
      // Pop-ups only: nothing on screen between changes (except while placing it).
      toggle(root, 'auto-hidden', popupMode === 'only' && !popping && !sample);
      show(pop, popping || (popupMode === 'only' && sample));
    };

    const popup = (it: CarSetting) => {
      const def = byId.get(it.id);
      text(popName, def?.long ?? it.id);
      text(popVal, value(it));
      text(popMax, s.showMax && it.max != null ? `/ ${it.max}` : '');
      popping = true;
      root.classList.add('cs-popping');
      clearTimeout(popTimer);
      popTimer = window.setTimeout(() => {
        popping = false;
        root.classList.remove('cs-popping');
        visibility();
      }, (s.popupSeconds as number) * 1000);
      visibility();
    };

    const flash = (id: string) => {
      const c = cells.get(id);
      if (!c) return;
      c.el.classList.add('changed');
      clearTimeout(flashTimers.get(id));
      flashTimers.set(id, window.setTimeout(() => c.el.classList.remove('changed'), Math.max(1200, (s.popupSeconds as number) * 1000)));
    };

    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        else if (topic === 'carsettings') live = data as CarSettings;
        else return;
        const d = (editing || preview) && !live?.items.length ? SAMPLE : live;
        if (!d) return;
        // A different car (or getting in): what it has now is not a change.
        const key = d.items.map((i) => i.id).join(',');
        const fresh = key !== carKey || !d.driving;
        carKey = key;
        const now = new Map(d.items.map((i) => [i.id, i.value]));
        for (const [id, c] of cells) {
          const it = d.items.find((i) => i.id === id);
          show(c.el, !!it);
          if (!it) continue;
          text(c.val, value(it));
          text(c.max, s.showMax && it.max != null ? `/${it.max}` : '');
        }
        if (!fresh && d !== SAMPLE) {
          for (const it of d.items) {
            const was = prev.get(it.id);
            if (was === undefined || was === it.value || !cells.has(it.id)) continue;
            flash(it.id);
            if (popupMode !== 'flash') popup(it);
          }
        }
        prev = now;
        if (d === SAMPLE && popupMode === 'only') {
          text(popName, 'Traction control');
          text(popVal, '4');
          text(popMax, s.showMax ? '/ 11' : '');
        }
        visibility();
      },
    };
  },
};
