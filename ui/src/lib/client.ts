// WebSocket client: subscribes to topics, reconnects automatically and
// batches incoming data so consumers render at most once per animation frame.

import type { TopicMap, TopicName } from './types';

type Handler = (topic: TopicName, data: any) => void;

/**
 * Topics whose rows carry only a car index (plus isPlayer / isFocus) and live data: each car's
 * fixed details (name, team, car, class, licence...) come on the `drivers` topic, sent only when
 * they change, and are filled into the rows here, so the frequent row messages stay small.
 */
const WITH_DRIVERS = new Set(['relative', 'standings', 'safety', 'battle']);

/** Details for a car the `drivers` topic has not listed (yet): blank rather than missing. */
const UNKNOWN_DRIVER = {
  number: '',
  name: '',
  abbrev: '',
  team: '',
  car: '',
  classId: '',
  className: '',
  classColor: '#888888',
  class2Id: '',
  class2Name: '',
  class2Color: '',
  userId: 0,
  carId: 0,
  irating: 0,
  license: '',
  licenseColor: '#888888',
  club: '',
  flair: '',
  country: '',
  make: '',
  teamColor: '',
  teamShort: '',
  teamLogo: '',
  teamLogoSmall: '',
};

export class Client {
  private ws: WebSocket | null = null;
  private topics = new Set<string>();
  /** Newest raw message per topic, waiting for the next frame. */
  private pending = new Map<TopicName, string>();
  private frame = 0;
  /** Topics delivered at most this often (ms between deliveries), and when each last was. */
  private gap = new Map<string, number>();
  private lastAt = new Map<string, number>();
  private timer = 0;
  /** Error reports made before the connection was up. */
  private outbox: string[] = [];
  readonly latest: Partial<TopicMap> = {};
  connected = false;
  /** Car index → fixed details, from the `drivers` topic. */
  private drivers: Map<number, object> | null = null;
  /** Rows that came before the drivers' details: delivered as soon as those arrive. */
  private waiting = new Map<TopicName, unknown>();

  constructor(private onData: Handler, private onConnection?: (up: boolean) => void) {
    this.open();
  }

  private url(): string {
    const q = new URLSearchParams(location.search);
    const host = q.get('server') ?? location.host;
    return `${location.protocol === 'https:' ? 'wss' : 'ws'}://${host}/ws`;
  }

  private open() {
    const ws = new WebSocket(this.url());
    this.ws = ws;
    ws.onopen = () => {
      this.connected = true;
      // Lines written while the connection was down go first, so the log keeps their order.
      for (const m of this.outbox.splice(0)) ws.send(m);
      this.onConnection?.(true);
      if (this.topics.size) this.send({ op: 'sub', topics: [...this.topics] });
    };
    ws.onmessage = (ev) => {
      // Messages are `{"t":"<topic>","d":<data>}`. Only the newest of each topic is used per
      // frame, so the data is parsed when it is delivered, not on arrival: a topic arriving
      // faster than the screen refreshes (or while the window is hidden) costs next to nothing.
      const raw = ev.data as string;
      const end = raw.indexOf('"', 6);
      if (!raw.startsWith('{"t":"') || end < 0) return;
      this.pending.set(raw.slice(6, end) as TopicName, raw);
      this.schedule();
    };
    ws.onclose = () => {
      this.connected = false;
      this.onConnection?.(false);
      setTimeout(() => this.open(), 1000);
    };
  }

  /** Milliseconds until `t` may be delivered again (0: now). */
  private wait(t: string, now: number): number {
    const gap = this.gap.get(t);
    return gap ? Math.max(0, (this.lastAt.get(t) ?? -Infinity) + gap - now) : 0;
  }

  /** Next frame for topics that are due; a timer (no frame) for rate-limited ones that are not yet. */
  private schedule() {
    if (this.frame || !this.pending.size) return;
    const now = performance.now();
    let soonest = Infinity;
    for (const t of this.pending.keys()) soonest = Math.min(soonest, this.wait(t, now));
    if (soonest <= 1) {
      clearTimeout(this.timer);
      this.timer = 0;
      this.frame = requestAnimationFrame(() => this.flush());
    } else if (!this.timer) {
      this.timer = window.setTimeout(() => {
        this.timer = 0;
        this.schedule();
      }, soonest);
    }
  }

