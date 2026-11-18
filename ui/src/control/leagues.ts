// League builder: custom classes (by car, number, driver, team, ...), points
// systems and driver overrides. Leagues can be imported from iRacing by league
// ID or created from the session you are in. Applied live to every overlay.

import { h } from '../lib/dom';
import type { ClassRules, League, LeagueClass, PointsConfig } from '../lib/types';
import { flagEmoji } from '../overlays/common';
import { showFlagCodes } from './flagCodes';
import { fromCommentator, isCommentatorLeague } from './leagueImport';
import type { Page } from './pages';
import { download, save, state, uid, upload } from './state';
import { arrowNav, teamsFromFile, teamsOf } from './teams';

let selected = '';

/** The league being edited (shared with the Team builder). */
export const selectedLeague = () => selected;
export function selectLeague(id: string) {
  selected = id;
}

const RULES: [keyof ClassRules, string, 'num' | 'text', string][] = [
  ['carNumbers', 'Car numbers', 'text', '1, 7, 42'],
  ['customerIds', 'iRacing customer IDs', 'num', '123456, 234567'],
  ['driverNames', 'Driver names', 'text', 'Max*, *Smith'],
  ['teamNames', 'Team names', 'text', '*Racing Academy*'],
  ['carNames', 'Car names', 'text', 'Porsche 911*, *GT3*'],
  ['carIds', 'iRacing car IDs', 'num', '169, 173'],
  ['iracingClassIds', 'iRacing class IDs', 'num', '4083'],
];

const PALETTE = ['#ff4d6d', '#3fa7ff', '#ffd23f', '#3ddc84', '#b26bff', '#ff8c42', '#2ee6d6', '#f2f4f8'];

const POINT_PRESETS: [string, number[]][] = [
  ['F1 (top 10)', [25, 18, 15, 12, 10, 8, 6, 4, 2, 1]],
  ['F1 sprint (top 8)', [8, 7, 6, 5, 4, 3, 2, 1]],
  ['MotoGP (top 15)', [25, 20, 16, 13, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]],
  ['IndyCar (top 25)', [50, 40, 35, 32, 30, 28, 26, 24, 22, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5]],
  ['Linear 30 → 1', Array.from({ length: 30 }, (_, i) => 30 - i)],
];

const emptyRules = (): ClassRules => ({ carIds: [], carNames: [], carNumbers: [], customerIds: [], teamNames: [], driverNames: [], iracingClassIds: [] });
const emptyPoints = (): PointsConfig => ({
  table: [],
  defaultPoints: 0,
  basis: 'class',
  fastestLapBonus: 0,
  poleBonus: 0,
  lapLedBonus: 0,
  mostLapsLedBonus: 0,
  finishBonus: 0,
  systemName: '',
  description: '',
});
const newLeague = (name: string): League => ({
  id: uid('league'),
  name,
  classes: [],
  driverOverrides: [],
  fallback: 'iracing',
  iracingLeagueId: 0,
  seasonId: 0,
  seasonName: '',
  points: emptyPoints(),
  seasonPoints: [],
  importedAt: '',
  stripNameClass: false,
  teams: [],
});

function btn(label: string, fn: () => void, cls = '') {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    fn();
  };
  return b;
}

function labeled(label: string, el: HTMLElement, help?: string) {
  const r = h('label', 'field');
  const l = h('span', 'field-label', label);
  if (help) l.append(h('small', 'help', help));
  r.append(l, el);
  return r;
}

function listInput(value: (string | number)[], kind: 'num' | 'text', placeholder: string, onChange: (v: (string | number)[]) => void) {
  const i = h('input');
  i.value = value.join(', ');
  i.placeholder = placeholder;
  i.oninput = () => {
    const parts = i.value
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    onChange(kind === 'num' ? parts.map(Number).filter(Number.isFinite) : parts);
  };
  return i;
}

