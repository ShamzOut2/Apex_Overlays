// The settings you change from the wheel (brake bias, ABS, traction control, engine map...),
// each on a card coloured by kind with a meter of where it sits, and a big pop-up whenever one
// changes, so a click of the rotary is easy to check mid-corner.

import { h, show, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns } from '../lib/settings';
import type { CarSetting, CarSettings, Status } from '../lib/types';

const LMU_ONLY = 'Le Mans Ultimate only';

type Kind = 'brakes' | 'abs' | 'tc' | 'engine' | 'hybrid' | 'chassis';

/** Every setting either game reports: short label on the card, full name on the pop-up. */
const ITEMS: { id: string; short: string; long: string; kind: Kind; lmu?: string }[] = [
  { id: 'brakeBias', short: 'BB', long: 'Brake bias', kind: 'brakes' },
  { id: 'peakBias', short: 'PEAK BB', long: 'Peak brake bias', kind: 'brakes' },
  { id: 'migration', short: 'MIGR', long: 'Brake migration', kind: 'brakes', lmu: LMU_ONLY },
  { id: 'abs', short: 'ABS', long: 'ABS', kind: 'abs' },
  { id: 'tc', short: 'TC', long: 'Traction control', kind: 'tc' },
  { id: 'tc2', short: 'TC2', long: 'Traction control 2', kind: 'tc' },
  { id: 'tcSlip', short: 'TC SLIP', long: 'TC slip', kind: 'tc', lmu: LMU_ONLY },
  { id: 'tcCut', short: 'TC CUT', long: 'TC power cut', kind: 'tc', lmu: LMU_ONLY },
  { id: 'engineMap', short: 'MAP', long: 'Engine map', kind: 'engine', lmu: LMU_ONLY },
  { id: 'fuelMix', short: 'MIX', long: 'Fuel mixture', kind: 'engine' },
  { id: 'throttleMap', short: 'THR', long: 'Throttle map', kind: 'engine' },
  { id: 'enginePower', short: 'PWR', long: 'Engine power', kind: 'engine' },
  { id: 'engineBraking', short: 'EB', long: 'Engine braking', kind: 'engine' },
  { id: 'boost', short: 'BOOST', long: 'Boost', kind: 'engine' },
  { id: 'deployMode', short: 'DEPLOY', long: 'Hybrid deploy', kind: 'hybrid' },
  { id: 'regenGain', short: 'REGEN', long: 'Hybrid regen', kind: 'hybrid' },
  { id: 'arbFront', short: 'ARB F', long: 'Front anti-roll bar', kind: 'chassis' },
  { id: 'arbRear', short: 'ARB R', long: 'Rear anti-roll bar', kind: 'chassis' },
  { id: 'weightJacker', short: 'JACK', long: 'Weight jacker', kind: 'chassis' },
  { id: 'diffEntry', short: 'DIFF EN', long: 'Diff entry', kind: 'chassis' },
  { id: 'diffMiddle', short: 'DIFF MID', long: 'Diff middle', kind: 'chassis' },
  { id: 'diffExit', short: 'DIFF EX', long: 'Diff exit', kind: 'chassis' },
];
const byId = new Map(ITEMS.map((i) => [i.id, i]));

const KINDS: [Kind, string, string][] = [
  ['brakes', 'Brake bias and migration', '#ff6b4a'],
  ['abs', 'ABS', '#ffc23d'],
  ['tc', 'Traction control', '#38c8ff'],
  ['engine', 'Engine map, mixture, power', '#55e08a'],
  ['hybrid', 'Hybrid deploy and regen', '#b47cff'],
  ['chassis', 'Anti-roll bars, diff, jacker', '#c9d1dc'],
];

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
const biasPct = (v: number) => (v < 1 ? v * 100 : v);
function value(s: CarSetting): string {
  if (isBias(s.id)) return biasPct(s.value).toFixed(1);
  return Number.isInteger(s.value) ? String(s.value) : s.value.toFixed(2).replace(/0$/, '');
}

/** A meter for a setting: step pips when the game says how many steps (up to 14), a fill
 *  otherwise, and for brake bias a front/rear split with the middle marked. */
