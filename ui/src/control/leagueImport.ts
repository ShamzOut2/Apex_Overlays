// League files exported by the commentator app ({"apex": "league", "format": 1, ...}):
// turned into an Apex Overlays league — classes, roster (number, class, nickname, team),
// points system and the championship so far, worked out from the rounds' results.

import type { DriverOverride, League, LeagueClass, PointsConfig, SeasonEntry } from '../lib/types';

/* The parts of the commentator app's export used here (snake_case, as written). */
interface CClass {
  id: string;
  name?: string;
  color?: string;
}
interface CDriver {
  cust_id?: number | string;
  name?: string;
  car_number?: string;
  nickname?: string;
  team?: string;
  class_id?: string;
}
interface CResult {
  driver?: string;
  name?: string;
  class_id?: string;
  position?: number | null;
  start_position?: number | null;
  laps?: number;
  laps_led?: number;
  fastest_lap?: number | null;
  incidents?: number;
  status?: string;
  adjust?: number;
}
interface CRound {
  name?: string;
  multiplier?: number;
  no_drop?: boolean;
  results?: CResult[];
}
interface CRules {
  points?: number[];
  scoring?: string;
  overall_points?: number[];
  class_points?: Record<string, number[]>;
  bonuses?: Record<string, number>;
  drop_rounds?: number;
  classified_pct?: number;
  dnf_scores?: boolean;
}
interface CPenalty {
  driver?: string;
  points?: number;
}
export interface CommentatorLeague {
  apex: 'league';
  format?: number;
  app_version?: string;
  league: {
    name?: string;
    league_id?: number | null;
    classes?: CClass[];
    drivers?: CDriver[];
    strip_class_prefix?: boolean;
    championship?: { rules?: CRules; rounds?: CRound[]; penalties?: CPenalty[] };
  };
}

/** Whether a parsed file is a commentator app league export. */
export function isCommentatorLeague(v: unknown): v is CommentatorLeague {
  const o = v as CommentatorLeague | null;
  return !!o && o.apex === 'league' && typeof o.league === 'object' && o.league !== null;
}

const num = (v: unknown, d = 0) => (typeof v === 'number' && Number.isFinite(v) ? v : d);
/** "cust:123" (results) or 123 (roster) → 123. */
const custId = (v: unknown) => {
  const n = typeof v === 'number' ? v : parseInt(String(v ?? '').replace(/^cust:/, ''), 10);
  return Number.isFinite(n) && n > 0 ? n : 0;
};

/** Points per driver over every round, as the commentator app scores them. */
function championship(rules: CRules, rounds: CRound[], penalties: CPenalty[], classOf: (r: CResult) => string): Map<number, { name: string; cls: string; points: number }> {
  const b = rules.bonuses ?? {};
  const byClass = rules.scoring !== 'overall';
  const tableFor = (cls: string) =>
    (byClass ? rules.class_points?.[cls] : rules.overall_points)?.length ? (byClass ? rules.class_points![cls] : rules.overall_points!) : (rules.points ?? []);
  // Each driver's score per round (for dropped rounds) and the total.
  const scores = new Map<number, { name: string; cls: string; rounds: { pts: number; droppable: boolean }[] }>();
  for (const round of rounds) {
    const results = (round.results ?? []).filter((r) => custId(r.driver));
    const groups = new Map<string, CResult[]>();
    for (const r of results) {
      const g = byClass ? classOf(r) : '';
      if (!groups.has(g)) groups.set(g, []);
      groups.get(g)!.push(r);
    }
    for (const [cls, group] of groups) {
      // Finishing order within the group; no position means not classified.
      const placed = group.filter((r) => typeof r.position === 'number').sort((a, c) => a.position! - c.position!);
      const winnerLaps = Math.max(0, ...group.map((r) => num(r.laps)));
      const fastest = Math.min(...group.map((r) => (num(r.fastest_lap) > 0 ? num(r.fastest_lap) : Infinity)));
      const mostLed = Math.max(0, ...group.map((r) => num(r.laps_led)));
      const gained = (r: CResult) => (typeof r.start_position === 'number' && typeof r.position === 'number' ? r.start_position - r.position : 0);
      const mostGained = Math.max(0, ...group.map(gained));
      const pole = group.reduce<CResult | null>((best, r) => (typeof r.start_position === 'number' && (!best || r.start_position < best.start_position!) ? r : best), null);
      const table = tableFor(cls);
      for (const r of group) {
        const id = custId(r.driver);
        const rank = placed.indexOf(r) + 1;
        const finished = r.status === 'finished';
        const classified = rank > 0 && (!rules.classified_pct || num(r.laps) >= (winnerLaps * rules.classified_pct) / 100);
        let pts = 0;
        if (classified && (finished || rules.dnf_scores !== false)) {
          pts = num(table[rank - 1]);
          if (rank === 1) pts += num(b.win);
          if (num(r.fastest_lap) > 0 && num(r.fastest_lap) === fastest) pts += num(b.fastest_lap);
          if (num(r.laps_led) > 0) pts += num(b.led_lap) + num(b.per_lap_led) * num(r.laps_led);
          if (mostLed > 0 && num(r.laps_led) === mostLed) pts += num(b.most_laps_led);
          if (mostGained > 0 && gained(r) === mostGained) pts += num(b.most_places_gained);
          if (finished) pts += num(b.finished);
          if (finished && num(r.incidents) === 0) pts += num(b.clean_race);
        }
        if (num(r.laps) > 0) pts += num(b.started);
        if (pole === r) pts += num(b.pole);
        pts = pts * num(round.multiplier, 1) + num(r.adjust);
        const s = scores.get(id) ?? { name: r.name ?? '', cls, rounds: [] };
        s.rounds.push({ pts, droppable: !round.no_drop });
        scores.set(id, s);
      }
    }
  }
  const out = new Map<number, { name: string; cls: string; points: number }>();
  for (const [id, s] of scores) {
    // The worst rounds are dropped (rounds marked "no drop" always count).
    const drop = new Set(
      s.rounds
        .map((r, i) => [r, i] as const)
        .filter(([r]) => r.droppable)
        .sort((x, y) => x[0].pts - y[0].pts)
        .slice(0, Math.max(0, num(rules.drop_rounds)))
        .map(([, i]) => i),
    );
    const points = s.rounds.reduce((t, r, i) => t + (drop.has(i) ? 0 : r.pts), 0);
    out.set(id, { name: s.name, cls: s.cls, points });
  }
  for (const p of penalties) {
    const s = out.get(custId(p.driver));
    if (s && typeof p.points === 'number') s.points -= Math.abs(p.points);
  }
  return out;
}