function classEditor(l: League, c: LeagueClass, i: number, rerender: () => void) {
  const el = h('div', 'league-class');
  el.style.setProperty('--class', c.color);
  const head = h('div', 'lc-head');
  const name = h('input', 'title-input');
  name.value = c.name;
  name.placeholder = 'Class name';
  name.oninput = () => {
    c.name = name.value;
    save();
  };
  const color = h('input');
  color.type = 'color';
  color.value = c.color;
  color.oninput = () => {
    c.color = color.value;
    el.style.setProperty('--class', c.color);
    save();
  };
  const move = (d: number) => () => {
    const [x] = l.classes.splice(i, 1);
    l.classes.splice(i + d, 0, x);
    save();
    rerender();
  };
  const up = btn('▲', move(-1), 'mini');
  const down = btn('▼', move(1), 'mini');
  up.disabled = i === 0;
  down.disabled = i === l.classes.length - 1;
  // Main or secondary: switching moves the drivers picked for it to the other roster column.
  const kind = h('select') as HTMLSelectElement;
  kind.title = 'Main: the class the overlays show. Secondary: a class across the main ones (e.g. a Rookie Cup), picked in the roster.';
  for (const [v, t] of [
    ['main', 'Main class'],
    ['secondary', 'Secondary class'],
  ]) {
    const o = h('option', '', t);
    o.value = v;
    kind.append(o);
  }
  const secondary = c.role === 'secondary';
  kind.value = secondary ? 'secondary' : 'main';
  kind.onchange = () => {
    const sec = kind.value === 'secondary';
    c.role = sec ? 'secondary' : 'main';
    for (const o of l.driverOverrides) {
      if (sec && o.classId === c.id) {
        o.classId = '';
        o.class2Id = c.id;
      } else if (!sec && o.class2Id === c.id) {
        o.class2Id = '';
        if (!o.classId) o.classId = c.id;
      }
    }
    save();
    rerender();
  };
  head.append(
    color,
    name,
    kind,
    up,
    down,
    btn(
      'Remove',
      () => {
        l.classes.splice(i, 1);
        save();
        rerender();
      },
      'danger mini',
    ),
  );
  const rules = h('div', 'rules');
  for (const [key, label, kind, ph] of RULES) {
    rules.append(
      labeled(
        label,
        listInput(c.rules[key] as (string | number)[], kind, ph, (v) => {
          (c.rules[key] as (string | number)[]) = v;
          save();
        }),
      ),
    );
  }
  el.append(head, secondary ? h('p', 'dim small', 'Picked per driver in the roster. Drivers keep their main class; overlays can colour numbers or stripes by this class and show it in a 2ND column.') : rules);
  return el;
}

