// Drivers, teams and classes from a spreadsheet (CSV), e.g. a league's sign-up sheet. The
// same rules as the commentator app (apex/csvimport.py): comma, semicolon or tab separated,
// quotes allowed; the columns are guessed from their headings (and the class column, without
// a heading for it, from its values), and can be changed before importing.

import type { DriverOverride, League } from '../lib/types';

export const FIELDS = ['custId', 'name', 'first', 'last', 'number', 'team', 'teamColor', 'class', 'nickname'] as const;
export type Field = (typeof FIELDS)[number];
export type Mapping = Partial<Record<Field, number>>;

export const FIELD_LABELS: Record<Field, string> = {
  custId: 'iRacing (customer) ID',
  name: 'Full name',
  first: 'First name',
  last: 'Last name',
  number: 'Car number',
  team: 'Team',
  teamColor: 'Team colour',
  class: 'Class',
  nickname: 'Nickname',
};

const HEADINGS: Record<Field, string[]> = {
  custId: ['customerid', 'custid', 'iracingid', 'iracingcustomerid', 'memberid', 'iracingmemberid', 'iracing', 'id'],
  name: ['fullname', 'name', 'drivername', 'driver'],
  first: ['firstname', 'first', 'forename', 'givenname'],
  last: ['lastname', 'last', 'surname', 'familyname'],
  number: ['carnumber', 'carno', 'carnum', 'number', 'no', 'racenumber', 'num', 'car'],
  team: ['teamname', 'team'],
  teamColor: ['teamcolor', 'teamcolour'],
  class: ['class', 'classname', 'category', 'division', 'carclass', 'tier', 'grade', 'licence', 'license'],
  nickname: ['nickname', 'nick', 'displayname'],
};

/** Classes known by name get their usual colour. */
const CLASS_COLORS: Record<string, string> = {
  gold: '#d4af37', silver: '#b8bec6', bronze: '#cd7f32', platinum: '#e5e4e2', pro: '#e8463c', proam: '#f2994a', am: '#27ae60', rookie: '#2f80ed',
};
const PALETTE = ['#ff4d6d', '#3fa7ff', '#ffd23f', '#3ddc84', '#b26bff', '#ff8c42', '#2ee6d6', '#f2f4f8'];

const key = (t: string) => t.toLowerCase().replace(/[^a-z0-9]/g, '');

/** CSV text → heading row and rows (each as long as the headings). */
export function read(text: string): { headers: string[]; rows: string[][] } {
  text = text.replace(/^﻿/, '');
  if (!text.trim()) throw new Error('The file is empty.');
  // The separator: whichever of , ; and tab the first line has most of.
  const first = text.split(/\r?\n/, 1)[0];
  const sep = [',', ';', '\t'].map((c) => [c, first.split(c).length] as const).sort((a, b) => b[1] - a[1])[0][0];
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') {
        cell += '"';
        i++;
      } else if (c === '"') quoted = false;
      else cell += c;
    } else if (c === '"' && !cell) quoted = true;
    else if (c === sep) {
      row.push(cell);
      cell = '';
    } else if (c === '\n' || c === '\r') {
      if (c === '\r' && text[i + 1] === '\n') i++;
      row.push(cell);
      rows.push(row);
      row = [];
      cell = '';
    } else cell += c;
  }
  if (cell || row.length) {
    row.push(cell);
    rows.push(row);
  }
  const filled = rows.filter((r) => r.some((c) => c.trim()));
  if (filled.length < 2) throw new Error('No rows under the heading row.');
  const headers = filled[0].map((h) => h.trim());
  const width = headers.length;
  return { headers, rows: filled.slice(1).map((r) => [...r, ...Array(width).fill('')].slice(0, width)) };
}

