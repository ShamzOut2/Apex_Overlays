// Corner times: your last lap, corner by corner, against your best. Each corner counts with
// the straight after it (exit speed shows there), so the figures add up to the lap's gap.
// Green where you gained, red where you lost.

import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { CornerTimes, Status } from '../lib/types';

/** Shown while placing the overlay. */
const SAMPLE: CornerTimes = {
  lapTime: 102.614,
  valid: true,
  by: 'corners',
  segments: [
    { number: 1, last: 0, name: 'La Source' },
    { number: 2, last: 0, name: 'Eau Rouge' },
    { number: 3, last: 4, name: 'Raidillon' },
    { number: 5, last: 6, name: 'Les Combes' },
    { number: 7, last: 0, name: 'Malmedy' },
    { number: 8, last: 0, name: 'Bruxelles' },
    { number: 10, last: 11, name: 'Double Gauche' },
    { number: 12, last: 13, name: 'Les Fagnes' },
    { number: 14, last: 0, name: 'Campus' },
    { number: 15, last: 0, name: 'Stavelot' },
    { number: 16, last: 17, name: 'Blanchimont' },
    { number: 18, last: 19, name: 'Chicane' },
  ],
  vs: [
    { source: 'session', lapTime: 102.2, delta: 0.414, deltas: [0.08, -0.03, 0.01, 0.21, -0.05, 0.12, 0.02, -0.04, 0.0, 0.09, 0.01, -0.006] },
    { source: 'allTime', lapTime: 101.9, delta: 0.714, deltas: [0.1, 0.0, 0.05, 0.25, 0.0, 0.15, 0.05, 0.0, 0.02, 0.1, 0.02, -0.03] },
  ],
};

