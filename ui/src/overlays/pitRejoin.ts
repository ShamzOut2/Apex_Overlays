// Pit rejoin: where you would come out if you pitted now. The time a stop costs (the pit lane,
// measured from cars driving through it, plus your stop) taken off your place on track: the
// position you would drop to and the cars you would come out between.

import { h, show, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { PitRejoin, RejoinCar, Status } from '../lib/types';

/** Shown while placing the overlay. */
const SAMPLE: PitRejoin = {
  ready: true,
  inPits: false,
  race: true,
  loss: 52.4,
  lane: 24.1,
  laneMeasured: 6,
  stop: 28.3,
  stopFrom: 'you',
  position: [5, 9],
  classPosition: [3, 6],
  ahead: { idx: 12, number: '12', name: 'Hartmann, L', classColor: '#ffda59', sameClass: true, gap: 1.4, laps: 0 },
  behind: { idx: 7, number: '7', name: 'Moreau, J', classColor: '#ffda59', sameClass: true, gap: -0.8, laps: 0 },
  ghostPct: 0.42,
};

const FROM: Record<PitRejoin['stopFrom'], string> = { set: 'set', you: 'your last', class: 'class', none: 'no stop seen' };

export const pitRejoin: OverlayDef = {
  kind: 'pitRejoin',
  name: 'Pit rejoin',
  description:
    'If you pitted now: the position you would drop to and the cars you would come out between, from the time a stop costs here (the pit lane, measured from cars driving through it, plus your stop).',
  topics: ['pitrejoin', 'status'],
  size: [300, 150],
  commonDefaults: { autoHeight: true },
  settings: [
    {
      key: 'stopSeconds',
      shared: 'pitRejoin.stopSeconds',
      label: 'Your stop (stood in the box)',
      type: 'number',
      default: 0,
      min: 0,
      max: 300,
      step: 0.5,
      unit: 's',
      group: 'The stop',
      help: '0: worked out from the stops seen: your last one, else the middle of your class’s',
    },
    {
      key: 'laneSeconds',
      shared: 'pitRejoin.laneSeconds',
      label: 'Pit lane time lost, until measured',
      type: 'number',
      default: 25,
      min: 0,
      max: 120,
      step: 0.5,
      unit: 's',
      group: 'The stop',
      help: 'Measured from every car that drives through the pit lane here (and kept for next time); this is used until then',
    },
    {
      key: 'positions',
      label: 'Position',
      type: 'select',
      default: 'class',
      options: [
        ['class', 'In your class'],
        ['overall', 'Overall'],
        ['both', 'Both'],
      ],
      group: 'What to show',
    },
    { key: 'showCars', label: 'The cars you would come out between', type: 'bool', default: true, group: 'What to show' },
    { key: 'showLoss', label: 'What the stop costs', type: 'bool', default: true, group: 'What to show' },
    { key: 'raceOnly', label: 'Only in races', type: 'bool', default: false, group: 'Behaviour' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('pit-rejoin');
    const head = h('div', 'pr-head');
    head.append(h('span', 'pr-title', 'IF YOU PIT NOW'));
    const pos = h('div', 'pr-pos');
    const cars = h('div', 'pr-cars');
    const carRow = (arrow: string) => {
      const el = h('div', 'pr-car');
      const num = h('span', 'pr-num');
      const name = h('span', 'pr-name');
      const laps = h('span', 'pr-laps');
      const gap = h('span', 'pr-gap');
      el.append(h('span', 'pr-arrow', arrow), num, name, laps, gap);
      cars.append(el);
      return { el, num, name, laps, gap };
    };
    const ahead = carRow('▲');
    const behind = carRow('▼');
    const loss = h('div', 'pr-loss');
    const note = h('div', 'pr-note dim');
    root.append(head, pos, cars, loss, note);
    show(cars, s.showCars !== false);
    show(loss, s.showLoss !== false);

    const fill = (row: ReturnType<typeof carRow>, c: RejoinCar | null) => {
      show(row.el, !!c);
      if (!c) return;
      text(row.num, `#${c.number}`);
      style(row.num, '--cc', c.classColor);
      text(row.name, c.name);
      text(row.laps, c.laps > 0 ? `+${c.laps}L` : c.laps < 0 ? `${c.laps}L` : '');
      toggle(row.el, 'other-class', !c.sameClass);
      text(row.gap, `${c.gap > 0 ? '+' : c.gap < 0 ? '−' : ''}${Math.abs(c.gap).toFixed(1)}`);
    };
    const place = (p: [number, number] | null, label: string) => (p ? `${label}P${p[0]} → P${p[1]}` : '');

    let editing = false;
    let live: PitRejoin | null = null;
    const preview = new URLSearchParams(location.search).has('preview');

    const draw = () => {
      const sample = editing || preview;
      const d = sample && !live?.ready ? SAMPLE : live;
      const off = !d?.ready || (!!s.raceOnly && !d.race && !sample);
      toggle(root, 'auto-hidden', off && !sample);
      if (!d || !d.ready) return;
      if (d.inPits) {
        text(pos, 'In the pit lane');
        show(cars, false);
      } else {
        const both = s.positions === 'both';
        const lines = s.positions === 'overall' ? [place(d.position, '')] : both ? [place(d.classPosition, 'Class '), place(d.position, 'Overall ')] : [place(d.classPosition, '')];
        text(pos, d.race ? lines.filter(Boolean).join('   ') || '–' : 'Practice: where you would come out');
        show(cars, s.showCars !== false);
        fill(ahead, d.ahead);
        fill(behind, d.behind);
      }
      text(loss, `Stop costs ${d.loss.toFixed(1)} s`);
      text(
        note,
        `lane ${d.lane.toFixed(1)} s ${d.laneMeasured ? `(${d.laneMeasured} measured)` : '(set)'} + stop ${d.stop.toFixed(1)} s (${FROM[d.stopFrom]})`,
      );
      show(note, s.showLoss !== false);
    };
    draw();

    return {
      update(topic, data) {
        if (topic === 'status') {
          editing = !!(data as Status).editMode;
          draw();
          return;
        }
        if (topic !== 'pitrejoin') return;
        live = data as PitRejoin;
        draw();
      },
    };
  },
};
