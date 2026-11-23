import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { onColor } from '../lib/theme';
import type { Fuel, Session } from '../lib/types';

/** Extra details, shown under the main figures in the same style: key, label, on by default. */
const EXTRAS: [string, string, string, boolean][] = [
  ['perLapLast', 'Last lap', 'LAST LAP', true],
  ['stops', 'Stops required', 'STOPS', true],
  ['window', 'Pit window', 'PIT WINDOW', true],
  ['expected', 'Usual for this car & track', 'USUAL HERE', false],
  ['perLapMax', 'Max per lap', 'MAX PER LAP', false],
  ['fuelToFinish', 'Needed to finish (with your margin)', 'TO FINISH', false],
  ['fuelToAdd', 'Add at next stop', 'ADD AT STOP', false],
  ['targetPerLap', 'Target per lap (no stop)', 'TARGET / LAP', false],
  ['savePerLap', 'Save per lap to skip a stop', 'SAVE / LAP', false],
  ['lapsPerTank', 'Laps per full tank', 'LAPS / TANK', false],
  ['usePerHour', 'Use per hour', 'PER HOUR', false],
  ['estLaps', 'Estimated laps left (time left ÷ your lap time)', 'EST. LAPS', false],
];

const full = (v: Record<string, unknown>) => v.mode !== 'compact';
/** Details (average, laps to go, extras) show in the full layout, and in the compact one when asked for. */
const details = (v: Record<string, unknown>) => full(v) || !!v.compactDetails;

/** How the fuel looks against the finish. */
type State = 'ok' | 'marginal' | 'short' | 'none';

