import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { DeltaRefs, G61Reference, Garage61, Player, Status } from '../lib/types';

/** Reference lap time at a lap distance (0–1), interpolated. */
function refAt(ref: G61Reference, pct: number): number {
  const n = ref.times.length - 1;
  const x = Math.max(0, Math.min(1, pct)) * n;
  const i = Math.min(n - 1, Math.floor(x));
  return ref.times[i] + (ref.times[i + 1] - ref.times[i]) * (x - i);
}

interface Pick {
  ref: G61Reference | null;
  label: string;
  /** Why the chosen lap is missing, or that a fallback is used. */
  note: string;
}

export const delta: OverlayDef = {
  kind: 'delta',
  name: 'Delta bar',
  description:
    'Live delta to a reference lap: your best this session, your all-time best (laps recorded by the app and your Garage 61 best), the fastest lap in one of your Garage 61 teams or all of them, or the fastest on Garage 61. Shows the delta, a bar, your predicted lap time and whether you are gaining or losing.',
  topics: ['player', 'delta', 'garage61', 'status'],
  size: [420, 96],
  commonDefaults: { hideWhenNotDriving: true },
  settings: [
    {
      key: 'reference',
      label: 'Compare with',
      type: 'select',
      default: 'allTime',
      options: [
        ['session', 'My best lap this session'],
        ['allTime', 'My all-time best (fastest of my laps here and on Garage 61)'],
        ['recorded', 'My all-time best recorded by Apex Overlays'],
        ['g61Personal', 'My Garage 61 best'],
        ['g61Team', 'Fastest in one of my Garage 61 teams'],
        ['g61Teams', 'Fastest in all my Garage 61 teams'],
        ['g61World', 'Fastest on Garage 61 (world)'],
      ],
      group: 'Reference lap',
      help: 'Garage 61 laps need Garage 61 set up (Settings → Garage 61) and only laps whose telemetry you can see can be compared with',
    },
    { key: 'team', when: (v) => v.reference === 'g61Team', label: 'Team', type: 'text', default: '', choices: 'g61Teams', placeholder: 'Set up Garage 61 to list your teams', group: 'Reference lap', help: 'For “Fastest in one of my Garage 61 teams”' },
    { key: 'fallback', when: (v) => v.reference !== 'session', label: 'Until that lap is available, use my best this session', type: 'bool', default: true, group: 'Reference lap' },
    { key: 'showRef', label: 'Reference lap (who and what time)', type: 'bool', default: true, group: 'What to show' },
    { key: 'showPredicted', label: 'Predicted lap time', type: 'bool', default: true, group: 'What to show' },
    { key: 'showTrend', label: 'Gaining / losing arrow', type: 'bool', default: true, group: 'What to show' },
    { key: 'showBar', label: 'Delta bar', type: 'bool', default: true, group: 'What to show' },
    { key: 'decimals', label: 'Delta decimals', type: 'number', default: 2, min: 1, max: 3, group: 'What to show' },
    { key: 'range', when: (v) => v.showBar !== false, label: 'Bar range', type: 'number', default: 1, min: 0.2, max: 5, step: 0.1, unit: 's', group: 'Bar', help: 'A delta this big fills half the bar' },
    { key: 'fasterSide', when: (v) => v.showBar !== false, label: 'Bar grows when faster to the', type: 'select', default: 'right', options: [['right', 'Right (slower to the left)'], ['left', 'Left (slower to the right)']], group: 'Bar' },
    { key: 'barHeight', when: (v) => v.showBar !== false, label: 'Bar height', type: 'number', default: 10, min: 3, max: 40, unit: 'px', group: 'Bar' },
    { key: 'fasterColor', label: 'Faster', type: 'color', default: '#2fd36b', group: 'Colours' },
    { key: 'slowerColor', label: 'Slower', type: 'color', default: '#ff4d4d', group: 'Colours' },
    { key: 'trendSeconds', when: (v) => v.showTrend !== false, label: 'Trend over', type: 'number', default: 1, min: 0.3, max: 5, step: 0.1, unit: 's', group: 'Advanced', help: 'How far back the gaining / losing arrow looks' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('delta');
    style(root, '--faster', s.fasterColor as string);
    style(root, '--slower', s.slowerColor as string);
    style(root, '--bar-h', `${s.barHeight}px`);
    toggle(root, 'faster-right', s.fasterSide === 'right');

    const top = h('div', 'dl-top');
    const trend = h('span', 'dl-trend');
    const value = h('span', 'dl-value');
    const pred = h('span', 'dl-pred');
    top.append(trend, value, pred);
    const bar = h('div', 'dl-bar');
    const fill = h('div', 'dl-fill');
    bar.append(fill);
    const ref = h('div', 'dl-ref');
    root.append(top, bar, ref);
    show(bar, !!s.showBar);
    show(pred, !!s.showPredicted);
    show(trend, !!s.showTrend);
    show(ref, !!s.showRef);

    let mine: DeltaRefs | null = null;
    let g61: Garage61 | null = null;
    let editing = false;
    let lap = -1;
    let invalid = true;
    const history: [number, number][] = [];

    const g61Ref = (key: string): [G61Reference | null, string] => {
      if (!g61 || g61.status !== 'ok') return [null, g61?.message || 'Garage 61 is not set up'];
      return [g61.references?.[key] ?? null, g61.referenceNotes?.[key] ?? 'loading…'];
    };

    const pick = (): Pick => {
      const team = s.team as string;
      let chosen: Pick;
      switch (s.reference) {
        case 'session':
          chosen = { ref: mine?.session ?? null, label: 'My best this session', note: 'no clean lap yet this session' };
          break;
        case 'recorded':
          chosen = { ref: mine?.allTime ?? null, label: 'My all-time best', note: 'no clean lap recorded for this car and track yet' };
          break;
        case 'g61Personal': {
          const [r, note] = g61Ref('personal');
          chosen = { ref: r, label: 'My Garage 61 best', note };
          break;
        }
        case 'g61Teams': {
          const [r, note] = g61Ref('teams');
          chosen = { ref: r, label: `Teams’ best · ${r?.driver ?? ''}`, note };
          break;
        }
        case 'g61Team': {
          const name = g61?.teams?.find((t) => t.slug === team)?.name ?? team;
          const [r, note] = team ? g61Ref(`team:${team}`) : [null, 'choose a team in the settings'];
          chosen = { ref: r, label: `${name} best · ${r?.driver ?? ''}`, note };
          break;
        }
        case 'g61World': {
          const [r, note] = g61Ref('world');
          chosen = { ref: r, label: `Garage 61 fastest · ${r?.driver ?? ''}`, note };
          break;
        }
        default: {
          // The fastest of everything that is yours.
          const [g] = g61Ref('personal');
          const all = [mine?.allTime, mine?.session, g].filter((r): r is G61Reference => !!r);
          const best = all.sort((a, b) => a.lapTime - b.lapTime)[0] ?? null;
          chosen = { ref: best, label: best === g ? 'My all-time best (Garage 61)' : 'My all-time best', note: 'no clean lap yet' };
        }
      }
      if (chosen.ref) return { ...chosen, note: '' };
      if (s.fallback && s.reference !== 'session' && mine?.session) return { ref: mine.session, label: 'My best this session', note: `${chosen.label.replace(/ · $/, '')}: ${chosen.note}` };
      return chosen;
    };

    const fmt = (v: number) => {
      const d = s.decimals as number;
      return `${v > 0 ? '+' : v < 0 ? '−' : '±'}${Math.abs(v).toFixed(d)}`;
    };

    const draw = (live: number | null, p: Pick) => {
      text(value, live == null ? '–' : fmt(live));
      toggle(root, 'gain', live != null && live < 0);
      toggle(root, 'loss', live != null && live > 0);
      const range = s.range as number;
      const w = live == null ? 0 : (Math.min(range, Math.abs(live)) / range) * 50;
      // Faster grows to one side (right by default), slower to the other.
      const toLeft = live != null && (live < 0) !== (s.fasterSide === 'right');
      style(fill, 'width', `${w}%`);
      style(fill, 'left', toLeft ? `${50 - w}%` : '50%');
      text(pred, live != null && p.ref ? f.lapTime(p.ref.lapTime + live) : '');
      // Trend: how the delta moved over the last second or so.
      const now = performance.now() / 1000;
      if (live != null) history.push([now, live]);
      while (history.length && now - history[0][0] > (s.trendSeconds as number)) history.shift();
      const change = live != null && history.length > 1 ? live - history[0][1] : 0;
      text(trend, Math.abs(change) < 0.01 ? '' : change < 0 ? '▼' : '▲');
      toggle(trend, 'gain', change < 0);
      toggle(trend, 'loss', change > 0);
      text(ref, p.ref ? `vs ${p.label.replace(/ · $/, '')} ${f.lapTime(p.ref.lapTime)}${p.note ? ' *' : ''}` : `No reference lap: ${p.note}`);
      ref.title = p.note;
    };

    const onPlayer = (pl: Player) => {
      const p = pick();
      if (pl.lap !== lap) {
        // A lap only counts from the line, on track and out of the pits.
        lap = pl.lap;
        invalid = pl.onPitRoad || !pl.onTrack;
        history.length = 0;
      }
      if (pl.onPitRoad || !pl.onTrack) invalid = true;
      const live = p.ref && !invalid && pl.lapTime > 0 ? pl.lapTime - refAt(p.ref, pl.lapPct) : null;
      draw(live, p);
    };

    const sample = () => {
      if (!(editing || preview) || ctx.latest.player?.onTrack) return;
      draw(-0.234, { ref: { driver: '', lapTime: 92.481, source: 'allTime', times: [] }, label: 'My all-time best', note: '' });
      text(trend, '▼');
      toggle(trend, 'gain', true);
    };

    return {
      update(topic, data) {
        if (topic === 'delta') mine = data as DeltaRefs;
        if (topic === 'garage61') g61 = data as Garage61;
        if (topic === 'status') editing = !!(data as Status).editMode;
        if (topic === 'player') onPlayer(data as Player);
        sample();
      },
    };
  },
};