function pointsCard(l: League, rerender: () => void) {
  l.points ??= emptyPoints();
  const p = l.points;
  const c = h('section', 'card');
  c.append(h('h2', '', 'Points system'));
  if (p.systemName || p.description) c.append(h('p', 'dim', `Imported: ${p.systemName}${p.description ? ` — ${p.description}` : ''}`));
  const basis = h('select');
  for (const [v, t] of [
    ['class', 'Class position (multi-class leagues)'],
    ['overall', 'Overall position'],
  ]) {
    const o = h('option', '', t);
    o.value = v;
    basis.append(o);
  }
  basis.value = p.basis || 'class';
  basis.onchange = () => {
    p.basis = basis.value as PointsConfig['basis'];
    save();
  };
  const num = (value: number, set: (v: number) => void) => {
    const i = h('input');
    i.type = 'number';
    i.step = '0.5';
    i.value = String(value);
    i.oninput = () => {
      set(parseFloat(i.value) || 0);
      save();
    };
    return i;
  };
  const scope = 'in the class (or overall, matching the setting above)';
  c.append(
    labeled('Points are scored by', basis, 'Leagues differ: some score by class finishing position, some overall'),
    h('h3', '', 'Bonus points (optional)'),
    labeled('Fastest lap', num(p.fastestLapBonus ?? 0, (v) => (p.fastestLapBonus = v)), `Fastest lap ${scope}`),
    labeled('Pole position', num(p.poleBonus ?? 0, (v) => (p.poleBonus = v)), `Qualifying P1 ${scope}`),
    labeled('Led a lap', num(p.lapLedBonus ?? 0, (v) => (p.lapLedBonus = v)), `Leading ${scope} across the line at least once`),
    labeled('Most laps led', num(p.mostLapsLedBonus ?? 0, (v) => (p.mostLapsLedBonus = v)), `Most laps led ${scope}`),
    labeled('Every finisher', num(p.finishBonus ?? 0, (v) => (p.finishBonus = v)), 'Participation points added for every classified driver'),
    labeled('Positions beyond the table', num(p.defaultPoints, (v) => (p.defaultPoints = v)), 'Points for positions after the last one listed below'),
    h('h3', '', 'Points per position'),
  );
  const table = h('div', 'points-table');
  p.table.forEach((v, i) => {
    const cell = h('label', '', `P${i + 1}`);
    cell.append(num(v, (x) => (p.table[i] = x)));
    table.append(cell);
  });
  const presets = h('select');
  presets.append(h('option', '', 'Load a preset…'));
  for (const [name] of POINT_PRESETS) presets.append(h('option', '', name));
  presets.onchange = () => {
    const preset = POINT_PRESETS.find(([n]) => n === presets.value);
    if (preset) {
      p.table = [...preset[1]];
      save();
      rerender();
    }
  };
  const paste = h('input');
  paste.placeholder = 'or paste: 25-18-15-12-10…';
  paste.onchange = () => {
    const nums = paste.value
      .split(/[^0-9.]+/)
      .filter(Boolean)
      .map(Number)
      .filter(Number.isFinite);
    if (nums.length) {
      p.table = nums;
      save();
      rerender();
    }
  };
  const bar = h('div', 'toolbar');
  bar.append(
    btn('Add position', () => {
      p.table.push(Math.max(0, (p.table[p.table.length - 1] ?? 1) - 1));
      save();
      rerender();
    }),
    btn('Remove last', () => {
      p.table.pop();
      save();
      rerender();
    }),
    presets,
    paste,
  );
  c.append(table, bar, h('p', 'dim small', 'Add the “League points” and “Championship total” columns to the Relative or Standings overlay to see live points.'));
  if (l.seasonPoints?.length) {
    c.append(h('h3', '', `Championship standings ${l.seasonName ? `(${l.seasonName})` : ''}`));
    // Scored per class: a table per class (by each driver's class in the roster).
    const classOf = new Map(l.driverOverrides.map((d) => [d.customerId, d.classId]));
    const mains = l.classes.filter((k) => k.role !== 'secondary');
    const groups = p.basis === 'class' && mains.length ? [...mains.map((k) => ({ name: k.name, id: k.id })), { name: 'Other', id: '' }] : [{ name: '', id: null }];
    for (const g of groups) {
      const rows = l.seasonPoints.filter((e) => g.id === null || (classOf.get(e.customerId) ?? '') === g.id || (g.id === '' && !mains.some((k) => k.id === classOf.get(e.customerId))));
      if (!rows.length) continue;
      const top = rows.sort((a, b) => b.points - a.points).slice(0, 10);
      const t = h('table', 'drivers');
      t.append(...top.map((e, i) => {
        const tr = h('tr');
        tr.append(h('td', '', String(e.position || i + 1)), h('td', '', e.name), h('td', '', String(e.points)));
        return tr;
      }));
      if (g.name) c.append(h('p', 'small', g.name));
      c.append(t);
    }
  }
  return c;
}

/** Drivers of the session in progress (name, number, customer ID, flag). */
function sessionRows() {
  return (state.standings?.classes.flatMap((c) => c.rows) ?? []).sort((a, b) => a.position - b.position);
}

/** "(P) Max Power" → "Max Power": the class tags some leagues put in names (as `strip_name_class` in the app). */
function stripNameClass(name: string): string {
  let out = '';
  let depth = 0;
  for (const ch of name) {
    if (ch === '(') depth++;
    else if (ch === ')' && depth > 0) depth--;
    else if (depth === 0) out += ch;
  }
  out = out.split(/\s+/).filter(Boolean).join(' ');
  return out || name.trim();
}

/**
 * With "Remove Driver Class in Name" on: the class tag comes out of each name, and the
 * display name (nickname) is filled with the full name, so overlays show it cleanly. A
 * nickname of the driver's own is kept, just without a tag. Returns whether anything changed.
 */
