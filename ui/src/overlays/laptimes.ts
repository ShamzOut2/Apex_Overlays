// Your recent laps with their sector times: personal bests in green, the class's fastest lap
// in purple, the best possible lap (your best sectors added up), and the lap under way.

import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { LapTimes, Status } from '../lib/types';

/** Shown while placing the overlay. */
const SAMPLE: LapTimes = {
  car: 0, currentLap: 9, currentTime: 41.2, currentSectors: [29.41, null, null], currentInvalid: false,
  laps: [
    { lap: 8, time: 88.917, sectors: [29.512, 30.004, 29.401], pit: false, invalid: false, pb: false, classBest: false },
    { lap: 7, time: 88.402, sectors: [29.388, 29.731, 29.283], pit: false, invalid: false, pb: true, classBest: false },
    { lap: 6, time: 88.655, sectors: [29.401, 29.698, 29.556], pit: false, invalid: true, pb: false, classBest: false },
    { lap: 5, time: 89.214, sectors: [29.602, 29.840, 29.772], pit: false, invalid: false, pb: false, classBest: false },
    { lap: 4, time: 118.03, sectors: [29.66, 30.112, 58.258], pit: true, invalid: false, pb: false, classBest: false },
  ],
  bestLap: 88.402, bestSectors: [29.388, 29.698, 29.283], theoretical: 88.369, classBest: 87.951,
};

const time = (v: number | null | undefined) => (v == null ? '–' : f.lapTime(v));
const sector = (v: number | null | undefined) => (v == null ? '–' : v >= 60 ? f.lapTime(v) : v.toFixed(2));

export const laptimes: OverlayDef = {
  kind: 'laptimes',
  name: 'Lap times',
  description:
    'Your last laps with sector times: personal bests in green, class best in purple, the gap to your best and your best possible lap. In LMU, laps lost to track limits are struck out.',
  topics: ['laptimes', 'status'],
  size: [420, 260],
  commonDefaults: { autoHeight: true },
  settings: [
    { key: 'laps', label: 'Laps shown', type: 'number', default: 5, min: 1, max: 15, group: 'What to show' },
    { key: 'showCurrent', label: 'The lap under way', type: 'bool', default: true, group: 'What to show' },
    { key: 'showSectors', label: 'Sector times', type: 'bool', default: true, group: 'What to show' },
    { key: 'showDelta', label: 'Gap to your best lap', type: 'bool', default: true, group: 'What to show' },
    { key: 'showFooter', label: 'Best lap and best possible lap', type: 'bool', default: true, group: 'What to show' },
    { key: 'markInvalid', label: 'Strike out invalid laps', type: 'bool', default: true, group: 'What to show', lmu: true, help: 'Le Mans Ultimate tells when a lap no longer counts; iRacing does not' },
    { key: 'slowerYellow', when: (v) => v.showSectors !== false, label: 'Slower sectors in yellow', type: 'bool', default: false, group: 'Colours', help: 'Off: only your best sectors are coloured (green)' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('laptimes');
    const head = h('div', 'lt-head');
    const title = h('span', 'lt-title', 'LAP TIMES');
    const cur = h('span', 'lt-cur');
    head.append(title, cur);
    const table = h('div', 'lt-table');
    const foot = h('div', 'lt-foot');
    root.append(head, table, foot);
    show(foot, s.showFooter !== false);
    show(cur, s.showCurrent !== false);

    const cell = (cls: string, t: string) => h('span', `lt-c ${cls}`, t);
    let editing = false;
    let live: LapTimes | null = null;
    const preview = new URLSearchParams(location.search).has('preview');
    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        else if (topic === 'laptimes') live = data as LapTimes;
        else return;
        const d = (editing || preview) && !live?.laps.length ? SAMPLE : live;
        if (!d) return;
        const n = Math.max(d.bestSectors.length, d.laps[0]?.sectors.length ?? 0, 1);
        const sectors = s.showSectors !== false;
        const delta = s.showDelta !== false;
        root.style.setProperty('--lt-cols', `2.6em 5.2em ${sectors ? `repeat(${n}, minmax(3.4em, 1fr))` : ''} ${delta ? '4em' : ''}`);
        // The lap under way: its time so far and the sectors done.
        const sec = d.currentSectors.map((v, i) => `S${i + 1} ${sector(v)}`).join('  ');
        text(cur, d.currentTime == null ? '' : `L${d.currentLap}  ${time(d.currentTime)}${sectors && d.currentSectors.some((v) => v != null) ? `  ·  ${sec}` : ''}`);
        toggle(cur, 'invalid', !!s.markInvalid && d.currentInvalid);
        const rows: HTMLElement[] = [];
        const hd = h('div', 'lt-row lt-hd');
        hd.append(cell('', 'LAP'), cell('', 'TIME'));
        if (sectors) for (let i = 0; i < n; i++) hd.append(cell('', `S${i + 1}`));
        if (delta) hd.append(cell('', 'Δ BEST'));
        rows.push(hd);
        for (const l of d.laps.slice(0, s.laps as number)) {
          const row = h('div', 'lt-row');
          const invalid = !!s.markInvalid && l.invalid;
          toggle(row, 'invalid', invalid);
          toggle(row, 'pit', l.pit);
          const lapCell = cell('lt-lap', String(l.lap));
          if (l.pit) lapCell.append(h('span', 'lt-tag', 'PIT'));
          if (invalid) lapCell.append(h('span', 'lt-tag inv', 'INVALID'));
          const t = cell('lt-time', time(l.time));
          toggle(t, 'best', l.classBest);
          toggle(t, 'pb', l.pb && !l.classBest);
          row.append(lapCell, t);
          if (sectors) {
            for (let i = 0; i < n; i++) {
              const v = l.sectors[i] ?? null;
              const c = cell('lt-s', sector(v));
              const isBest = v != null && d.bestSectors[i] != null && Math.abs(v - (d.bestSectors[i] as number)) < 0.0005;
              toggle(c, 'pb', isBest && !invalid);
              toggle(c, 'slow', !isBest && v != null && !!s.slowerYellow && !invalid);
              row.append(c);
            }
          }
          if (delta) {
            const dv = l.time != null && d.bestLap != null ? l.time - d.bestLap : null;
            const c = cell('lt-d', dv == null ? '' : Math.abs(dv) < 0.0005 ? 'BEST' : `${dv > 0 ? '+' : '−'}${Math.abs(dv).toFixed(3)}`);
            toggle(c, 'good', dv != null && Math.abs(dv) < 0.0005);
            row.append(c);
          }
          rows.push(row);
        }
        if (!d.laps.length) rows.push(h('div', 'lt-empty dim', 'Laps show here as you complete them.'));
        table.replaceChildren(...rows);
        foot.replaceChildren(
          h('span', '', 'BEST '),
          Object.assign(h('b', 'pb'), { textContent: time(d.bestLap) }),
          h('span', '', '   BEST POSSIBLE '),
          Object.assign(h('b', ''), { textContent: time(d.theoretical) }),
          ...(d.classBest != null ? [h('span', '', '   CLASS '), Object.assign(h('b', 'best'), { textContent: time(d.classBest) })] : []),
        );
      },
    };
  },
};