function meter() {
  const el = h('div', 'cs-meter');
  let pips: HTMLElement[] = [];
  let mode = '';
  return {
    el,
    set(it: CarSetting) {
      const m = isBias(it.id) ? 'bias' : it.max != null && it.max > 0 && it.max <= 14 ? `pips${it.max}` : it.max != null && it.max > 0 ? 'fill' : 'none';
      if (m !== mode) {
        mode = m;
        el.className = `cs-meter m-${m.replace(/\d+/, '')}`;
        el.replaceChildren();
        pips = [];
        if (m.startsWith('pips')) {
          // Steps from 0 or 1 to max: one pip per step above the lowest.
          for (let i = 0; i < (it.max as number); i++) pips.push(el.appendChild(h('span', 'cs-pip')));
        } else if (m !== 'none') {
          el.append(h('span', 'cs-fill'));
          if (m === 'bias') el.append(h('span', 'cs-mid'));
        }
      }
      // Bias: 35% to 65% front across the bar (50/50 marked in the middle), so a half-percent click shows.
      if (m === 'bias') style(el, '--p', String(Math.max(0, Math.min(1, (biasPct(it.value) - 35) / 30))));
      else if (m === 'fill') style(el, '--p', String(Math.max(0, Math.min(1, it.value / (it.max as number)))));
      else pips.forEach((p, i) => toggle(p, 'on', i < it.value));
    },
  };
}