function applyStripNames(l: League): boolean {
  let changed = false;
  for (const o of l.driverOverrides) {
    const name = stripNameClass(o.name ?? '');
    const nick = o.displayName ? stripNameClass(o.displayName) : name;
    if (name !== o.name || nick !== o.displayName) {
      o.name = name;
      o.displayName = nick;
      changed = true;
    }
  }
  return changed;
}

/** League roster: drivers pulled automatically from the session (names, numbers), editable by hand. */
function rosterCard(l: League, rerender: () => void) {
  const c = h('section', 'card');
  c.append(
    h('h2', '', 'Drivers'),
    h(
      'p',
      'dim',
      'Pull in every driver in this session with one click, then pick their class and, if you like, rename them, set a team, flag or number, or hide spectators.',
    ),
  );
  const bar = h('div', 'toolbar');
  bar.append(
    btn(
      'Pull drivers from current session',
      () => {
        const rows = sessionRows();
        if (!rows.length) return alert('Join a session (or run the simulator) first.');
        let added = 0;
        for (const r of rows) {
          const existing = l.driverOverrides.find((o) => o.customerId === r.userId);
          if (existing) {
            existing.name ||= r.name;
            existing.carNumber ||= r.number;
            continue;
          }
          const cls = l.classes.find((k) => k.rules.customerIds.includes(r.userId));
          l.driverOverrides.push({ customerId: r.userId, name: r.name, carNumber: r.number, classId: cls?.id ?? '', displayName: '', teamName: '', country: '', hidden: false });
          added++;
        }
        save();
        rerender();
        if (!added) alert('Every driver in this session is already on the roster.');
      },
      'primary',
    ),
    btn('Add driver manually', () => {
      l.driverOverrides.push({ customerId: 0, name: '', carNumber: '', classId: '', displayName: '', teamName: '', country: '', hidden: false });
      save();
      rerender();
    }),
    btn('Show Flag Abbreviations', showFlagCodes),
  );
  // Drivers added since the box was ticked get the same treatment.
  if (l.stripNameClass && applyStripNames(l)) save();
  const strip = h('label', 'inline');
  const stripBox = h('input');
  stripBox.type = 'checkbox';
  stripBox.checked = !!l.stripNameClass;
  stripBox.onchange = () => {
    l.stripNameClass = stripBox.checked;
    if (l.stripNameClass) applyStripNames(l);
    save();
    rerender();
  };
  strip.title = 'For leagues that put the class in front of names, like “(P) Max Power”: the overlays show just “Max Power”.';
  strip.append(stripBox, ' Remove Driver Class in Name');
  bar.append(strip);
  if (l.driverOverrides.length) {
    bar.append(
      btn(
        'Clear roster',
        () => {
          if (!confirm('Remove every driver from this league roster?')) return;
          l.driverOverrides = [];
          save();
          rerender();
        },
        'danger',
      ),
    );
  }
  const table = h('table', 'drivers roster');
  const head = h('tr');
  const mains = l.classes.filter((k) => k.role !== 'secondary');
  const seconds = l.classes.filter((k) => k.role === 'secondary');
  for (const t of ['Car #', 'Flag', 'iRacing name', 'Customer ID', 'Class', ...(seconds.length ? ['2nd class'] : []), 'Display name', 'Team', 'Hide', '']) head.append(h('th', '', t));
  table.append(head);
  const live = new Map(sessionRows().map((r) => [r.userId, r]));
  l.driverOverrides.forEach((o, i) => {
    o.name ??= '';
    o.carNumber ??= '';
    o.classId ??= '';
    o.country ??= '';
    const tr = h('tr');
    const cell = (el: HTMLElement | string) => {
      const td = h('td');
      td.append(el);
      tr.append(td);
    };
    const input = (value: string, set: (v: string) => void, placeholder = '', width = '') => {
      const el = h('input');
      el.value = value;
      el.placeholder = placeholder;
      if (width) el.style.width = width;
      el.oninput = () => {
        set(el.value);
        save();
      };
      return el;
    };
    const r = live.get(o.customerId);
    cell(input(o.carNumber, (v) => (o.carNumber = v.trim()), r?.number ?? '#', '56px'));
    cell(input(o.country, (v) => (o.country = v.trim().toUpperCase()), r?.country ? `${flagEmoji(r.country)} ${r.country}` : 'GB', '64px'));
    cell(input(o.name, (v) => (o.name = v), 'Name'));
    cell(input(o.customerId ? String(o.customerId) : '', (v) => (o.customerId = parseInt(v, 10) || 0), 'ID', '90px'));
    const cls = h('select');
    const none = h('option', '', 'By class rules');
    none.value = '';
    cls.append(none);
    for (const k of mains) {
      const opt = h('option', '', k.name || '(unnamed)');
      opt.value = k.id;
      cls.append(opt);
    }
    cls.value = mains.some((k) => k.id === o.classId) ? o.classId : '';
    cls.onchange = () => {
      o.classId = cls.value;
      save();
    };
    cell(cls);
    if (seconds.length) {
      const cls2 = h('select') as HTMLSelectElement;
      const none2 = h('option', '', '—');
      none2.value = '';
      cls2.append(none2);
      for (const k of seconds) {
        const opt = h('option', '', k.name || '(unnamed)');
        opt.value = k.id;
        cls2.append(opt);
      }
      cls2.value = seconds.some((k) => k.id === o.class2Id) ? o.class2Id! : '';
      cls2.onchange = () => {
        o.class2Id = cls2.value;
        save();
      };
      cell(cls2);
    }
    cell(input(o.displayName, (v) => (o.displayName = v), 'Nickname'));
    const team = input(o.teamName, (v) => (o.teamName = v), 'Team');
    team.setAttribute('list', 'league-teams');
    cell(team);
    const hide = h('input');
    hide.type = 'checkbox';
    hide.checked = o.hidden;
    hide.onchange = () => {
      o.hidden = hide.checked;
      save();
    };
    cell(hide);
    cell(
      btn(
        '✕',
        () => {
          l.driverOverrides.splice(i, 1);
          save();
          rerender();
        },
        'mini',
      ),
    );
    table.append(tr);
  });
  // The Team column suggests the league's teams (Team builder); a new name makes a new team.
  const teams = h('datalist');
  teams.id = 'league-teams';
  for (const t of teamsOf(l)) teams.append(h('option', '', t.name));
  arrowNav(table);
  c.append(bar, l.driverOverrides.length ? table : h('p', 'dim', 'No drivers on the roster yet.'), teams);
  return c;
}

