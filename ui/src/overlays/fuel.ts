import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { onColor } from '../lib/theme';
import type { Fuel } from '../lib/types';

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
];

const full = (v: Record<string, unknown>) => v.mode !== 'compact';

/** How the fuel looks against the finish. */
type State = 'ok' | 'marginal' | 'short' | 'none';

export const fuel: OverlayDef = {
  kind: 'fuel',
  name: 'Fuel calculator',
  description:
    'Laps of fuel left against laps to go, on a bar with the finish marked: cyan when you make the finish, amber when it is close, red with how much to add when you are short. Plus consumption, stops, pit window and more. Works for lap- and time-limited races; a compact mode shows just the essentials.',
  topics: ['fuel', 'session'],
  size: [340, 210],
  commonDefaults: { autoHeight: true },
  settings: [
    { key: 'mode', label: 'Layout', type: 'select', default: 'full', options: [['full', 'Full'], ['compact', 'Compact (laps, bar and fuel at the finish)']], group: 'Layout' },
    { key: 'showHeader', when: full, label: '“Fuel” title and status pill', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_level', when: full, label: 'Fuel in tank', type: 'bool', default: true, group: 'Layout' },
    { key: 'showGauge', label: 'Fuel bar', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_perLapAvg', when: full, label: 'Per lap (average)', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_lapsRemaining', when: full, label: 'Laps to go', type: 'bool', default: true, group: 'Layout' },
    { key: 'show_atFinish', label: 'Fuel at the finish', type: 'bool', default: true, group: 'Layout', help: 'What is left when you cross the line (minus: how much you are short)' },
    ...EXTRAS.map(([key, label, , on]) => ({ key: `show_${key}`, when: full, label, type: 'bool' as const, default: on, group: 'Extra details' })),
    { key: 'showHistory', when: full, label: 'Per-lap history bars', type: 'bool', default: false, group: 'Extra details' },
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
    const atFinishC = h('span', 'fl-num fl-at');

    if (compact) {
      const left = h('div', 'fl-c-left');
      left.append(hero, label('LAPS'));
      root.append(left, bar, atFinishC);
      show(atFinishC, s.show_atFinish !== false);
    } else {
      const top = h('div', 'fl-top');
      pill = h('span', 'fl-pill');
      top.append(label('FUEL'), pill);
      show(top, s.showHeader !== false);
      const main = h('div', 'fl-main');
      tank = figure('fl-tank');
      main.append(hero, h('span', 'fl-label fl-hero-label', 'LAPS LEFT'), tank.el);
      show(tank.el, s.show_level !== false);
      const row = h('div', 'fl-stats');
      for (const [key, title, on] of [
        ['perLapAvg', 'PER LAP', s.show_perLapAvg !== false],
        ['lapsRemaining', 'LAPS TO GO', s.show_lapsRemaining !== false],
        ['atFinish', 'AT FINISH', s.show_atFinish !== false],
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
      show(hist, !!s.showHistory);
      root.append(top, main, bar, row, more, hist);
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

        if (compact) {
          text(atFinishC, atFinish == null ? vol(d.level, 1).replace(` ${f.fuelUnit(u())}`, '') : signed(atFinish, false));
          atFinishC.title = atFinish == null ? 'Fuel in tank' : 'Fuel at the finish';
          return;
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
        if (stats.perLapAvg) stats.perLapAvg.el.title = d.estimateFrom === 'history' ? 'From your earlier sessions here until you have a clean lap' : `Average of ${d.lapsSampled} laps`;
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
