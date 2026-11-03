import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { headerSettings, sessionHeader } from '../lib/header';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES } from '../lib/settings';
import { type Col, columnFitter, Table } from '../lib/table';
import type { Session, StandClass, StandRow, Standings } from '../lib/types';
import { classBadge } from '../lib/classBadges';
import { classLogo, COLUMN_STYLE_SETTINGS, driverCols, extraCols, tireCell } from './common';

const COLS = [
  { id: 'pos', label: 'Class position — POS' },
  { id: 'overall', label: 'Overall position — OVR' },
  { id: 'number', label: 'Number — #' },
  { id: 'flag', label: 'Country flag — NAT' },
  { id: 'name', label: 'Driver' },
  { id: 'team', label: 'Team' },
  { id: 'make', label: 'Manufacturer — MAKE' },
  { id: 'class', label: 'Class' },
  { id: 'car', label: 'Car' },
  { id: 'license', label: 'Licence — LIC' },
  { id: 'irating', label: 'iRating — IR' },
  { id: 'srDelta', label: 'Safety rating gain/loss (est.) — SR ±' },
  { id: 'incidents', label: 'Incidents — INC' },
  { id: 'gained', label: 'Positions gained — +/−' },
  { id: 'gap', label: 'Gap to leader — GAP' },
  { id: 'interval', label: 'Interval — INT' },
  { id: 'lastLap', label: 'Last lap — LAST' },
  { id: 'bestLap', label: 'Best lap — BEST' },
  { id: 'laps', label: 'Laps completed — LAPS' },
  { id: 'lap', label: 'Current lap — LAP' },
  { id: 'stint', label: 'Stint lap / OUTLAP / PIT — STINT' },
  { id: 'pits', label: 'Pit stops — STOPS' },
  { id: 'tire', label: 'Tyre (wet/dry) — TYRE' },
  { id: 'points', label: 'League points for position — PTS' },
  { id: 'seasonPoints', label: 'Championship total (projected) — CHAMP' },
];

interface Section {
  id: string;
  el: HTMLElement;
  head: HTMLElement;
  name: HTMLElement;
  meta: HTMLElement;
  logo: HTMLImageElement;
  table: Table<StandRow>;
}

