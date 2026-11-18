import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { headerSettings, sessionHeader } from '../lib/header';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES } from '../lib/settings';
import { type Col, columnFitter, contrast, Table } from '../lib/table';
import type { Session, StandClass, StandRow, Standings } from '../lib/types';
import { classBadge } from '../lib/classBadges';
import { applyLook, classLogo, COLUMN_STYLE_SETTINGS, lookSettings, driverCols, extraCols, markersCell, markerSettings, STRIPE_COLOR_SETTING, stripeColor, tireCell } from './common';

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
  { id: 'fuelLeft', label: 'Fuel / energy left — FUEL', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'fuelAdded', label: 'Fuel added at last stop — FUEL+', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
  { id: 'pitTime', label: 'Pit time: stood in the box at the last stop (S&G: a penalty) — PIT TIME' },
  { id: 'fuelTotal', label: 'Fuel added this session — FUEL Σ', lmu: 'Every car in Le Mans Ultimate; only you in iRacing' },
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
    ...lookSettings('standings'),
    { key: 'columns', label: 'Columns', type: 'columns', ...columns(COLS, ['pos', 'number', 'flag', 'name', 'irating', 'gap', 'interval', 'lastLap', 'bestLap']), group: 'Columns' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Columns' },
    ...markerSettings(true),
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
      help: 'Built in for iRacing’s classes. Your own images go in the data folder under logos/ (e.g. class-gt3.png)',
    },
    { key: 'showColumnTitles', label: 'Show column titles', type: 'bool', default: true, group: 'Header' },
    { key: 'columnTitlesEveryClass', when: (v) => v.showColumnTitles !== false && (v.mode ?? 'multiclass') === 'multiclass', label: 'Repeat column titles for every class', type: 'bool', default: false, group: 'Header' },
    STRIPE_COLOR_SETTING,
    { key: 'dimPitCars', label: 'Dim cars in the pits', type: 'bool', default: true, group: 'Colours' },
    { key: 'dimDisconnected', label: 'Dim disconnected cars', type: 'bool', default: true, group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const dec = s.gapDecimals as number;
    const overall = s.mode === 'overall';
    applyLook(root, s);
    root.classList.toggle('mode-overall', overall);
    const modern = root.classList.contains('modern');
    const close = s.closeGap as number;
    const battles = modern && s.battleLine !== false;
    const podium = modern && s.podium !== false;
    /** Within the "close" gap of the car directly ahead (in the list shown). */
    const closeBehind = (r: StandRow, ahead: StandRow | null | undefined) => {
      const pos = overall ? r.position : r.classPosition;
      const int = overall ? r.intervalOverall : r.interval;
      const laps = overall ? r.intervalOverallLaps : r.intervalLaps;
      return !!ahead && (overall ? ahead.position : ahead.classPosition) === pos - 1 && int != null && int < close && laps === 0;
    };
    const header = sessionHeader(s, 'Standings', () => ctx.units);
    const body = h('div', 'sections');
    // Flags and penalties outside the table: the table on a card of its own, the markers
    // just past its left or right edge, on the overlay's clear background.
    const outside = s.markersAt === 'end';
    if (outside) {
      root.classList.add('mk-outside', s.markersSide === 'right' ? 'mk-right' : 'mk-left');
      const card = h('div', 'st-card');
      card.append(header.el, body);
      root.append(card);
    } else root.append(header.el, body);

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
      ...extraCols<StandRow>(s, () => ctx.units),
      // Flags and penalties outside the table: placed past the row's end (see .mk-outside).
      { id: 'markers', label: '', width: '0px', render: (c, r) => markersCell(c, r) },
    ];
    const order = [...enabledColumns(s.columns), ...(s.markersAt === 'end' ? ['markers'] : [])];
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

    let myClass = '';
    let emptySince = 0;
    const report = (m: string) => ctx.log?.(m);
    return {
      update(topic, data) {
        if (topic === 'session') return header.update(data as Session);
        if (topic !== 'standings') return;
        const d = data as Standings;
        // Your class, remembered for the moments the data has none (a reset, the camera on
        // another car, between sessions), so "My class only" does not empty.
        if (d.focusClass) myClass = d.focusClass;
        const focusClass = d.focusClass || myClass;
        let classes: StandClass[] = d.classes;
        if (s.mode === 'myClass') classes = classes.filter((c) => c.id === focusClass);
        // Nothing to show for a moment (no cars listed between sessions or while the game
        // reloads): keep the table as it is rather than shrink the window to nothing, which
        // looks as if the overlay was switched off. The game closing hides it as before.
        if (!classes.some((c) => c.rows.length) && sections.length) {
          if (!emptySince) {
            emptySince = Date.now();
            report('standings: no cars to show; keeping the last table (' + (d.classes.length ? `none in class ${focusClass || '?'}` : 'none listed') + ')');
          }
          return;
        }
        if (emptySince) {
          report(`standings: cars to show again after ${((Date.now() - emptySince) / 1000).toFixed(1)}s`);
          emptySince = 0;
        }
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
          if (modern && /^#[0-9a-f]{6}$/i.test(c.color)) sec.head.style.setProperty('--class-fg', contrast(c.color));
          else sec.head.style.removeProperty('--class-fg');
          text(sec.meta, [c.sof ? `SOF ${f.sof(c.sof)}` : '', `${c.count} cars`, c.bestLap ? f.lapTime(c.bestLap) : ''].filter(Boolean).join('  ·  '));
          const mine = c.id === focusClass || s.mode !== 'multiclass';
          const rows = pick(c.rows, mine ? (s.maxRows as number) : (s.otherClassRows as number));
          show(sec.el, rows.length > 0);
          // Modern look: cars within the close gap of the car ahead, joined by a battle line.
          const near = modern ? rows.map((r, j) => !!r && closeBehind(r, rows[j - 1])) : [];
          sec.table.render(
            rows,
            (r) => {
              const j = rows.indexOf(r);
              const pos = overall ? r.position : r.classPosition;
              return {
                player: r.isFocus,
                pit: (s.dimPitCars as boolean) && r.onPitRoad,
                out: (s.dimDisconnected as boolean) && !r.inWorld,
                close: modern && near[j],
                'bt-up': battles && near[j],
                'bt-down': battles && !!near[j + 1],
                p1: podium && pos === 1,
                p2: podium && pos === 2,
                p3: podium && pos === 3,
              };
            },
            (row, r) => row.style.setProperty('--class', stripeColor(r, s)),
          );
        });
        if (s.autoWidth) fit(sections.map((x) => x.table));
      },
    };
  },
};