  /** Deliver `topic` at most `hz` times a second (the newest data each time); 0 = as it comes. */
  limit(topic: string, hz: number) {
    if (hz > 0) this.gap.set(topic, 1000 / hz - 2);
    else this.gap.delete(topic);
  }

  private flush() {
    this.frame = 0;
    const now = performance.now();
    // The drivers' details first, so rows in the same batch can use them.
    const batch = [...this.pending]
      .filter(([t]) => this.wait(t, now) <= 1)
      .sort((a, b) => Number(b[0] === ('drivers' as TopicName)) - Number(a[0] === ('drivers' as TopicName)));
    for (const [t] of batch) {
      this.pending.delete(t);
      if (this.gap.has(t)) this.lastAt.set(t, now);
    }
    // Rate-limited topics not due yet wait for their turn.
    this.schedule();
    for (const [t, raw] of batch) {
      const d = (JSON.parse(raw) as { d: unknown }).d;
      if ((t as string) === 'drivers') {
        this.drivers = new Map(((d as { cars?: { idx: number }[] })?.cars ?? []).map((c) => [c.idx, c]));
        const held = [...this.waiting];
        this.waiting.clear();
        for (const [wt, wd] of held) this.deliver(wt, wd);
        continue;
      }
      if (WITH_DRIVERS.has(t)) {
        if (!this.drivers) {
          this.waiting.set(t, d);
          continue;
        }
      }
      this.deliver(t, d);
    }
  }

  private deliver(t: TopicName, d: unknown) {
    if (WITH_DRIVERS.has(t)) this.fill(d);
    (this.latest as Record<string, unknown>)[t] = d;
    this.onData(t, d);
  }

  /** Puts each car's fixed details into the rows (objects with a car `idx` and `isFocus`). */
  private fill(v: unknown) {
    if (Array.isArray(v)) {
      for (const x of v) this.fill(x);
      return;
    }
    if (!v || typeof v !== 'object') return;
    const o = v as Record<string, unknown>;
    if (typeof o.idx === 'number' && 'isFocus' in o) Object.assign(o, UNKNOWN_DRIVER, this.drivers?.get(o.idx));
    for (const k in o) {
      const x = o[k];
      if (x && typeof x === 'object') this.fill(x);
    }
  }

  /** Stop receiving topics (e.g. data a settings page no longer shows). */
  unsubscribe(...topics: string[]) {
    const gone = topics.filter((t) => this.topics.delete(t));
    // The drivers' details are only needed while a topic with rows is.
    if (gone.length && this.topics.has('drivers') && ![...this.topics].some((t) => WITH_DRIVERS.has(t)) && this.topics.delete('drivers')) {
      gone.push('drivers');
      this.drivers = null;
    }
    gone.forEach((t) => this.waiting.delete(t as TopicName));
    gone.forEach((t) => this.pending.delete(t as TopicName));
    if (gone.length && this.ws?.readyState === WebSocket.OPEN) this.send({ op: 'unsub', topics: gone });
  }

  subscribe(...topics: string[]) {
    // Rows need the drivers' details: subscribe to those first, so they arrive first.
    if (topics.some((t) => WITH_DRIVERS.has(t))) topics = ['drivers', ...topics];
    const fresh = topics.filter((t) => !this.topics.has(t));
    fresh.forEach((t) => this.topics.add(t));
    if (fresh.length && this.ws?.readyState === WebSocket.OPEN) this.send({ op: 'sub', topics: fresh });
  }

  /** Writes an error or warning to the app's log file (apex.log). */
  report(level: 'error' | 'warn' | 'info', source: string, message: string) {
    const m = JSON.stringify({ op: 'log', level, source, message });
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(m);
    else if (this.outbox.length < 20) this.outbox.push(m);
  }

  send(msg: object) {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg));
  }
}