/** A league built from the session you are in: its iRacing classes and league ID. */
function fromSession(): League | null {
  const st = state.standings;
  if (!st?.classes.length) return null;
  const l = newLeague(state.session?.leagueId ? `League ${state.session.leagueId}` : `${state.session?.trackName ?? 'Session'} league`);
  l.iracingLeagueId = state.session?.leagueId ?? 0;
  st.classes.forEach((c, i) => {
    const irClass = /^ir(\d+)$/.exec(c.id);
    const carIds = [...new Set(c.rows.map((r) => r.carId))];
    l.classes.push({
      id: uid('c'),
      name: c.name,
      color: c.color && c.color !== '#ffffff' ? c.color : PALETTE[i % PALETTE.length],
      rules: { ...emptyRules(), iracingClassIds: irClass ? [Number(irClass[1])] : [], carIds: irClass ? [] : carIds },
    });
  });
  // Roster: every driver with their name, number and class.
  st.classes.forEach((c, i) => {
    for (const r of c.rows) {
      l.driverOverrides.push({ customerId: r.userId, name: r.name, carNumber: r.number, classId: l.classes[i].id, displayName: '', teamName: '', country: '', hidden: false });
    }
  });
  l.points.basis = l.classes.length > 1 ? 'class' : 'overall';
  return l;
}

