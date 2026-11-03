import { h, show, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Incident, Incidents, Status } from '../lib/types';

const KINDS: [Incident['kind'], string][] = [
  ['offTrack', 'Off track'],
  ['lossOfControl', 'Loss of control'],
  ['objectContact', 'Wall / object'],
  ['carContact', 'Car contact'],
  ['heavyContact', 'Heavy contact'],
];

/** Shown while positioning the overlay. */
const SAMPLE: Incidents = {
  total: 7,
  limit: 17,
  canReplay: false,
  sessionNum: 2,
  list: [
    { points: 1, kind: 'offTrack', label: 'Off track', total: 1, lap: 3, sessionNum: 2, sessionName: 'RACE', sessionTime: 312, involved: [] },
    { points: 2, kind: 'lossOfControl', label: 'Loss of control', total: 3, lap: 7, sessionNum: 2, sessionName: 'RACE', sessionTime: 790, involved: [] },
    {
      points: 4,
      kind: 'carContact',
      label: 'Contact with another car',
      total: 7,
      lap: 11,
      sessionNum: 2,
      sessionName: 'RACE',
      sessionTime: 1255,
      involved: [{ idx: 4, name: 'Jamie Clarke', number: '27', dist: -1.8, confirmed: true }],
    },
  ],
};

export const incidents: OverlayDef = {
  kind: 'incidents',
  name: 'Incident tracker',
  description:
    'Every incident you pick up: what it most likely was (off track, loss of control, hitting a wall, contact), the points, who you made contact with, and a replay button that jumps the iRacing replay to the moment (works while watching a replay).',
  topics: ['incidents', 'status'],
  size: [340, 230],
  settings: [
    { key: 'rows', label: 'Incidents shown', type: 'number', default: 6, min: 1, max: 30, group: 'Layout' },
    { key: 'showSummary', label: 'Count per type', type: 'bool', default: true, group: 'Layout' },
    { key: 'showWhen', label: 'Lap and session time', type: 'bool', default: true, group: 'Layout' },
    { key: 'showReplay', label: 'Replay buttons', type: 'bool', default: true, group: 'Replay', help: 'Only work while a replay is playing in iRacing' },
    { key: 'replayLead', when: (v) => v.showReplay !== false, label: 'Start the replay before the incident', type: 'number', default: 5, min: 0, max: 30, unit: 's', group: 'Replay' },
    { key: 'hideWhenNone', label: 'Hide until the first incident', type: 'bool', default: false, group: 'Behaviour' },
  ],
  commonDefaults: { autoHeight: true },
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('incidents');
    const header = h('div', 'header');
    const title = h('span', 'title', 'Incidents');
    const total = h('span', 'inc-total');
    header.append(title, total);
    const summary = h('div', 'inc-summary');
    const list = h('div', 'inc-list');
    const empty = h('div', 'inc-empty dim', 'No incidents — keep it clean');
    const hint = h('div', 'inc-hint dim', 'Replay buttons work while a replay is playing');
    root.append(header, summary, list, empty, hint);

    let live: Incidents | null = null;
    let editing = false;
    let interactive: boolean | null = null;

    /** Window takes clicks only while the replay buttons can do something. */
    const setInteractive = (on: boolean) => {
      if (on === interactive || preview) return;
      interactive = on;
      ctx.send({ op: 'setInteractive', id: ctx.id, value: on });
    };

    const row = (inc: Incident, canReplay: boolean) => {
      const r = h('div', `inc-row k-${inc.kind}`);
      const pts = h('span', `inc-pts p${Math.min(inc.points, 4)}`, `${inc.points}x`);
      const body = h('div', 'inc-body');
      const what = h('div', 'inc-what', inc.label);
      body.append(what);
      for (const c of inc.involved) {
        const where = Math.abs(c.dist) < 0.5 ? 'alongside' : `${Math.abs(c.dist).toFixed(0)} m ${c.dist > 0 ? 'ahead' : 'behind'}`;
        const who = h('div', 'inc-who', `#${c.number} ${c.name} · ${where}`);
        if (c.confirmed) who.append(h('span', 'inc-ok', ' ✓'));
        who.title = c.confirmed ? 'They picked up a 4x at the same time' : 'Nearest car when it happened';
        body.append(who);
      }
      if (s.showWhen) body.append(h('div', 'inc-when dim', `Lap ${inc.lap} · ${inc.sessionName || 'Session'} ${f.duration(inc.sessionTime)}`));
      r.append(pts, body);
      if (s.showReplay) {
        const b = h('button', 'inc-replay', '▶');
        b.disabled = !canReplay;
        b.title = canReplay ? 'Watch this in the replay' : 'Open a replay in iRacing to watch this';
        b.onclick = () => ctx.send({ op: 'replay', sessionNum: inc.sessionNum, sessionTime: Math.max(0, inc.sessionTime - (s.replayLead as number)) });
        r.append(b);
      }
      return r;
    };

    const render = () => {
      const d = live?.list.length || !(editing || preview) ? live : SAMPLE;
      if (!d) return;
      const canReplay = d.canReplay && d !== SAMPLE;
      setInteractive(!!s.showReplay && canReplay);
      text(total, d.limit ? `${d.total}x / ${d.limit}x` : `${d.total}x`);
      const near = d.limit ? d.total / d.limit : 0;
      total.classList.toggle('warn', near >= 0.6 && near < 0.85);
      total.classList.toggle('bad', near >= 0.85);
      const shown = [...d.list].reverse().slice(0, s.rows as number);
      list.replaceChildren(...shown.map((i) => row(i, canReplay)));
      show(empty, !d.list.length);
      show(hint, !!s.showReplay && !canReplay && d.list.length > 0 && d !== SAMPLE);
      show(summary, !!s.showSummary && d.list.length > 0);
      summary.replaceChildren(
        ...KINDS.map(([k, label]) => [label, d.list.filter((i) => i.kind === k)] as const)
          .filter(([, l]) => l.length)
          .map(([label, l]) => h('span', 'inc-chip', `${label} ×${l.length} (${l.reduce((a, i) => a + i.points, 0)}x)`)),
      );
      root.classList.toggle('auto-hidden', !!s.hideWhenNone && !d.list.length && !(editing || preview));
    };

    let lastKey = '';
    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        if (topic === 'incidents') live = data as Incidents;
        // Only rebuild the list when something changed (the topic repeats every tick).
        const key = JSON.stringify([live?.total, live?.list.length, live?.canReplay, live?.list.map((i) => i.involved.map((c) => c.confirmed)), editing]);
        if (key === lastKey) return;
        lastKey = key;
        render();
      },
    };
  },
};