/** Which column is which, as far as the headings (or the values, for the class) tell. */
export function guess(headers: string[], rows: string[][]): Mapping {
  const keys = headers.map(key);
  const out: Mapping = {};
  const taken = () => new Set(Object.values(out));
  // An exact heading first, in the order of preference; then a heading containing one.
  for (const f of FIELDS) {
    for (const want of HEADINGS[f]) {
      const i = keys.findIndex((k, j) => k === want && !taken().has(j));
      if (i >= 0) {
        out[f] = i;
        break;
      }
    }
  }
  for (const f of FIELDS) {
    if (out[f] != null) continue;
    for (const want of HEADINGS[f]) {
      if (want.length < 4) continue;
      const i = keys.findIndex((k, j) => k.includes(want) && !taken().has(j));
      if (i >= 0) {
        out[f] = i;
        break;
      }
    }
  }
  if (out.class == null) {
    // A column that looks like classes: a few different words, repeated, in most rows.
    let best: [number, number, number] | null = null;
    headers.forEach((_, i) => {
      if (taken().has(i)) return;
      const values = rows.map((r) => r[i].trim()).filter(Boolean);
      if (values.length < Math.max(2, Math.floor(rows.length / 2))) return;
      const distinct = new Set(values.map((v) => v.toLowerCase()));
      if (distinct.size < 2 || distinct.size > 10 || distinct.size >= values.length) return;
      if ([...distinct].some((v) => /^\d+$/.test(v.replace('.', '')))) return;
      if (values.reduce((a, v) => a + v.length, 0) / values.length > 20) return;
      const score: [number, number, number] = [i, distinct.size / values.length, -values.length];
      if (!best || score[1] < best[1] || (score[1] === best[1] && score[2] < best[2])) best = score;
    });
    if (best) out.class = (best as [number, number, number])[0];
  }
  return out;
}

export interface Summary {
  added: number;
  updated: number;
  teams: number;
  classes: number;
  skipped: number;
}

/**
 * Puts the sheet's drivers, teams and classes into the league: a driver already on the roster
 * (same iRacing ID, else the same name) gets the sheet's details, anyone else is added; teams
 * and classes are added when the league does not have them yet.
 */
export function apply(l: League, rows: string[][], mapping: Mapping, role: 'main' | 'secondary' = 'main'): Summary {
  const sum: Summary = { added: 0, updated: 0, teams: 0, classes: 0, skipped: 0 };
  l.teams ??= [];
  const cell = (r: string[], f: Field) => {
    const i = mapping[f];
    return i != null && i >= 0 && i < r.length ? r[i].trim() : '';
  };
  // The league's class of that name, or a new one.
  const classOf = (name: string) => {
    let c = l.classes.find((k) => key(k.name) === key(name));
    if (!c) {
      const label = name === name.toUpperCase() && name.length > 3 ? name[0] + name.slice(1).toLowerCase() : name;
      c = {
        id: `c-${Math.random().toString(36).slice(2, 7)}`,
        name: label,
        color: CLASS_COLORS[key(name)] ?? PALETTE[l.classes.length % PALETTE.length],
        rules: { carIds: [], carNames: [], carNumbers: [], customerIds: [], teamNames: [], driverNames: [], iracingClassIds: [] },
      };
      l.classes.push(c);
      sum.classes++;
    }
    return c;
  };
  for (const r of rows) {
    const idText = cell(r, 'custId');
    const custId = /^\d+$/.test(idText) ? Number(idText) : 0;
    const name = cell(r, 'name') || [cell(r, 'first'), cell(r, 'last')].filter(Boolean).join(' ');
    if (!name && !custId) {
      sum.skipped++;
      continue;
    }
    let d: DriverOverride | undefined = (custId && l.driverOverrides.find((o) => o.customerId === custId)) || (name ? l.driverOverrides.find((o) => o.name.trim().toLowerCase() === name.toLowerCase()) : undefined);
    if (!d) {
      d = { customerId: custId, name, carNumber: '', classId: '', displayName: '', teamName: '', country: '', hidden: false };
      l.driverOverrides.push(d);
      sum.added++;
    } else sum.updated++;
    if (name) d.name = name;
    if (custId) d.customerId = custId;
    const number = cell(r, 'number').replace(/^#/, '');
    if (number) d.carNumber = number;
    const nick = cell(r, 'nickname');
    if (nick) d.displayName = nick;
    const team = cell(r, 'team');
    if (team) {
      let t = l.teams.find((x) => x.name.trim().toLowerCase() === team.toLowerCase());
      if (!t) {
        t = { name: team, short: '', color: '', logo: '', logoSmall: '', shared: [] };
        l.teams.push(t);
        sum.teams++;
      }
      const color = cell(r, 'teamColor');
      if (/^#?[0-9a-f]{6}$/i.test(color) && !t.color) t.color = `#${color.replace(/^#/, '').toLowerCase()}`;
      d.teamName = t.name;
    }
    const klass = cell(r, 'class');
    if (klass) {
      const c = classOf(klass);
      // The sheet's class column is each driver's main class, or their 2nd class.
      if (role === 'secondary') d.class2Id = c.id;
      else d.classId = c.id;
    }
  }
  return sum;
}
