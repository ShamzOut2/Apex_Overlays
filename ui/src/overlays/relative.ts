import { h, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { headerSettings, sessionHeader } from '../lib/header';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES } from '../lib/settings';
import { type Col, columnFitter, Table } from '../lib/table';
import type { RelRow, Relative, Session } from '../lib/types';
import { applyLook, COLUMN_STYLE_SETTINGS, driverCols, lookSettings, markerSettings, extraCols, STRIPE_COLOR_SETTING, stripeColor, tireCell, privateQualifyingSetting } from './common';

const COLS = [
  { id: 'pos', label: 'Class position — POS' },
  { id: 'overall', label: 'Overall position — OVR' },
  { id: 'number', label: 'Number — #' },
  { id: 'flag', label: 'Country flag — NAT' },
  { id: 'name', label: 'Driver' },
  { id: 'team', label: 'Team' },
  { id: 'teamLogo', label: 'Team logo (Team builder)' },
  { id: 'class2', label: 'Secondary class (League builder) — 2ND' },
  { id: 'make', label: 'Manufacturer — MAKE' },
  { id: 'class', label: 'Class' },
  { id: 'car', label: 'Car' },
  { id: 'license', label: 'Licence — LIC' },
  { id: 'irating', label: 'iRating — IR' },
  { id: 'srDelta', label: 'Safety rating gain/loss (est.) — SR ±' },
  { id: 'incidents', label: 'Incidents — INC' },
  { id: 'lap', label: 'Current lap — LAP' },
  { id: 'stint', label: 'Stint lap / OUTLAP / PIT — STINT' },
  { id: 'lastLap', label: 'Last lap — LAST' },
  { id: 'bestLap', label: 'Best lap — BEST' },
  { id: 'tire', label: 'Tyre (wet/dry) — TYRE' },
  { id: 'points', label: 'League points for position — PTS' },
  { id: 'fuelLeft', label: 'Fuel / energy left — FUEL', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'fuelAdded', label: 'Fuel added at last stop — FUEL+', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'pitTime', label: 'Pit time: stood in the box at the last stop (S&G: a penalty) — PIT TIME' },
  { id: 'fuelTotal', label: 'Fuel added this session — FUEL Σ', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'seasonPoints', label: 'Championship total (projected) — CHAMP' },
  { id: 'gap', label: 'Gap' },
];