export const fuel: OverlayDef = {
  kind: 'fuel',
  name: 'Fuel calculator',
  description:
    'Laps of fuel left against laps to go: cyan when you make it, amber when close, red with how much to add. Plus use per lap, stops and the pit window.',
  topics: ['fuel', 'session'],
  size: [340, 210],
  commonDefaults: { autoHeight: true },
  settings: [
    { key: 'mode', label: 'Layout', type: 'select', default: 'full', options: [['full', 'Full'], ['compact', 'Compact (laps, bar and fuel at the finish)']], group: 'Layout' },
    { key: 'compactDetails', when: (v) => !full(v), label: 'Details under the bar', type: 'bool', default: false, group: 'Layout', help: 'The average, laps to go and the extra details you pick' },
    { key: 'showHeader', when: full, label: '“Fuel” title and status pill', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_level', when: full, label: 'Fuel in tank', type: 'bool', default: true, group: 'Layout' },
    { key: 'showGauge', label: 'Fuel bar', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_perLapAvg', when: details, label: 'Average per lap', type: 'bool', default: true, group: 'Layout' },
    { key: 'avgLaps', when: details, label: 'Laps in the average', type: 'number', default: 5, min: 1, max: 20, unit: 'laps', group: 'Layout', shared: 'fuel.avgLaps', help: 'Your last laps. Everything the calculator works out uses this average (same as Strategy → Fuel)' },
    { key: 'anyLaps', when: details, label: 'Average every lap, clean or not', type: 'bool', default: false, group: 'Layout', shared: 'fuel.anyLaps', help: 'Off: clean laps only (no pit-lane or caution laps). On: those count too. A lap you refuelled on never does, as what it used cannot be measured (same as Strategy → Fuel)' },
    { key: 'show_lapsRemaining', when: details, label: 'Laps to go', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_atFinish', label: 'Fuel at the finish', type: 'bool', default: true, group: 'Layout', help: 'What is left when you cross the line (minus: how much you are short)' },
    {
      key: 'show_race',
      label: 'Race estimate before the start',
      type: 'bool',
      default: true,
      group: 'Race estimate',
      help: 'In practice, qualifying and on the grid: how many laps the race will be (from its laps, or its time and your lap time), the fuel for all of it and the stops',
    },
    {
      key: 'raceMinutes',
      when: (v) => v.show_race !== false,
      label: 'Race length when the game does not say',
      type: 'number',
      default: 0,
      min: 0,
      max: 1440,
      unit: 'min',
      group: 'Race estimate',
      shared: 'fuel.raceMinutes',
      help: 'Le Mans Ultimate tells the race length only once in the race session. 0: none',
    },
    { key: 'raceLaps', when: (v) => v.show_race !== false, label: 'Or in laps', type: 'number', default: 0, min: 0, max: 2000, unit: 'laps', group: 'Race estimate', shared: 'fuel.raceLaps' },
    ...EXTRAS.map(([key, label, , on]) => ({ key: `show_${key}`, when: details, label, type: 'bool' as const, default: on, group: 'Extra details' })),
    { key: 'estDecimals', when: (v) => details(v) && !!v.show_estLaps, label: 'Estimated laps: decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Extra details' },
    { key: 'showHistory', when: full, label: 'Per-lap history bars', type: 'bool', default: false, group: 'Extra details' },
    {
      key: 'showCoach',
      label: 'Fuel save coach (this lap)',
      type: 'bool',
      default: true,
      group: 'Save coach',
      help: 'As you drive: how this lap stands against the fuel per lap you can use (to skip a stop when saving would, else your average). Green: under it; red: over. Judged by how your last clean lap used fuel round the lap, so straights and corners count as they should',
    },
    { key: 'coachTolerance', when: (v) => v.showCoach !== false, label: 'Counts as on target within', type: 'number', default: 0.02, min: 0, max: 0.5, step: 0.01, unit: 'L', group: 'Save coach' },
    { key: 'coachProjected', when: (v) => v.showCoach !== false, label: 'What the lap will use at this rate', type: 'bool', default: true, group: 'Save coach' },
    { key: 'marginalLaps', label: 'Amber when the spare is under', type: 'number', default: 0.5, min: 0, max: 5, step: 0.1, unit: 'laps', group: 'Status', help: 'Fuel left at the finish, in laps, below which it shows as marginal' },
    { key: 'warnLaps', label: 'Outside races, red when fuel is under', type: 'number', default: 2, min: 0, max: 10, step: 0.5, unit: 'laps', group: 'Status', help: 'Practice and sessions without a set length' },
    { key: 'okColor', label: 'Makes the finish', type: 'color', default: '#3dd6f5', group: 'Colours' },
    { key: 'marginalColor', label: 'Marginal', type: 'color', default: '#ffb020', group: 'Colours' },
    { key: 'shortColor', label: 'Short (pit needed)', type: 'color', default: '#ff5a3c', group: 'Colours' },
    { key: 'lineColor', label: 'Finish line on the bar', type: 'color', default: '#ffffff', group: 'Colours' },
    { key: 'segments', when: (v) => v.showGauge !== false && full(v), label: 'Bar segments', type: 'number', default: 20, min: 6, max: 40, group: 'Bar', help: 'One lap per segment, more laps per segment in long races' },
    { key: 'compactSegments', when: (v) => v.showGauge !== false && !full(v), label: 'Bar segments', type: 'number', default: 12, min: 4, max: 30, group: 'Bar' },
    { key: 'decimals', label: 'Decimals per lap', type: 'number', default: 2, min: 0, max: 3, group: 'Format' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const u = () => ctx.units;
    const dec = s.decimals as number;
    const compact = s.mode === 'compact';
    root.classList.add('fuel', compact ? 'fl-compact' : 'fl-full');
    style(root, '--fl-ok', s.okColor as string);
    style(root, '--fl-marginal', s.marginalColor as string);
    style(root, '--fl-short', s.shortColor as string);
    style(root, '--fl-line', s.lineColor as string);

    const label = (t: string) => h('span', 'fl-label', t);
    const unit = () => h('span', 'fl-unit', f.fuelUnit(u()));
    /** A figure with a small unit after it. */
    const figure = (cls: string) => {
      const el = h('span', `fl-num ${cls}`);
      const v = h('span');
      el.append(v, unit());
      return { el, v };
    };
    /** Label over value, as in the bottom rows. */
    const stat = (title: string) => {
      const el = h('div', 'fl-stat');
      const v = h('div', 'fl-num fl-val');
      el.append(label(title), v);
      return { el, v };
    };

    // The bar: segments filled to the laps of fuel, a line at the finish.
    const nSeg = Math.round((compact ? s.compactSegments : s.segments) as number);
    const bar = h('div', 'fl-bar');
    const segs = Array.from({ length: nSeg }, () => h('div', 'fl-seg'));
    const finish = h('div', 'fl-finish');
    bar.append(...segs, finish);
    show(bar, s.showGauge !== false);

    const hero = h('span', 'fl-num fl-hero');
    let pill: HTMLElement | null = null;
    let tank: ReturnType<typeof figure> | null = null;
    const stats: Record<string, ReturnType<typeof stat>> = {};
    const extras: Record<string, ReturnType<typeof stat>> = {};
    const hist = h('div', 'fl-history');
    // The race ahead, before the start.
    const raceRow = h('div', 'fl-stats fl-race');
    const raceStat = (title: string) => {
      const st = stat(title);
      raceRow.append(st.el);
      return st;
    };
    const raceLaps = raceStat('RACE LAPS');
    const raceFuel = raceStat('RACE FUEL');
    const raceStops = raceStat('RACE STOPS');
    show(raceRow, false);
    const atFinishC = h('span', 'fl-num fl-at');
    // Save coach: this lap against the per lap you can use.
    const coach = h('div', 'fl-coach');
    const coachLabel = h('span', 'fl-label', 'THIS LAP');
    const coachVal = h('span', 'fl-num fl-coach-val');
    const coachMeter = h('div', 'fl-coach-meter');
    const coachFill = h('div', 'fl-coach-fill');
    coachMeter.append(coachFill, h('div', 'fl-coach-mid'));
    const coachNote = h('span', 'fl-coach-note');
    coach.append(coachLabel, coachVal, coachMeter, coachNote);
    show(coach, false);

    /** The rows of figures: the average, laps to go and fuel at the finish, then the extras. */
    const detailRows = (withAtFinish: boolean) => {
      const row = h('div', 'fl-stats');
      for (const [key, title, on] of [
        ['perLapAvg', 'AVG / LAP', s.show_perLapAvg !== false],
        ['lapsRemaining', 'LAPS TO GO', s.show_lapsRemaining !== false],
        ['atFinish', 'AT FINISH', withAtFinish && s.show_atFinish !== false],
      ] as const) {
        if (!on) continue;
        stats[key] = stat(title);
        row.append(stats[key].el);
      }
      const more = h('div', 'fl-stats fl-extra');
      for (const [key, , title] of EXTRAS) {
        if (!s[`show_${key}`]) continue;
        extras[key] = stat(title);
        more.append(extras[key].el);
      }
      show(row, row.childElementCount > 0);
      show(more, more.childElementCount > 0);
      return [row, more];
    };

    if (compact) {
      const left = h('div', 'fl-c-left');
      left.append(hero, label('LAPS'));
      const line = h('div', 'fl-c-line');
      line.append(left, bar, atFinishC);
      root.append(line);
      show(atFinishC, s.show_atFinish !== false);
      // Fuel at the finish is already on the line.
      if (s.compactDetails) root.append(...detailRows(false));
      root.append(coach, raceRow);
    } else {
      const top = h('div', 'fl-top');
      pill = h('span', 'fl-pill');
      top.append(label('FUEL'), pill);
      show(top, s.showHeader !== false);
      const main = h('div', 'fl-main');
      tank = figure('fl-tank');
      main.append(hero, h('span', 'fl-label fl-hero-label', 'LAPS LEFT'), tank.el);
      show(tank.el, s.show_level !== false);
      const [row, more] = detailRows(true);
      show(hist, !!s.showHistory);
      root.append(top, main, bar, coach, row, more, raceRow, hist);
    }

    const vol = (l: number | null | undefined, d = dec) => (l == null ? '–' : `${f.fuel(l, u()).toFixed(d)} ${f.fuelUnit(u())}`);
    const signed = (l: number, withUnit: boolean) => {
      const v = f.fuel(l, u());
      const n = Math.abs(v) < 0.05 ? '0.0' : `${v > 0 ? '+' : '−'}${Math.abs(v).toFixed(1)}`;
      return withUnit ? `${n} ${f.fuelUnit(u())}` : n;
    };
    const laps = (v: number | null) => (v == null ? '–' : Math.abs(v - Math.round(v)) < 0.05 ? String(Math.round(v)) : v.toFixed(1));

    return {
      update(topic, data) {
        if (topic === 'session') {
          // Timed sessions: the time left over your lap time.
          const est = (data as Session).estLapsLeft;
          if (extras.estLaps) text(extras.estLaps.v, est == null ? '–' : `~${est.toFixed(s.estDecimals as number)}`);
          return;
        }
        if (topic !== 'fuel') return;
        const d = data as Fuel;
        const perLap = d.perLapAvg;
        const inTank = d.lapsInTank;
        const toGo = d.lapsRemaining;
        const atFinish = perLap != null && toGo != null ? d.level - toGo * perLap : null;
        let state: State;
        if (atFinish == null || inTank == null || toGo == null) state = inTank != null && inTank < (s.warnLaps as number) ? 'short' : 'none';
        else if (atFinish < 0) state = 'short';
        else if (inTank - toGo < (s.marginalLaps as number)) state = 'marginal';
        else state = 'ok';
        for (const st of ['ok', 'marginal', 'short', 'none']) toggle(root, `st-${st}`, st === state);
        const tone = { ok: s.okColor, marginal: s.marginalColor, short: s.shortColor, none: s.okColor }[state] as string;
        style(root, '--fl-tone-text', onColor(tone));

        text(hero, inTank == null ? '–' : inTank.toFixed(1));

        // Bar: one lap per segment, or more in long races so that the finish fits.
        if (s.showGauge !== false) {
          let fill: number;
          let line: number | null = null;
          if (inTank != null && toGo != null) {
            // Full: the longer of fuel and race fills up to 90% of the bar. Compact: about half,
            // so the finish sits mid-bar with room either side.
            const perSeg = Math.max(compact ? 0 : 1, Math.max(toGo, inTank, 0.1) / (nSeg * (compact ? 0.6 : 0.9)));
            fill = inTank / perSeg;
            line = Math.min(nSeg, toGo / perSeg);
          } else {
            // No race length: the tank's level.
            fill = (d.capacity > 0 ? Math.min(1, d.level / d.capacity) : d.pct) * nSeg;
          }
          segs.forEach((el, i) => el.style.setProperty('--p', String(Math.max(0, Math.min(1, fill - i)).toFixed(3))));
          show(finish, line != null);
          // On a segment boundary: the gaps between segments count too.
          if (line != null) finish.style.left = `calc((100% + var(--fl-gap)) * ${(line / nSeg).toFixed(4)} - var(--fl-gap) / 2)`;
        }

        // Save coach: green under the per lap you can use, red over it.
        const c = s.showCoach !== false ? d.coach : null;
        show(coach, !!c);
        if (c) {
          const tol = s.coachTolerance as number;
          const tone = c.delta >= -tol ? (c.delta > tol ? 'under' : 'even') : 'over';
          coach.dataset.tone = tone;
          text(coachVal, signed(c.delta, true));
          // A tenth of a lap's fuel either way fills the meter.
          const span = Math.max(0.05, c.perLap * 0.1);
          const p = Math.max(-1, Math.min(1, c.delta / span));
          coachFill.style.left = p >= 0 ? '50%' : `${50 + p * 50}%`;
          coachFill.style.width = `${Math.abs(p) * 50}%`;
          const against = c.against === 'save' ? 'save target' : 'average';
          text(coachNote, s.coachProjected !== false && c.projected != null ? `→ ${vol(c.projected, 2)} vs ${vol(c.perLap, 2)} ${against}` : `vs ${vol(c.perLap, 2)} ${against}`);
        }

        const r = s.show_race !== false ? d.race : null;
        show(raceRow, !!r);
        if (r) {
          text(raceLaps.v, `${r.exact ? '' : '~'}${r.laps}`);
          raceLaps.el.title = r.exact
            ? `A ${r.laps}-lap race`
            : `${r.minutes != null ? `${r.minutes} minutes` : 'The race'} at ${f.lapTime(r.lapTime)} a lap${r.from === 'settings' ? ' (the race length from your settings)' : ''}`;
          text(raceFuel.v, vol(r.fuel, 1));
          raceFuel.el.title = r.fuel == null ? 'Needs a lap to know your use per lap' : `For every lap plus your ${d.marginLaps}-lap margin. Start with ${vol(r.startFuel, 1)}`;
          text(raceStops.v, r.stops == null ? '–' : String(r.stops));
          raceStops.el.title = 'Starting with a full tank';
        }

        if (compact) {
          text(atFinishC, atFinish == null ? vol(d.level, 1).replace(` ${f.fuelUnit(u())}`, '') : signed(atFinish, false));
          atFinishC.title = atFinish == null ? 'Fuel in tank' : 'Fuel at the finish';
          if (!s.compactDetails) return;
        }

        if (pill) {
          let msg: string;
          if (perLap == null) msg = 'WAITING FOR A CLEAN LAP';
          else if (state === 'ok') msg = 'MAKES THE FINISH';
          else if (state === 'marginal') msg = 'MARGINAL';
          else if (state === 'short' && atFinish != null) {
            const add = d.fuelToAdd != null && d.fuelToAdd > 0 ? d.fuelToAdd : -atFinish;
            msg = `${d.stops != null && d.stops > 1 ? `${d.stops} STOPS` : 'PIT'} · ADD ${vol(add, 1)}`;
          } else if (state === 'short') msg = 'LOW FUEL';
          else msg = d.estimateFrom === 'history' ? 'FROM EARLIER SESSIONS' : `${d.lapsSampled} LAPS SAMPLED`;
          text(pill, msg);
          toggle(pill, 'neutral', state === 'none');
          pill.title = state === 'short' && d.fuelToAdd != null ? 'Includes your safety margin (Strategy → Fuel)' : '';
        }
        if (tank) text(tank.v, f.fuel(d.level, u()).toFixed(1));

        const set = (cells: Record<string, ReturnType<typeof stat>>, k: string, v: string) => cells[k] && text(cells[k].v, v);
        set(stats, 'perLapAvg', perLap == null ? '–' : `${vol(perLap)}${d.estimateFrom === 'history' ? '*' : ''}`);
        if (stats.perLapAvg)
          stats.perLapAvg.el.title =
            d.estimateFrom === 'history' ? 'From your earlier sessions here until you have a clean lap' : `Average of your last ${Math.min(d.avgLaps, d.lapsSampled)} ${d.anyLaps ? '' : 'clean '}laps (set to ${d.avgLaps})`;
        set(stats, 'lapsRemaining', laps(toGo));
        set(stats, 'atFinish', atFinish == null ? '–' : signed(atFinish, true));

        const x = d.expected;
        set(extras, 'expected', x ? vol(x.perLap) : '–');
        if (extras.expected) extras.expected.el.title = x ? `${x.car} at ${x.track}: average of your recent laps in earlier sessions (${x.laps} laps, highest ${vol(x.max)}). Last driven ${new Date(x.updated * 1000).toLocaleDateString()}.` : 'Not driven here yet';
        set(extras, 'perLapLast', vol(d.perLapLast));
        set(extras, 'perLapMax', vol(d.perLapMax));
        set(extras, 'fuelToFinish', vol(d.fuelToFinish, 1));
        set(extras, 'fuelToAdd', d.stops === 0 ? 'None' : vol(d.fuelToAdd, 1));
        set(extras, 'stops', d.stops == null ? '–' : String(d.stops));
        set(extras, 'window', d.windowOpensLap != null && d.windowClosesLap != null ? `L${d.windowOpensLap}–L${d.windowClosesLap}` : d.stops === 0 ? 'No stop' : '–');
        set(extras, 'targetPerLap', vol(d.targetPerLap));
        set(extras, 'savePerLap', d.savePerLap == null ? '–' : vol(d.savePerLap));
        set(extras, 'lapsPerTank', laps(d.lapsPerTank));
        set(extras, 'usePerHour', `${d.usePerHour.toFixed(1)} kg/h`);

        if (s.showHistory) {
          const max = Math.max(...d.history, 0.001);
          const hl = [...d.history].reverse();
          while (hist.children.length < hl.length) hist.append(h('div', 'fl-hbar'));
          [...hist.children].forEach((c, i) => {
            const el = c as HTMLElement;
            show(el, i < hl.length);
            if (i < hl.length) {
              el.style.height = `${(hl[i] / max) * 100}%`;
              el.title = vol(hl[i]);
            }
          });
        }
      },
    };
  },
};
