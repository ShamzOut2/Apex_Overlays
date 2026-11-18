// Race control: hand out penalties (drive-through, stop & go, time) that are shown
// in a box next to the driver's name. iRacing itself only reports "black flag",
// never which penalty it is, so leagues and streamers set it here.

import { h } from '../lib/dom';
import type { Penalty, StandRow } from '../lib/types';
import { flagEmoji } from '../overlays/common';
import type { Page } from './pages';
import { save, state, uid } from './state';

const label = (p: Penalty) => {
  const s = Math.round(p.seconds);
  if (p.kind === 'dt') return 'DT';
  if (p.kind === 'sg') return s > 0 ? `S&G ${s}s` : 'S&G';
  if (p.kind === 'time') return `+${s}s`;
  return p.note || 'PEN';
};

function btn(t: string, fn: () => void, cls = '') {
  const b = h('button', cls, t);
  b.onclick = (e) => {
    e.preventDefault();
    fn();
  };
  return b;
}

const FLAG_NAMES: Record<string, string> = { yellow: 'caused yellow', black: 'black flag', disqualify: 'DQ', repair: 'meatball', slow: 'slow down', blue: 'blue flag' };

export const raceControlPage: Page = {
  id: 'racecontrol',
  title: 'Race control',
  render(el) {
    const cfg = state.config!;
    cfg.penalties ??= [];
    const rerender = () => this.render(el);
    const set = (r: StandRow, kind: Penalty['kind'], seconds: number, note = '') => {
      cfg.penalties = cfg.penalties.filter((p) => p.customerId !== r.userId);
      cfg.penalties.push({ id: uid('pen'), customerId: r.userId, name: r.name, kind, seconds, note });
      save(true);
      rerender();
    };
    const clear = (customerId: number) => {
      cfg.penalties = cfg.penalties.filter((p) => p.customerId !== customerId);
      save(true);
      rerender();
    };

    const intro = h('section', 'card');
    intro.append(
      h('h2', '', 'Penalties'),
      h(
        'p',
        'dim',
        'Give a driver a penalty (DT, S&G 40s, +5s) and it shows next to their name in the relative and standings. Black-flagged cars serving in the pits show DT or an S&G timer automatically.',
      ),
    );

    const active = h('section', 'card');
    active.append(h('h2', '', 'Active penalties'));
    if (!cfg.penalties.length) active.append(h('p', 'dim', 'None.'));
    else {
      const t = h('table', 'drivers');
      for (const p of cfg.penalties) {
        const tr = h('tr');
        const box = h('span', 'pen-chip', label(p));
        const td = h('td');
        td.append(box);
        const act = h('td');
        act.append(btn('Clear', () => clear(p.customerId), 'mini'));
        tr.append(td, h('td', '', p.name || String(p.customerId)), act);
        t.append(tr);
      }
      active.append(t, btn('Clear all', () => {
        cfg.penalties = [];
        save(true);
        rerender();
      }, 'danger'));
    }

    const drivers = h('section', 'card');
    drivers.append(h('h2', '', 'Drivers in this session'));
    const dl = h('a', 'button mini', 'Download session info');
    dl.setAttribute('href', '/api/session-info');
    dl.setAttribute('download', 'apex-session-info.yaml');
    drivers.append(
      h('p', 'dim small', 'Flags come from iRacing, or are looked up through the iRacing Data API (League builder → Import from iRacing). You can also set them in the League builder.'),
      dl,
    );
    const rows = (state.standings?.classes.flatMap((c) => c.rows) ?? []).sort((a, b) => a.position - b.position);
    if (!rows.length) drivers.append(h('p', 'dim', 'Join a session to see the drivers.'));
    else {
      const t = h('table', 'drivers');
      const head = h('tr');
      for (const x of ['Pos', '#', 'Flag', 'Driver', 'Now', 'Penalty']) head.append(h('th', '', x));
      t.append(head);
      for (const r of rows) {
        const tr = h('tr');
        const flagTd = h('td');
        const emoji = flagEmoji(r.country);
        flagTd.append(emoji ? h('span', 'flag-emoji', emoji) : h('span', 'dim small', r.flair ? `“${r.flair}”` : '—'));
        flagTd.title = `iRacing flair: ${r.flair || '(none)'}${r.country ? ` → ${r.country}` : ' (not recognised)'}`;
        const now = h('td', 'dim small', [...r.flags.map((f) => FLAG_NAMES[f] ?? f), r.penalty ?? ''].filter(Boolean).join(', '));
        const pen = h('td');
        const secs = h('input');
        secs.type = 'number';
        secs.value = '30';
        secs.min = '0';
        secs.style.width = '64px';
        const sec = () => Math.max(0, parseFloat(secs.value) || 0);
        const existing = cfg.penalties.find((p) => p.customerId === r.userId);
        if (existing) pen.append(h('span', 'pen-chip', label(existing)), ' ');
        pen.append(
          btn('DT', () => set(r, 'dt', 0), 'mini'),
          ' ',
          btn('S&G', () => set(r, 'sg', sec()), 'mini'),
          ' ',
          btn('+Time', () => set(r, 'time', sec()), 'mini'),
          ' ',
          secs,
          ' s ',
        );
        if (existing) pen.append(btn('Clear', () => clear(r.userId), 'mini danger'));
        tr.append(h('td', '', String(r.position)), h('td', '', r.number), flagTd, h('td', '', r.name), now, pen);
        t.append(tr);
      }
      drivers.append(t);
    }
    el.replaceChildren(intro, active, drivers);
  },
};