/** The commentator app's league as an Apex Overlays league (`id` from `newId`). */
export function fromCommentator(file: CommentatorLeague, base: League): League {
  const src = file.league;
  const classes: LeagueClass[] = (src.classes ?? [])
    .filter((c) => c && c.id)
    .map((c) => ({
      id: c.id,
      name: c.name || 'Class',
      color: /^#[0-9a-f]{6}$/i.test(c.color ?? '') ? c.color! : '#888888',
      rules: { carIds: [], carNames: [], carNumbers: [], customerIds: [], teamNames: [], driverNames: [], iracingClassIds: [] },
    }));
  const known = new Set(classes.map((c) => c.id));
  const roster = new Map<number, CDriver>();
  const driverOverrides: DriverOverride[] = [];
  for (const d of src.drivers ?? []) {
    const id = custId(d.cust_id);
    if (!id || roster.has(id)) continue;
    roster.set(id, d);
    driverOverrides.push({
      customerId: id,
      name: d.name ?? '',
      carNumber: d.car_number ?? '',
      classId: known.has(d.class_id ?? '') ? d.class_id! : '',
      displayName: d.nickname ?? '',
      teamName: d.team ?? '',
      country: '',
      hidden: false,
    });
  }
  const rules = src.championship?.rules ?? {};
  const b = rules.bonuses ?? {};
  const unsupported = Object.entries({
    'win bonus': b.win,
    'points per lap led': b.per_lap_led,
    'most places gained': b.most_places_gained,
    'clean race': b.clean_race,
    'started bonus': b.started,
  })
    .filter(([, v]) => num(v) !== 0)
    .map(([k, v]) => `${k} ${v}`);
  const points: PointsConfig = {
    ...base.points,
    table: (rules.points ?? []).map((p) => num(p)),
    defaultPoints: 0,
    basis: rules.scoring === 'overall' ? 'overall' : 'class',
    fastestLapBonus: num(b.fastest_lap),
    poleBonus: num(b.pole),
    lapLedBonus: num(b.led_lap),
    mostLapsLedBonus: num(b.most_laps_led),
    finishBonus: num(b.finished),
    systemName: 'From the commentator app',
    description: unsupported.length ? `Live points leave out: ${unsupported.join(', ')} (they are in the championship so far)` : '',
  };
  // Championship so far, ranked within each class (or overall).
  const classOf = (r: CResult) => (known.has(r.class_id ?? '') ? r.class_id! : (roster.get(custId(r.driver))?.class_id ?? ''));
  const table = championship(rules, src.championship?.rounds ?? [], src.championship?.penalties ?? [], classOf);
  const seasonPoints: SeasonEntry[] = [];
  const groups = new Map<string, [number, { name: string; cls: string; points: number }][]>();
  for (const e of table) {
    const g = points.basis === 'class' ? e[1].cls : '';
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g)!.push(e);
  }
  for (const list of groups.values()) {
    list.sort((a, c) => c[1].points - a[1].points);
    list.forEach(([id, s], i) => seasonPoints.push({ customerId: id, name: s.name, points: Math.round(s.points * 100) / 100, position: i + 1 }));
  }
  return {
    ...base,
    name: src.name || base.name,
    classes,
    driverOverrides,
    fallback: 'iracing',
    iracingLeagueId: num(src.league_id),
    points,
    seasonPoints,
    stripNameClass: !!src.strip_class_prefix,
  };
}
