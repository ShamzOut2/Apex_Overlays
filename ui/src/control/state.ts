// Control panel state: a local copy of the config that is pushed to the
// core (debounced) whenever the user edits something.

import { Client } from '../lib/client';
import type { Config, Garage61, Monitor, LeagueImport, Player, Session, Standings, Status, TopicName, TrackMap, UpdateState, VrState } from '../lib/types';

export const state = {
  config: null as Config | null,
  status: null as Status | null,
  session: null as Session | null,
  standings: null as Standings | null,
  garage61: null as Garage61 | null,
  trackmap: null as TrackMap | null,
  player: null as Player | null,
  leagueImport: null as LeagueImport | null,
  update: null as UpdateState | null,
  monitor: null as Monitor | null,
  vr: null as VrState | null,
};

type Listener = (topic: TopicName) => void;
const listeners: Listener[] = [];
export const listen = (fn: Listener) => listeners.push(fn);

let timer = 0;
let lastSent = '';

/** Key-order independent JSON (the core stores free-form settings with sorted keys). */
export function canon(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(canon).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canon((v as Record<string, unknown>)[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v) ?? 'null';
}

export const client = new Client((t, d) => {
  if (t === 'config') {
    const json = canon(d);
    // Ignore the echo of our own save and anything arriving while an edit is pending.
    if (json === lastSent || timer) return;
    lastSent = json;
    state.config = d as Config;
  } else {
    (state as Record<string, unknown>)[t] = d;
  }
  listeners.forEach((l) => l(t));
});
client.subscribe('config', 'status', 'session', 'garage61', 'leagueImport', 'update', 'vr');

/** Live data only some pages show: received only while one of them is open. */
const PAGE_TOPICS: Record<string, TopicName[]> = { leagues: ['standings'], racecontrol: ['standings'], general: ['trackmap', 'player', 'monitor'] };

export function pageTopics(page: string) {
  const want = new Set(PAGE_TOPICS[page] ?? []);
  const all = new Set(Object.values(PAGE_TOPICS).flat());
  client.unsubscribe(...[...all].filter((t) => !want.has(t)));
  client.subscribe(...want);
}

export function save(immediate = false) {
  clearTimeout(timer);
  const send = () => {
    timer = 0;
    if (!state.config) return;
    lastSent = canon(state.config);
    client.send({ op: 'setConfig', config: state.config });
  };
  if (immediate) send();
  else timer = window.setTimeout(send, 250);
}

export function uid(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 7)}`;
}

export function download(name: string, data: unknown) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }));
  a.download = name;
  a.click();
  URL.revokeObjectURL(a.href);
}

export function upload(): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const i = document.createElement('input');
    i.type = 'file';
    i.accept = '.json,application/json';
    i.onchange = async () => {
      try {
        resolve(JSON.parse(await i.files![0].text()));
      } catch (e) {
        reject(e);
      }
    };
    i.click();
  });
}
