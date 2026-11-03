import { h, show, style, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { G61Reference, Garage61, Player, TrackMap } from '../lib/types';

/** Reference lap time at a lap distance (0–1), interpolated. */
function refAt(ref: G61Reference, pct: number): number {
  const n = ref.times.length - 1;
  const x = Math.max(0, Math.min(1, pct)) * n;
  const i = Math.min(n - 1, Math.floor(x));
  return ref.times[i] + (ref.times[i + 1] - ref.times[i]) * (x - i);
}

export const garage61: OverlayDef = {
  kind: 'garage61',
  name: 'Garage 61',
  description:
    'Live delta to your Garage 61 personal best (or your team’s best lap), sector by sector, plus your personal best and your teams’ leaderboard for the current car and track.',
  topics: ['garage61', 'player', 'trackmap'],
  size: [300, 230],
  settings: [
    { key: 'showDelta', label: 'Live delta to the reference lap', type: 'bool', default: true, group: 'Delta', help: 'Choose your best or the team’s best on the Garage 61 settings page' },
    { key: 'deltaRange', when: (v) => v.showDelta !== false, label: 'Delta bar range', type: 'number', default: 1, min: 0.2, max: 5, step: 0.1, unit: 's', group: 'Delta' },
    { key: 'showSectors', when: (v) => v.showDelta !== false, label: 'Sector deltas', type: 'bool', default: true, group: 'Delta' },
    { key: 'rows', label: 'Team leaderboard rows', type: 'number', default: 5, min: 0, max: 10, group: 'Layout' },
    { key: 'showDates', label: 'Show lap dates', type: 'bool', default: false, group: 'Layout' },
    { key: 'showSessionDelta', label: 'Compare with my session best', type: 'bool', default: true, group: 'Layout' },
    { key: 'showFilters', label: 'Which laps were searched', type: 'bool', default: false, group: 'Layout', help: 'e.g. “Current season · 25–35 °C · dry”' },
  ],
  commonDefaults: { autoHeight: true },
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('g61');
    const header = h('div', 'header');
    const title = h('span', 'title', 'Garage 61');
    const info = h('span', 'info');
    header.append(title, info);
    const status = h('div', 'g61-status dim');

    const delta = h('div', 'g61-delta');
    const refLine = h('div', 'g61-ref dim');
    const dVal = h('div', 'g61-dval');
    const bar = h('div', 'g61-bar');
    const fill = h('div', 'g61-fill');
    bar.append(fill);
    const sectors = h('div', 'g61-sectors');
    delta.append(refLine, dVal, bar, sectors);

    const pb = h('div', 'kv');
    const pbK = h('div', 'k', 'Personal best');
    const pbV = h('div', 'v');
    const sbK = h('div', 'k', 'Session best');
    const sbV = h('div', 'v');
    pb.append(pbK, pbV, sbK, sbV);
    show(sbK, s.showSessionDelta as boolean);
    show(sbV, s.showSessionDelta as boolean);
    const filters = h('div', 'g61-filters dim');
    const list = h('div', 'g61-list');
    root.append(header, status, delta, pb, filters, list);
    let g: Garage61 | null = null;
    let best: number | null = null;
    let starts: number[] = [0, 1 / 3, 2 / 3];

    // Live delta state for the lap in progress.
    let lap = -1;
    let invalid = true;
    let sector = -1;
    let sectorStartDelta = 0;
    let sectorDeltas: (number | null)[] = [];
    let live: number | null = null;

    const sectorChips = () => {
      while (sectors.children.length < starts.length) sectors.append(h('span', 'g61-sec'));
      while (sectors.children.length > starts.length) sectors.lastElementChild!.remove();
      [...sectors.children].forEach((el, i) => {
        const c = el as HTMLElement;
        const cur = i === sector && live != null && !invalid;
        const v = cur ? live! - sectorStartDelta : sectorDeltas[i];
        text(c, `S${i + 1} ${v == null ? '–' : f.signed(v, 2)}`);
        c.classList.toggle('cur', cur);
        c.classList.toggle('gain', v != null && v < 0);
        c.classList.toggle('loss', v != null && v > 0);
      });
    };

    const renderDelta = () => {
      const ref = g?.status === 'ok' ? g.reference : null;
      show(delta, !!s.showDelta && g?.status === 'ok');
      if (!s.showDelta || g?.status !== 'ok') return;
      if (!ref) {
        text(refLine, g.referenceNote || 'No reference lap');
        text(dVal, '');
        show(bar, false);
        show(sectors, false);
        return;
      }
      const who = ref.source === 'team' ? `Team best · ${ref.driver}` : 'My best';
      text(refLine, `vs ${who} ${f.lapTime(ref.lapTime)}${g.referenceNote ? ' *' : ''}`);
      refLine.title = g.referenceNote;
      text(dVal, live == null ? '–' : f.signed(live, 2));
      dVal.classList.toggle('gain', live != null && live < 0);
      dVal.classList.toggle('loss', live != null && live > 0);
      show(bar, true);
      const range = s.deltaRange as number;
      const w = live == null ? 0 : (Math.min(range, Math.abs(live)) / range) * 50;
      style(fill, 'width', `${w}%`);
      style(fill, 'left', live != null && live < 0 ? `${50 - w}%` : '50%');
      fill.classList.toggle('gain', live != null && live < 0);
      show(sectors, !!s.showSectors);
      if (s.showSectors) sectorChips();
    };

    const onPlayer = (p: Player) => {
      const ref = g?.status === 'ok' ? g.reference : null;
      const [prevLive, prevInvalid] = [live, invalid];
      if (p.lap !== lap) {
        // A new lap counts only if it starts on track, out of the pits.
        lap = p.lap;
        invalid = p.onPitRoad || !p.onTrack;
      }
      if (p.onPitRoad || !p.onTrack) invalid = true;
      live = ref && !invalid && p.lapTime > 0 ? p.lapTime - refAt(ref, p.lapPct) : null;
      const k = Math.max(0, starts.filter((st) => st <= p.lapPct).length - 1);
      if (k !== sector) {
        if (sector >= 0 && k === (sector + 1) % starts.length) {
          // The sector just finished is final; across the line the lap timer has already
          // restarted, so the last sector ends with the delta just before the line.
          const [end, bad] = k === 0 ? [prevLive, prevInvalid] : [live, invalid];
          sectorDeltas[sector] = end != null && !bad ? end - sectorStartDelta : null;
        }
        sectorStartDelta = k === 0 ? 0 : (live ?? 0);
        sector = k;
      }
      renderDelta();
    };

    const render = () => {
      if (!g) return;
      const ok = g.status === 'ok';
      show(status, !ok);
      show(pb, ok);
      show(list, ok);
      show(filters, ok && !!s.showFilters && !!g.filters);
      text(filters, g.filters);
      text(info, ok ? `${g.car}` : '');
      text(status, g.status === 'loading' ? 'Loading…' : g.message || g.status);
      renderDelta();
      if (!ok) return;
      text(pbV, g.personalBest ? f.lapTime(g.personalBest.lapTime) : 'No lap yet');
      const d = best != null && g.personalBest ? best - g.personalBest.lapTime : null;
      text(sbV, best ? `${f.lapTime(best)}${d != null ? ` (${f.signed(d, 3)})` : ''}` : '–');
      sbV.classList.toggle('good', d != null && d < 0);
      const rows = g.leaderboard.slice(0, s.rows as number);
      const top = rows[0]?.lapTime;
      while (list.children.length < rows.length) {
        const r = h('div', 'g61-row');
        r.append(h('span', 'p'), h('span', 'n'), h('span', 't'), h('span', 'gap dim'));
        list.append(r);
      }
      [...list.children].forEach((el, i) => {
        const r = el as HTMLElement;
        show(r, i < rows.length);
        if (i >= rows.length) return;
        const l = rows[i];
        const [p, n, t, gp] = [...r.children] as HTMLElement[];
        text(p, String(i + 1));
        text(n, l.driver + (s.showDates && l.date ? `  ${l.date}` : ''));
        text(t, f.lapTime(l.lapTime));
        text(gp, i > 0 && top ? f.signed(l.lapTime - top, 3) : '');
        r.classList.toggle('me', l.isMe);
      });
    };

    return {
      update(topic, data) {
        if (topic === 'garage61') {
          const key = (x: Garage61 | null) => (x?.reference ? `${x.reference.driver}|${x.reference.lapTime}` : '');
          const prev = key(g);
          g = data as Garage61;
          // A different reference lap: the sector deltas no longer apply.
          if (key(g) !== prev) sectorDeltas = [];
          render();
        }
        if (topic === 'trackmap') {
          const st = [...((data as TrackMap).sectors ?? [])].filter((x) => x >= 0 && x < 1).sort((a, b) => a - b);
          if (st.length && st[0] > 0) st.unshift(0);
          const next = st.length >= 2 ? st : [0, 1 / 3, 2 / 3];
          if (next.join() !== starts.join()) {
            starts = next;
            sectorDeltas = [];
            sector = -1;
          }
        }
        if (topic === 'player') {
          const p = data as Player;
          onPlayer(p);
          if (p.bestLap !== best) {
            best = p.bestLap;
            render();
          }
        }
      },
    };
  },
};