export const standings: OverlayDef = {
  kind: 'standings',
  name: 'Standings',
  description: 'Multi-class leaderboard with gaps, intervals, lap times, pit stops and positions gained. Respects league classes.',
  topics: ['standings', 'session'],
  size: [700, 380],
  commonDefaults: { autoHeight: true, autoWidth: true },
  fitsWidth: true,
  settings: [
    { key: 'mode', label: 'Show', type: 'select', default: 'multiclass', options: [['multiclass', 'All classes'], ['myClass', 'My class only'], ['overall', 'Single overall list']], group: 'Layout' },
    { key: 'maxRows', label: 'Rows in my class', type: 'number', default: 10, min: 1, max: 64, group: 'Layout' },
    { key: 'otherClassRows', when: (v) => (v.mode ?? 'multiclass') === 'multiclass', label: 'Rows in other classes', type: 'number', default: 3, min: 0, max: 64, group: 'Layout' },
    { key: 'topRows', label: 'Always show top', type: 'number', default: 3, min: 0, max: 20, group: 'Layout', help: 'The rest of the rows follow the car you are watching' },
    { key: 'columns', label: 'Columns', type: 'columns', ...columns(COLS, ['pos', 'number', 'flag', 'name', 'irating', 'gap', 'interval', 'lastLap', 'bestLap']), group: 'Columns' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Columns' },
    { key: 'gapDecimals', label: 'Gap decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Columns' },
    ...COLUMN_STYLE_SETTINGS,
    ...headerSettings(['time', 'lap', 'cars']),
    { key: 'showClassHeader', label: 'Show class headers', type: 'bool', default: true, group: 'Header' },
    {
      key: 'classLogo', when: (v) => v.showClassHeader !== false,
      label: 'Class badge in class headers',
      type: 'select',
      default: 'only',
      options: [
        ['only', 'Badge (class name only when there is no badge)'],
        ['withName', 'Badge and class name'],
        ['off', 'Class name only'],
      ],
      group: 'Header',
      help: 'Badges are built in for iRacing’s classes (GT3, LMP2, TCR, …). Your own images in the data folder under logos/ (e.g. class-gt3.png) take priority',
    },
    { key: 'showColumnTitles', label: 'Show column titles', type: 'bool', default: true, group: 'Header' },
    { key: 'columnTitlesEveryClass', when: (v) => v.showColumnTitles !== false && (v.mode ?? 'multiclass') === 'multiclass', label: 'Repeat column titles for every class', type: 'bool', default: false, group: 'Header' },
    { key: 'dimPitCars', label: 'Dim cars in the pits', type: 'bool', default: true, group: 'Colours' },
    { key: 'dimDisconnected', label: 'Dim disconnected cars', type: 'bool', default: true, group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const dec = s.gapDecimals as number;
    const overall = s.mode === 'overall';
    const header = sessionHeader(s, 'Standings', () => ctx.units);
    const body = h('div', 'sections');
    root.append(header.el, body);

    const cols: Col<StandRow>[] = [
      { id: 'pos', label: 'Pos', width: '2.4em', align: 'center', render: (c, r) => text(c, String(r.classPosition)) },
      { id: 'overall', label: 'Ovr', width: '2.4em', align: 'center', render: (c, r) => text(c, String(r.position)) },
      ...driverCols<StandRow>(s),
      {
        id: 'gained',
        label: '+/−',
        width: '2.6em',
        align: 'center',
        render: (c, r) => {
          const g = r.positionsGained ?? 0;
          text(c, g > 0 ? `▲${g}` : g < 0 ? `▼${-g}` : '');
          toggle(c, 'good', g > 0);
          toggle(c, 'bad', g < 0);
        },
      },
      // In one overall list there is a single leader (overall P1) and gaps are overall.
      {
        id: 'gap',
        label: 'Gap',
        width: '3.8em',
        align: 'center',
        render: (c, r) =>
          text(c, overall ? (r.position === 1 ? 'Leader' : f.gap(r.gapOverall, r.gapOverallLaps, dec)) : r.classPosition === 1 ? 'Leader' : f.gap(r.gap, r.gapLaps, dec)),
      },
      {
        id: 'interval',
        label: 'Int',
        width: '3.6em',
        align: 'center',
        render: (c, r) => text(c, overall ? (r.position === 1 ? '' : f.gap(r.intervalOverall, r.intervalOverallLaps, dec)) : r.classPosition === 1 ? '' : f.gap(r.interval, r.intervalLaps, dec)),
      },
      { id: 'laps', label: 'Laps', width: '2.6em', align: 'center', render: (c, r) => text(c, String(r.laps)) },
      { id: 'pits', label: 'Stops', width: '2.8em', align: 'center', render: (c, r) => text(c, r.pitStops ? String(r.pitStops) : '') },
      { id: 'tire', label: 'Tyre', width: '2.4em', align: 'center', render: (c, r) => tireCell(c, r.tireWet) },
      ...extraCols<StandRow>(s),
    ];
    const order = enabledColumns(s.columns);
    let sections: Section[] = [];
    const fit = columnFitter();
    let key = '';

    const section = (id: string, first: boolean): Section => {
      const el = h('div', 'section');
      const head = h('div', 'class-head');
      const name = h('span', 'class-name');
      const meta = h('span', 'info');
      const logo = h('img', 'class-logo');
      logo.hidden = true;
      const left = h('span', 'class-left');
      left.append(logo, name);
      head.append(left, meta);
      show(head, s.showClassHeader as boolean);
      const table = new Table<StandRow>(cols, order, (s.showColumnTitles as boolean) && (first || (s.columnTitlesEveryClass as boolean)));
      el.append(head, table.el);
      return { id, el, head, name, meta, logo, table };
    };

    /** Top rows plus a window around the focused car, padded with leaders. */
    const pick = (rows: StandRow[], max: number): (StandRow | null)[] => {
      if (rows.length <= max) return rows;
      const top = Math.min(s.topRows as number, max);
      const focus = rows.findIndex((r) => r.isFocus);
      if (focus < 0 || focus < max) return rows.slice(0, max);
      const rest = max - top;
      let start = Math.max(top, focus - Math.floor((rest - 1) / 2));
      start = Math.min(start, rows.length - rest);
      return [...rows.slice(0, top), ...rows.slice(start, start + rest)];
    };

    return {
      update(topic, data) {
        if (topic === 'session') return header.update(data as Session);
        if (topic !== 'standings') return;
        const d = data as Standings;
        let classes: StandClass[] = d.classes;
        if (s.mode === 'myClass') classes = classes.filter((c) => c.id === d.focusClass);
        if (s.mode === 'overall') {
          const all = classes.flatMap((c) => c.rows).sort((a, b) => a.position - b.position);
          classes = [{ id: 'overall', name: 'Overall', color: 'transparent', count: all.length, sof: 0, bestLap: null, rows: all }];
        }
        const k = classes.map((c) => c.id).join('|');
        if (k !== key) {
          key = k;
          sections = classes.map((c, i) => section(c.id, i === 0));
          body.replaceChildren(...sections.map((x) => x.el));
        }
        classes.forEach((c, i) => {
          const sec = sections[i];
          text(sec.name, c.name);
          const redo = () => (key = '');
          // The single overall list has no class badge: its header just says Overall.
          const wanted = s.classLogo !== 'off' && c.id !== 'overall';
          const url = wanted ? (classLogo(c.name, redo) ?? classBadge(c.name, c.id, c.color, c.rows.map((r) => r.car), s.classBadgeColor as string, redo)) : null;
          if (url && sec.logo.getAttribute('src') !== url) sec.logo.src = url;
          if (sec.logo.hidden !== !url) sec.logo.hidden = !url;
          show(sec.name, !(url && s.classLogo === 'only'));
          sec.logo.alt = c.name;
          sec.head.style.setProperty('--class', c.color);
          text(sec.meta, [c.sof ? `SOF ${f.iRating(c.sof)}` : '', `${c.count} cars`, c.bestLap ? f.lapTime(c.bestLap) : ''].filter(Boolean).join('  ·  '));
          const mine = c.id === d.focusClass || s.mode !== 'multiclass';
          const rows = pick(c.rows, mine ? (s.maxRows as number) : (s.otherClassRows as number));
          show(sec.el, rows.length > 0);
          sec.table.render(
            rows,
            (r) => ({
              player: r.isFocus,
              pit: (s.dimPitCars as boolean) && r.onPitRoad,
              out: (s.dimDisconnected as boolean) && !r.inWorld,
            }),
            (row, r) => row.style.setProperty('--class', r.classColor),
          );
        });
        if (s.autoWidth) fit(sections.map((x) => x.table));
      },
    };
  },
};