export const relative: OverlayDef = {
  kind: 'relative',
  name: 'Relative',
  description: 'Cars around you on track with live time gaps, lap-ahead/behind colouring and class colours.',
  topics: ['relative', 'session'],
  size: [600, 320],
  fitsWidth: true,
  settings: [
    privateQualifyingSetting('Layout'),
    { key: 'carsAhead', label: 'Cars ahead', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'carsBehind', label: 'Cars behind', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'keepEmptyRows', label: 'Keep empty rows (fixed height)', type: 'bool', default: true, group: 'Layout' },
    ...lookSettings('relative'),
    { key: 'columns', label: 'Columns', type: 'columns', ...columns(COLS, ['pos', 'number', 'flag', 'name', 'license', 'irating', 'lastLap', 'bestLap', 'gap']), group: 'Columns' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Columns' },
    {
      key: 'gapMode',
      label: 'Gap shows',
      type: 'select',
      default: 'relative',
      options: [
        ['relative', 'Time to each car on track (relative)'],
        ['leader', 'Gap to the class leader'],
        ['ahead', 'Gap to the car ahead in class (interval)'],
      ],
      help: 'Leader and interval gaps are race gaps, shown in race sessions',
      group: 'Columns',
    },
    { key: 'gapSign', label: 'Gap sign', type: 'select', default: 'aheadNegative', options: [['abs', 'No sign'], ['aheadNegative', 'Ahead negative'], ['aheadPositive', 'Ahead positive']], group: 'Columns' },
    { key: 'gapDecimals', label: 'Gap decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Columns' },
    ...markerSettings(false),
    ...COLUMN_STYLE_SETTINGS,
    ...headerSettings(['time', 'sof', 'incidents']),
    { key: 'showColumnTitles', label: 'Show column titles', type: 'bool', default: true, group: 'Header' },
    { key: 'lapColors', label: 'Colour cars a lap ahead / behind', type: 'bool', default: true, group: 'Colours' },
    { key: 'classStripe', label: 'Colour bar by each driver', type: 'bool', default: true, group: 'Colours' },
    { ...STRIPE_COLOR_SETTING, when: (v) => v.classStripe !== false },
    { key: 'dimPitCars', label: 'Dim cars in the pits', type: 'bool', default: true, group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    applyLook(root, s);
    const modern = root.classList.contains('modern');
    const close = s.closeGap as number;
    const range = s.gapBarRange as number;
    const bars = modern && s.gapBar !== false;
    const trends = modern && s.gapTrend !== false;
    const trend = gapTrends();
    const header = sessionHeader(s, 'Relative', () => ctx.units);
    const cols: Col<RelRow>[] = [
      { id: 'pos', label: 'Pos', width: '2.4em', align: 'center', render: (c, r) => text(c, r.classPosition > 0 ? String(r.classPosition) : '') },
      { id: 'overall', label: 'Ovr', width: '2.4em', align: 'center', render: (c, r) => text(c, r.position > 0 ? String(r.position) : '') },
      ...driverCols<RelRow>(s),
      ...extraCols<RelRow>(s, () => ctx.units),
      { id: 'tire', label: 'Tyre', width: '2.4em', align: 'center', render: (c, r) => tireCell(c, r.tireWet) },
      {
        id: 'gap',
        label: s.gapMode === 'leader' ? 'Leader' : s.gapMode === 'ahead' ? 'Int' : 'Gap',
        width: trends ? '4.2em' : '3.5em',
        align: 'center',
        render: (c, r) => {
          const dec = s.gapDecimals as number;
          if (s.gapMode === 'leader' || s.gapMode === 'ahead') {
            // Race gaps: to the class leader, or to the car directly ahead in class.
            const leader = s.gapMode === 'leader';
            const v = leader ? r.gapLeader : r.interval;
            const laps = leader ? r.gapLeaderLaps : r.intervalLaps;
            return gapCell(c, r.classPosition === 1 ? (leader ? 'Leader' : '') : f.gap(v, laps, dec), trends ? trend.of(r) : 0, modern, trends);
          }
          if (r.isFocus) return gapCell(c, '', 0, modern, trends);
          const v = s.gapSign === 'abs' ? Math.abs(r.gap) : s.gapSign === 'aheadNegative' ? -r.gap : r.gap;
          gapCell(c, s.gapSign === 'abs' ? v.toFixed(dec) : f.signed(v, dec), trends ? trend.of(r) : 0, modern, trends);
        },
      },
    ];
    const table = new Table<RelRow>(cols, enabledColumns(s.columns), s.showColumnTitles as boolean);
    const fit = columnFitter();
    root.append(header.el, table.el);
    root.classList.toggle('stripe', s.classStripe as boolean);

    const ahead = s.carsAhead as number;
    const behind = s.carsBehind as number;
    return {
      update(topic, data) {
        if (topic === 'session') return header.update(data as Session);
        if (topic !== 'relative') return;
        const rows = (data as Relative).rows;
        const me = rows.findIndex((r) => r.isFocus);
        const slots: (RelRow | null)[] = [];
        for (let k = -ahead; k <= behind; k++) slots.push(me < 0 ? null : (rows[me + k] ?? null));
        const out = s.keepEmptyRows ? slots : slots.filter((r) => r != null);
        if (trends) trend.add(out);
        table.render(
          out,
          (r) => ({
            player: r.isFocus,
            'lap-ahead': (s.lapColors as boolean) && r.lapDiff > 0,
            'lap-behind': (s.lapColors as boolean) && r.lapDiff < 0,
            pit: (s.dimPitCars as boolean) && r.onPitRoad,
            off: r.offTrack,
            // Modern look: a car within the "close" gap, and the gap bar behind the row.
            close: modern && !r.isFocus && Math.abs(r.gap) < close,
            near: bars && !r.isFocus && Math.abs(r.gap) < range,
          }),
          (row, r) => {
            row.style.setProperty('--class', stripeColor(r, s));
            if (bars) {
              // In 5% steps: the bar moves when the eye can see it, not on every update.
              const near = r.isFocus ? '0' : String(Math.round(Math.max(0, 1 - Math.abs(r.gap) / range) * 20) / 20);
              if (row.style.getPropertyValue('--near') !== near) row.style.setProperty('--near', near);
            }
          },
        );
        if (s.autoWidth) fit([table]);
      },
    };
  },
};

/**
 * The gap with, in the modern look, an arrow by it: ▲ green when the gap is moving your way
 * (closing on a car ahead, pulling away from one behind), ▼ red when not, nothing when steady.
 */
function gapCell(c: HTMLElement, value: string, dir: number, modern: boolean, arrows: boolean) {
  if (!modern) return text(c, value);
  let arrow = c.firstElementChild as HTMLElement | null;
  let v = arrow?.nextElementSibling as HTMLElement | null;
  if (!arrow || !v) {
    arrow = h('span', 'trend');
    v = h('span', 'gv');
    c.replaceChildren(arrow, v);
  }
  text(v, value);
  if (arrow.hidden !== !arrows) arrow.hidden = !arrows;
  text(arrow, dir > 0 ? '▲' : dir < 0 ? '▼' : '');
  toggle(arrow, 'good', dir > 0);
  toggle(arrow, 'bad', dir < 0);
}

/** How each car's gap has moved over the last second and a half (for the gain/loss arrows). */
function gapTrends() {
  const WINDOW = 1500;
  const STEADY = 0.05;
  const hist = new Map<number, { t: number; g: number }[]>();
  let now = 0;
  return {
    add(rows: (RelRow | null)[]) {
      now = performance.now();
      const seen = new Set<number>();
      for (const r of rows) {
        if (!r || r.isFocus) continue;
        seen.add(r.idx);
        let h = hist.get(r.idx);
        if (!h) hist.set(r.idx, (h = []));
        h.push({ t: now, g: r.gap });
        while (h.length > 2 && now - h[1].t >= WINDOW) h.shift();
      }
      for (const k of hist.keys()) if (!seen.has(k)) hist.delete(k);
    },
    /** +1: moving your way, -1: against you, 0: steady or not known yet. */
    of(r: RelRow): number {
      const h = hist.get(r.idx);
      const ref = h?.[0];
      if (!ref || now - ref.t < WINDOW * 0.66) return 0;
      // A car that changed sides (passed or was passed) has no trend.
      if (Math.sign(ref.g) !== Math.sign(r.gap)) return 0;
      const d = Math.abs(r.gap) - Math.abs(ref.g);
      if (Math.abs(d) < STEADY) return 0;
      // Ahead (gap > 0): the gap shrinking is good; behind: it growing is.
      return (r.gap > 0 ? d < 0 : d > 0) ? 1 : -1;
    },
  };
}
