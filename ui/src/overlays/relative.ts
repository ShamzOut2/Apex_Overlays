import { text } from '../lib/dom';
import * as f from '../lib/format';
import { headerSettings, sessionHeader } from '../lib/header';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES } from '../lib/settings';
import { type Col, columnFitter, Table } from '../lib/table';
import type { RelRow, Relative, Session } from '../lib/types';
import { COLUMN_STYLE_SETTINGS, driverCols, extraCols, STRIPE_COLOR_SETTING, stripeColor, tireCell } from './common';

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
  { id: 'fuelAdded', label: 'Fuel / energy added at the last stop (every car in LMU; only you in iRacing) — FUEL+', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'fuelTotal', label: 'Fuel / energy added this session (every car in LMU; only you in iRacing) — FUEL Σ', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
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
    { key: 'carsAhead', label: 'Cars ahead', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'carsBehind', label: 'Cars behind', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'keepEmptyRows', label: 'Keep empty rows (fixed height)', type: 'bool', default: true, group: 'Layout' },
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
        width: '3.5em',
        align: 'center',
        render: (c, r) => {
          const dec = s.gapDecimals as number;
          if (s.gapMode === 'leader' || s.gapMode === 'ahead') {
            // Race gaps: to the class leader, or to the car directly ahead in class.
            const leader = s.gapMode === 'leader';
            const v = leader ? r.gapLeader : r.interval;
            const laps = leader ? r.gapLeaderLaps : r.intervalLaps;
            return text(c, r.classPosition === 1 ? (leader ? 'Leader' : '') : f.gap(v, laps, dec));
          }
          if (r.isFocus) return text(c, '');
          const v = s.gapSign === 'abs' ? Math.abs(r.gap) : s.gapSign === 'aheadNegative' ? -r.gap : r.gap;
          text(c, s.gapSign === 'abs' ? v.toFixed(dec) : f.signed(v, dec));
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
        table.render(
          out,
          (r) => ({
            player: r.isFocus,
            'lap-ahead': (s.lapColors as boolean) && r.lapDiff > 0,
            'lap-behind': (s.lapColors as boolean) && r.lapDiff < 0,
            pit: (s.dimPitCars as boolean) && r.onPitRoad,
            off: r.offTrack,
          }),
          (row, r) => row.style.setProperty('--class', stripeColor(r, s)),
        );
        if (s.autoWidth) fit([table]);
      },
    };
  },
};