export const cornerTimes: OverlayDef = {
  kind: 'cornerTimes',
  name: 'Corner times',
  description:
    'After each lap: where you gained and lost time against your best, corner by corner (each corner with the straight after it), green and red. Turn numbers as on iRacing’s map.',
  topics: ['cornertimes', 'status'],
  size: [800, 70],
  fitsWidth: true,
  commonDefaults: { autoHeight: true },
  settings: [
    {
      key: 'reference',
      label: 'Against',
      type: 'select',
      default: 'session',
      options: [
        ['session', 'Your best lap this session'],
        ['allTime', 'Your best lap ever (this car and track)'],
      ],
      group: 'What to show',
      help: 'Your best as it stood before the lap: a new best shows what made it quicker',
    },
    {
      key: 'show',
      label: 'Corners shown',
      type: 'select',
      default: 'all',
      options: [
        ['all', 'Every corner, in lap order'],
        ['worst', 'Where you lost most, worst first'],
        ['both', 'Biggest gains and losses, worst first'],
      ],
      group: 'What to show',
    },
    { key: 'count', when: (v) => v.show !== 'all', label: 'How many', type: 'number', default: 4, min: 1, max: 12, group: 'What to show' },
    { key: 'showHeader', label: 'Lap time and gap to your best', type: 'bool', default: true, group: 'What to show' },
    {
      key: 'label',
      label: 'Corner names',
      type: 'select',
      default: 'number',
      options: [
        ['number', 'Turn numbers (T5)'],
        ['name', 'Names, where known (Les Combes)'],
        ['both', 'Both (T5 Les Combes)'],
      ],
      group: 'What to show',
    },
    { key: 'prefix', when: (v) => v.label !== 'name', label: 'Turn number prefix', type: 'text', default: 'T', group: 'What to show' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'row', options: [['row', 'In a row'], ['column', 'In a column']], group: 'Layout' },
    { key: 'tolerance', label: 'Counts as even within', type: 'number', default: 0.02, min: 0, max: 0.5, step: 0.01, unit: 's', group: 'Colours' },
    { key: 'gainColor', label: 'Gained', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'lossColor', label: 'Lost', type: 'color', default: '#ff4d4d', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('corner-times', s.layout === 'column' ? 'ct-column' : 'ct-row');
    style(root, '--ct-gain', s.gainColor as string);
    style(root, '--ct-loss', s.lossColor as string);
    const head = h('div', 'ct-head');
    const title = h('span', 'ct-title', 'LAST LAP');
    const lap = h('span', 'ct-lap');
    const gap = h('span', 'ct-gap');
    const note = h('span', 'ct-note');
    head.append(title, lap, gap, note);
    show(head, s.showHeader !== false);
    const cells = h('div', 'ct-cells');
    const empty = h('div', 'ct-empty dim', 'Drive a full lap to see your corner times');
    root.append(head, cells, empty);
    const tol = s.tolerance as number;
    const prefix = String(s.prefix ?? 'T');

    const label = (g: CornerTimes['segments'][number]) => {
      const num = g.number > 0 ? `${prefix}${g.number}${g.last > g.number ? `-${g.last}` : ''}` : '';
      if (s.label === 'name') return g.name || num;
      if (s.label === 'both') return num && g.name ? `${num} ${g.name}` : num || g.name;
      return num || g.name;
    };
    const signed = (v: number) => `${v > 0 ? '+' : v < 0 ? '−' : '±'}${Math.abs(v).toFixed(2)}`;

    let editing = false;
    let live: CornerTimes | null = null;
    let key = '';
    const preview = new URLSearchParams(location.search).has('preview');

    const draw = () => {
      const d = (editing || preview) && !live?.lapTime ? SAMPLE : live;
      const vs = d?.vs.find((v) => v.source === s.reference) ?? d?.vs[0] ?? null;
      const ok = !!d?.lapTime && !!vs && vs.deltas.length === d.segments.length;
      show(empty, !ok);
      show(cells, ok);
      if (!ok || !d || !vs) {
        text(lap, '');
        text(gap, '');
        text(note, '');
        return;
      }
      const k = JSON.stringify([d.lapTime, vs.source, vs.deltas]);
      if (k === key) return;
      key = k;
      text(lap, f.lapTime(d.lapTime as number));
      text(gap, `${signed(vs.delta)} vs ${vs.source === 'allTime' ? 'your best ever' : 'your best'}`);
      toggle(gap, 'loss', vs.delta > tol);
      toggle(gap, 'gain', vs.delta < -tol);
      text(note, d.valid ? '' : 'off track / pit lane');
      // Which corners, in which order.
      let idx = d.segments.map((_, i) => i);
      const n = Math.max(1, Math.round(s.count as number));
      if (s.show === 'worst') idx = idx.filter((i) => vs.deltas[i] > tol).sort((a, b) => vs.deltas[b] - vs.deltas[a]).slice(0, n);
      else if (s.show === 'both') idx = [...idx].sort((a, b) => Math.abs(vs.deltas[b]) - Math.abs(vs.deltas[a])).slice(0, n).sort((a, b) => vs.deltas[b] - vs.deltas[a]);
      // The bars: the biggest change on the lap fills one.
      const most = Math.max(0.05, ...vs.deltas.map((v) => Math.abs(v)));
      cells.replaceChildren(
        ...idx.map((i) => {
          const v = vs.deltas[i];
          const c = h('div', `ct-cell ${v > tol ? 'loss' : v < -tol ? 'gain' : 'even'}`);
          const bar = h('div', 'ct-bar');
          bar.style.setProperty('--w', `${Math.round((Math.abs(v) / most) * 100)}%`);
          c.append(h('span', 'ct-label', label(d.segments[i])), h('span', 'ct-val', signed(v)), bar);
          return c;
        }),
      );
      if (!idx.length) cells.append(h('div', 'ct-cell even', 'No time lost'));
    };
    draw();

    return {
      update(topic, data) {
        if (topic === 'status') {
          editing = !!(data as Status).editMode;
          draw();
          return;
        }
        if (topic !== 'cornertimes') return;
        live = data as CornerTimes;
        draw();
      },
    };
  },
};