export const carsettings: OverlayDef = {
  kind: 'carsettings',
  name: 'Car settings',
  description:
    'Brake bias, ABS, TC, engine map and whatever else your car has, each with a meter. A big pop-up shows the new value whenever you change one.',
  topics: ['carsettings', 'status'],
  size: [420, 72],
  fitsWidth: true,
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
    { key: 'showMeter', label: 'Meter under each setting', type: 'bool', default: true, group: 'What to show', help: 'Steps (Le Mans Ultimate says how many each setting has) and brake bias front / rear' },
    { key: 'showMax', label: 'Number of steps (4/11)', type: 'bool', default: false, group: 'What to show', lmu: true, help: 'Le Mans Ultimate says how many steps each setting has' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'row', options: [['row', 'Cards in a row'], ['column', 'Rows in a column']], group: 'Layout' },
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
    { key: 'showPrevious', when: (v) => v.popup !== 'flash', label: 'The value before the change', type: 'bool', default: true, group: 'Pop-up' },
    { key: 'kindColors', label: 'A colour for each kind of setting', type: 'bool', default: true, group: 'Colours', help: 'Off: every setting in the colour below' },
    { key: 'color', when: (v) => v.kindColors === false, label: 'Settings', type: 'color', default: '#ffd84d', group: 'Colours' },
    ...KINDS.map(([kind, label, color]) => ({ key: `${kind}Color`, when: (v: Record<string, unknown>) => v.kindColors !== false, label, type: 'color' as const, default: color, group: 'Colours' })),
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const popupMode = (s.popup as string) || 'over';
    root.classList.add('carsettings', `cs-${s.layout === 'column' ? 'column' : 'row'}`, `cs-pop-${popupMode}`);
    toggle(root, 'no-meter', s.showMeter === false);
    const colorOf = (kind: Kind) => (s.kindColors === false ? (s.color as string) : (s[`${kind}Color`] as string));

    const strip = h('div', 'cs-strip');
    const pop = h('div', 'cs-pop');
    const popName = h('span', 'cs-pop-name');
    const popVal = h('span', 'cs-pop-val');
    const popUnit = h('span', 'cs-pop-unit');
    const popWas = h('span', 'cs-pop-was');
    const popMeter = meter();
    const popText = h('div', 'cs-pop-text');
    const popLine = h('div', 'cs-pop-line');
    popLine.append(popVal, popUnit, popWas);
    popText.append(popName, popLine);
    pop.append(popText, popMeter.el);
    const empty = h('div', 'cs-empty dim', 'This car has none of the chosen settings');
    root.append(strip, pop, empty);
    show(pop, false);
    show(empty, false);
    if (popupMode === 'only') show(strip, false);

    const cells = new Map<string, { el: HTMLElement; val: HTMLElement; unit: HTMLElement; meter: ReturnType<typeof meter> }>();
    for (const id of enabledColumns(s.items)) {
      const def = byId.get(id);
      if (!def) continue;
      const el = h('div', `cs-item k-${def.kind}`);
      style(el, '--k', colorOf(def.kind));
      const val = h('span', 'cs-val');
      const unit = h('span', 'cs-unit');
      const line = h('div', 'cs-line');
      line.append(val, unit);
      const m = meter();
      el.append(h('span', 'cs-label', def.short), line, m.el);
      show(el, false);
      strip.append(el);
      cells.set(id, { el, val, unit, meter: m });
    }

    let editing = false;
    let live: CarSettings | null = null;
    let prev = new Map<string, number>();
    let carKey = '';
    let popTimer = 0;
    let popping = false;
    const flashTimers = new Map<string, number>();
    const preview = new URLSearchParams(location.search).has('preview');

    const unitOf = (it: CarSetting) => (isBias(it.id) ? '%' : s.showMax && it.max != null ? `/${it.max}` : '');

    const visibility = () => {
      const sample = editing || preview;
      const any = [...cells.values()].some((c) => !c.el.hidden);
      show(empty, !any && sample);
      // Pop-ups only: nothing on screen between changes (except while placing it).
      toggle(root, 'auto-hidden', popupMode === 'only' && !popping && !sample);
      show(pop, popping || (popupMode === 'only' && sample));
    };

    const fillPop = (it: CarSetting, was: number | null) => {
      const def = byId.get(it.id);
      style(pop, '--k', colorOf(def?.kind ?? 'chassis'));
      text(popName, def?.long ?? it.id);
      text(popVal, value(it));
      text(popUnit, unitOf(it));
      const before = was == null || s.showPrevious === false ? '' : value({ ...it, value: was });
      text(popWas, before ? `${was! < it.value ? '▲' : '▼'} from ${before}` : '');
      popMeter.set(it);
    };

    /**
     * The pop-up fills the overlay, whatever its size: its text is scaled up or down so the
     * name, value and meter take the room there is. Measured at the normal size (what the
     * content needs), then scaled by how much room there is, and checked once more so it
     * never spills over. With the window fitted to the pop-up itself (pop-ups only, Fit the
     * window to the content), the window follows the text, so it is only scaled down there.
     */
    const fitPop = () => {
      if (pop.hidden) return;
      pop.style.fontSize = '';
      const cs = getComputedStyle(pop);
      const padX = parseFloat(cs.paddingLeft) + parseFloat(cs.paddingRight);
      const padY = parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom);
      const gap = parseFloat(cs.columnGap) || 0;
      const roomW = pop.clientWidth - padX;
      const roomH = pop.clientHeight - padY;
      const meterOn = !popMeter.el.hidden && getComputedStyle(popMeter.el).display !== 'none';
      const meterW = meterOn ? parseFloat(getComputedStyle(popMeter.el).minWidth) || 0 : 0;
      const needW = popText.offsetWidth + (meterOn ? meterW + gap : 0);
      const needH = popText.offsetHeight;
      if (roomW <= 0 || roomH <= 0 || needW <= 0 || needH <= 0) return;
      const fitted = root.classList.contains('auto-height') || root.classList.contains('auto-width');
      const most = popupMode === 'only' && fitted ? 1 : 12;
      let k = Math.min(most, roomW / needW, roomH / needH) * 0.94;
      pop.style.fontSize = `${k * 100}%`;
      // The padding grows with the text: take off whatever still overflows.
      const over = Math.max(popText.offsetHeight / Math.max(1, pop.clientHeight - padY * k), (pop.scrollWidth - padX * k) / Math.max(1, pop.clientWidth - padX * k));
      if (over > 1) {
        k /= over * 1.02;
        pop.style.fontSize = `${k * 100}%`;
      }
    };
    new ResizeObserver(() => fitPop()).observe(root);

    const popup = (it: CarSetting, was: number) => {
      fillPop(it, was);
      popping = true;
      root.classList.add('cs-popping');
      clearTimeout(popTimer);
      popTimer = window.setTimeout(() => {
        popping = false;
        root.classList.remove('cs-popping');
        visibility();
      }, (s.popupSeconds as number) * 1000);
      visibility();
      fitPop();
    };

    const flash = (id: string) => {
      const c = cells.get(id);
      if (!c) return;
      c.el.classList.remove('changed');
      void c.el.offsetWidth; // restart the animation
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
        for (const [id, c] of cells) {
          const it = d.items.find((i) => i.id === id);
          show(c.el, !!it);
          if (!it) continue;
          text(c.val, value(it));
          text(c.unit, unitOf(it));
          c.meter.set(it);
        }
        if (!fresh && d !== SAMPLE) {
          for (const it of d.items) {
            const was = prev.get(it.id);
            if (was === undefined || was === it.value || !cells.has(it.id)) continue;
            flash(it.id);
            if (popupMode !== 'flash') popup(it, was);
          }
        }
        prev = new Map(d.items.map((i) => [i.id, i.value]));
        if (d === SAMPLE && popupMode === 'only') fillPop({ id: 'tc', value: 5, max: 11 }, 4);
        visibility();
        if (!pop.hidden) fitPop();
      },
    };
  },
};
