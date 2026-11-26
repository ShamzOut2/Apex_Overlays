// Pit strategy: your next stop as set in iRacing's black box, at a glance. Fuel to add, the four
// tyres (lit when ticked), windscreen tear-off and fast repair (how many are left of the limit).
// Only what the black box says: nothing worked out.

import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { PitPlan, Status } from '../lib/types';

/** Shown while placing the overlay. */
const SAMPLE: PitPlan = {
  available: true,
  fuel: true,
  fuelAdd: 45,
  fuelFits: 45,
  level: 18.4,
  capacity: 100,
  tyres: [true, true, false, false],
  pressures: [165.5, 165.5, 162, 162],
  compound: 'Dry',
  compoundWet: false,
  tearoff: true,
  fastRepair: false,
  fastRepairsUsed: 1,
  fastRepairsLimit: 2,
  repair: 0,
  optRepair: 0,
};

const NAMES: Record<string, [string, string, string, string]> = {
  us: ['LF', 'RF', 'LR', 'RR'],
  uk: ['NSF', 'OSF', 'NSR', 'OSR'],
};

export const pitStrategy: OverlayDef = {
  kind: 'pitStrategy',
  name: 'Pit strategy',
  description:
    'Your next pit stop as set in iRacing’s black box: fuel to add, the four tyres (lit when ticked), tear-off and fast repairs left. iRacing only.',
  topics: ['pitplan', 'status'],
  size: [360, 190],
  commonDefaults: { autoHeight: true },
  settings: [
    {
      key: 'tyreNames',
      label: 'Tyre names',
      type: 'select',
      default: 'us',
      options: [
        ['us', 'LF, RF, LR, RR'],
        ['uk', 'NSF, OSF, NSR, OSR (UK)'],
      ],
      group: 'What to show',
      help: 'UK: nearside is the left of the car, offside the right',
    },
    { key: 'showFuelAfter', label: 'Fuel in the tank after the stop', type: 'bool', default: true, group: 'What to show' },
    { key: 'showPressures', label: 'Tyre pressures set', type: 'bool', default: false, group: 'What to show', help: 'kPa, or psi with imperial units' },
    { key: 'showCompound', label: 'Tyre compound', type: 'bool', default: true, group: 'What to show', help: 'For cars with more than one (dry and wet)' },
    { key: 'showTearoff', label: 'Windscreen tear-off', type: 'bool', default: true, group: 'What to show' },
    { key: 'showFastRepair', label: 'Fast repair (left / limit)', type: 'bool', default: true, group: 'What to show' },
    { key: 'showRepairs', label: 'Repair time still to do', type: 'bool', default: true, group: 'What to show' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'card', options: [['card', 'Fuel beside the car'], ['column', 'Fuel above the car']], group: 'Layout' },
    { key: 'onColor', label: 'Selected', type: 'color', default: '#3ddc84', group: 'Colours', help: 'What will be done at the stop lights up in this colour' },
    { key: 'wetColor', label: 'Wet tyres', type: 'color', default: '#3d9bff', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('pit-strategy', s.layout === 'column' ? 'ps-column' : 'ps-card');
    style(root, '--ps-on', s.onColor as string);
    style(root, '--ps-wet', s.wetColor as string);
    const u = () => ctx.units;
    const litres = (l: number) => `${f.fuel(l, u()).toFixed(1)} ${f.fuelUnit(u())}`;
    const pressure = (kpa: number) => (u() === 'imperial' ? `${(kpa * 0.145038).toFixed(1)}` : `${Math.round(kpa)}`);

    const head = h('div', 'ps-head');
    const repair = h('span', 'ps-repair');
    head.append(h('span', 'ps-title', 'PIT STRATEGY'), repair);

    // Fuel: what goes in, and the tank after.
    const fuel = h('div', 'ps-fuel');
    const fuelVal = h('div', 'ps-fuel-val');
    const fuelAfter = h('div', 'ps-fuel-after');
    fuel.append(h('div', 'ps-label', 'FUEL'), fuelVal, fuelAfter);

    // The car from above with a tyre at each corner.
    const car = h('div', 'ps-car');
    const body = h('div', 'ps-body');
    const names = NAMES[s.tyreNames as string] ?? NAMES.us;
    const tyres = names.map((n, i) => {
      const t = h('div', `ps-tyre t${i}`);
      const name = h('span', 'ps-tyre-name', n);
      const p = h('span', 'ps-tyre-p');
      t.append(name, p);
      return { t, p };
    });
    const compound = h('div', 'ps-compound');
    body.append(compound);
    car.append(tyres[0].t, tyres[1].t, body, tyres[2].t, tyres[3].t);

    const main = h('div', 'ps-main');
    main.append(fuel, car);

    // The rest of the stop.
    const chips = h('div', 'ps-chips');
    const chip = (label: string) => {
      const c = h('div', 'ps-chip');
      const v = h('span', 'ps-chip-val');
      c.append(h('span', 'ps-chip-label', label), v);
      chips.append(c);
      return { c, v };
    };
    const tear = chip('TEAR-OFF');
    const fast = chip('FAST REPAIR');
    show(tear.c, s.showTearoff !== false);
    show(fast.c, s.showFastRepair !== false);
    const note = h('div', 'ps-note dim', 'iRacing only: the black box is not in Le Mans Ultimate’s data');
    root.append(head, main, chips, note);

    let editing = false;
    let live: PitPlan | null = null;
    const preview = new URLSearchParams(location.search).has('preview');

    const draw = () => {
      const sample = editing || preview;
      const d = live?.available ? live : sample ? SAMPLE : null;
      toggle(root, 'auto-hidden', !d && !sample);
      show(note, !d && sample);
      show(main, !!d);
      show(chips, !!d && (s.showTearoff !== false || s.showFastRepair !== false));
      show(head, true);
      if (!d) return;
      // Fuel.
      toggle(fuel, 'on', d.fuel && d.fuelAdd > 0);
      text(fuelVal, d.fuel && d.fuelAdd > 0 ? `+${litres(d.fuelFits)}` : 'NO FUEL');
      const after = Math.min(d.capacity || Infinity, d.level + (d.fuel ? d.fuelFits : 0));
      show(fuelAfter, s.showFuelAfter !== false && d.fuel && d.fuelAdd > 0);
      text(fuelAfter, `${litres(d.level)} → ${litres(after)}${d.fuelFits < d.fuelAdd - 0.05 ? ` (asked ${litres(d.fuelAdd)})` : ''}`);
      // Tyres.
      tyres.forEach((t, i) => {
        toggle(t.t, 'on', d.tyres[i]);
        toggle(t.t, 'wet', d.tyres[i] && d.compoundWet);
        const p = d.pressures[i];
        show(t.p, !!s.showPressures && p != null);
        if (p != null) text(t.p, pressure(p));
      });
      show(compound, s.showCompound !== false && !!d.compound && d.tyres.some(Boolean));
      text(compound, d.compound ?? '');
      toggle(compound, 'wet', d.compoundWet);
      // Tear-off and fast repair.
      toggle(tear.c, 'on', d.tearoff);
      text(tear.v, d.tearoff ? 'YES' : 'NO');
      toggle(fast.c, 'on', d.fastRepair);
      const lim = d.fastRepairsLimit;
      text(fast.v, lim == null ? (d.fastRepair ? 'YES' : 'NO') : lim < 0 ? '∞' : `${Math.max(0, lim - d.fastRepairsUsed)} / ${lim}`);
      toggle(fast.c, 'none-left', lim != null && lim >= 0 && d.fastRepairsUsed >= lim);
      // Repairs still to do.
      show(repair, s.showRepairs !== false && d.repair > 0);
      text(repair, `REPAIRS ${Math.round(d.repair)} s${!d.fastRepair && d.optRepair > 0 ? ` +${Math.round(d.optRepair)} s opt.` : ''}`);
    };
    draw();

    return {
      update(topic, data) {
        if (topic === 'status') {
          editing = !!(data as Status).editMode;
          draw();
          return;
        }
        if (topic !== 'pitplan') return;
        live = data as PitPlan;
        draw();
      },
    };
  },
};