export const leaguesPage: Page = {
  id: 'leagues',
  title: 'League builder',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    if (!cfg.leagues.some((l) => l.id === selected)) selected = cfg.activeLeague ?? cfg.leagues[0]?.id ?? '';

    const top = h('section', 'card');
    top.append(
      h('h2', '', 'Leagues'),
      h('p', 'dim', 'Make your own classes for league racing: group cars by number, driver, team or car, give each class a name and colour, set the points, and see live points in the relative and standings.'),
    );
    cfg.leagueMode ??= 'auto';
    const mode = h('select');
    for (const [v, t] of [
      ['auto', 'Automatic — match the session’s iRacing league ID'],
      ['always', 'Always use one league'],
    ]) {
      const o = h('option', '', t);
      o.value = v;
      mode.append(o);
    }
    mode.value = cfg.leagueMode;
    mode.onchange = () => {
      cfg.leagueMode = mode.value as 'auto' | 'always';
      save(true);
      rerender();
    };
    const active = h('select');
    const none = h('option', '', 'None (use iRacing classes)');
    none.value = '';
    active.append(none);
    for (const l of cfg.leagues) {
      const o = h('option', '', l.name || '(unnamed league)');
      o.value = l.id;
      active.append(o);
    }
    active.value = cfg.activeLeague ?? '';
    active.onchange = () => {
      cfg.activeLeague = active.value || null;
      save();
    };
    // What applies to the session you are in right now.
    const sess = state.session;
    const sid = sess?.leagueId ?? 0;
    const match = sid > 0 ? cfg.leagues.find((x) => x.iracingLeagueId === sid) : undefined;
    let now: string;
    if (!sess?.connected) now = 'Not in a session.';
    else if (cfg.leagueMode === 'always') now = `Using ${sess.league ? `“${sess.league}”` : 'no league'} in every session.`;
    else if (!sid) now = 'This session is not a league session (e.g. an official race), so no league is used.';
    else if (match) now = `This session belongs to iRacing league ${sid} → using “${match.name || 'unnamed league'}”.`;
    else now = `This session belongs to iRacing league ${sid}, but none of your leagues has that ID — no league is used. Import its league file, or set the ID on one of your leagues.`;
    const bar = h('div', 'toolbar');
    const add = (l: League) => {
      cfg.leagues.push(l);
      selected = l.id;
      save();
      rerender();
    };
    bar.append(
      btn('New league', () => add(newLeague('My league')), 'primary'),
      btn('Create from current session', () => {
        const l = fromSession();
        if (!l) return alert('Join a session first — the classes and drivers of that session are used.');
        add(l);
      }),
      btn('Import league file', async () => {
        let file: unknown;
        try {
          file = await upload();
        } catch {
          return alert('That file could not be read (it is not a JSON file).');
        }
        if (isCommentatorLeague(file)) {
          // From the commentator app: update the league with the same iRacing league ID, or add it.
          const leagueId = Number(file.league.league_id) || 0;
          const same = leagueId > 0 ? cfg.leagues.find((x) => x.iracingLeagueId === leagueId) : undefined;
          // Teams with their logos (stored like uploaded ones).
          const teams = await teamsFromFile({ teams: (file.league as { teams?: [] }).teams, images: (file as { images?: Record<string, string> }).images }).catch(() => []);
          if (same && confirm(`Update “${same.name || 'unnamed league'}” (iRacing league ${leagueId}) with this file?\n\nOK: update it. Cancel: add it as a new league.`)) {
            Object.assign(same, fromCommentator(file, same));
            same.teams = teams.map((t) => ({ ...t, shared: same.teams?.find((x) => x.name.toLowerCase() === t.name.toLowerCase())?.shared ?? [] }));
            selected = same.id;
            save();
            rerender();
            return;
          }
          return add({ ...fromCommentator(file, newLeague(file.league.name || 'Imported league')), teams });
        }
        const l = file as League;
        if (!l || !Array.isArray(l.classes)) return alert('That file is not a league export (from Apex Overlays or the commentator app).');
        add({ ...newLeague(l.name), ...l, id: uid('league') });
      }),
    );
    for (const l of cfg.leagues) {
      bar.append(
        btn(l.name || '(unnamed)', () => {
          selected = l.id;
          rerender();
        }, `tab${l.id === selected ? ' sel' : ''}`),
      );
    }
    top.append(
      labeled('Which league to use', mode, 'Automatic switches the league on in sessions of that league and off everywhere else'),
      cfg.leagueMode === 'always' ? labeled('League', active) : h('span'),
      h('p', `small ${match || cfg.leagueMode === 'always' ? '' : 'dim'}`, now),
      bar,
    );

    const l = cfg.leagues.find((x) => x.id === selected);
    if (!l) {
      el.replaceChildren(top);
      return;
    }
    l.driverOverrides ??= [];
    const ed = h('section', 'card');
    const name = h('input', 'title-input');
    name.value = l.name;
    name.className = '';
    name.oninput = () => {
      l.name = name.value;
      save();
    };
    const fallback = h('select');
    for (const [v, t] of [
      ['iracing', 'Keep their iRacing class'],
      ['other', 'Put them in an “Other” class'],
    ]) {
      const o = h('option', '', t);
      o.value = v;
      fallback.append(o);
    }
    fallback.value = l.fallback || 'iracing';
    fallback.onchange = () => {
      l.fallback = fallback.value as League['fallback'];
      save();
    };
    const actions = h('div', 'toolbar');
    actions.append(
      btn('Add class', () => {
        l.classes.push({ id: uid('c'), name: `Class ${l.classes.length + 1}`, color: PALETTE[l.classes.length % PALETTE.length], rules: emptyRules() });
        save();
        rerender();
      }, 'primary'),
      Object.assign(
        btn('Add secondary class', () => {
          l.classes.push({ id: uid('c'), name: `Cup ${l.classes.filter((k) => k.role === 'secondary').length + 1}`, color: PALETTE[(l.classes.length + 3) % PALETTE.length], rules: emptyRules(), role: 'secondary' });
          save();
          rerender();
        }),
        { title: 'A class running across the main ones, e.g. a Rookie Cup or a Pro-Am trophy: picked for each driver in the roster’s 2nd class column' },
      ),
      Object.assign(btn('Team builder →', () => (location.hash = '#teams')), { title: 'Teams of this league: names, short names, colours, logos and who drives for each' }),
      btn('Export league', () => download(`${l.name || 'league'}.json`, l)),
      btn('Delete league', () => {
        if (!confirm(`Delete ${l.name}?`)) return;
        cfg.leagues = cfg.leagues.filter((x) => x !== l);
        if (cfg.activeLeague === l.id) cfg.activeLeague = null;
        save();
        rerender();
      }, 'danger'),
    );
    const meta = l.importedAt ? h('p', 'dim small', `Last imported from iRacing ${new Date(Number(l.importedAt) * 1000).toLocaleString()}`) : h('span');
    const leagueId = h('input');
    leagueId.type = 'number';
    leagueId.placeholder = 'optional';
    leagueId.style.width = '140px';
    name.style.width = '260px';
    leagueId.value = l.iracingLeagueId ? String(l.iracingLeagueId) : '';
    leagueId.oninput = () => {
      l.iracingLeagueId = parseInt(leagueId.value, 10) || 0;
      save();
    };
    const season = h('input');
    season.placeholder = 'e.g. Season 3';
    season.style.width = '260px';
    season.value = l.seasonName ?? '';
    season.oninput = () => {
      l.seasonName = season.value;
      save();
    };
    ed.append(
      h('h2', '', 'League info'),
      h('p', 'dim', 'Everything here can be entered by hand, or brought in with “Import league file”.'),
      labeled('League name', name),
      labeled('iRacing league ID', leagueId, 'Used automatically in sessions of this league'),
      labeled('Season', season),
      meta,
      labeled('Cars matching no class', fallback),
      h('h3', '', 'Classes'),
      actions,
      h(
        'p',
        'dim small',
        'A driver’s class comes from the roster; otherwise the first class whose rules match. Rules ignore case; * is a wildcard; separate values with commas.',
      ),
    );
    l.classes.forEach((c, i) => ed.append(classEditor(l, c, i, rerender)));

    el.replaceChildren(top, ed, rosterCard(l, rerender), pointsCard(l, rerender));
  },
};
