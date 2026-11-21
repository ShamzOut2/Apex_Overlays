// Team builder: the teams of a league (name, short name, colour, full and small logo), who
// drives for each, and teams kept in step across leagues. Opened from the League builder;
// edits the same league. A driver's team is their roster Team field.

import { h } from '../lib/dom';
import type { DriverOverride, League, LeagueTeam } from '../lib/types';
import { selectedLeague, selectLeague } from './leagues';
import type { Page } from './pages';
import * as csv from './signupCsv';
import { download, save, state } from './state';

export const logoUrl = (file: string) => `/team-logos/${encodeURIComponent(file)}`;
const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();
const newTeam = (name: string): LeagueTeam => ({ name, short: '', color: '', logo: '', logoSmall: '', shared: [] });

/** The league's teams, with any team named on the roster but not made yet added. */
export function teamsOf(l: League): LeagueTeam[] {
  l.teams ??= [];
  for (const d of l.driverOverrides) {
    const name = (d.teamName ?? '').trim();
    if (name && !l.teams.some((t) => same(t.name, name))) l.teams.push(newTeam(name));
  }
  for (const t of l.teams) {
    t.short ??= '';
    t.color ??= '';
    t.logo ??= '';
    t.logoSmall ??= '';
    t.shared ??= [];
  }
  return l.teams;
}

const leagueById = (id: string) => state.config?.leagues.find((x) => x.id === id);

/** The same driver in another league: by iRacing customer ID, else by name. */
function counterpart(o: League, d: DriverOverride): DriverOverride | undefined {
  if (d.customerId) {
    const m = o.driverOverrides.find((x) => x.customerId === d.customerId);
    if (m) return m;
  }
  const name = (d.name || d.displayName).trim();
  return name ? o.driverOverrides.find((x) => same(x.name || x.displayName || '', name)) : undefined;
}

/**
 * Carries a team over to the leagues it is shared with: its details, and its drivers who race
 * there too (when they have no team there, or are in another team shared with this league).
 */
export function syncShared(l: League, t: LeagueTeam) {
  t.shared = t.shared.filter((id) => id !== l.id && !!leagueById(id));
  for (const id of t.shared) {
    const o = leagueById(id)!;
    const theirs = teamsOf(o);
    let copy = theirs.find((x) => same(x.name, t.name));
    if (!copy) {
      copy = newTeam(t.name);
      theirs.push(copy);
    }
    Object.assign(copy, { name: t.name, short: t.short, color: t.color, logo: t.logo, logoSmall: t.logoSmall });
    if (!copy.shared.includes(l.id)) copy.shared.push(l.id);
    const linked = (name: string) => theirs.some((x) => same(x.name, name) && x.shared.includes(l.id));
    for (const d of l.driverOverrides.filter((x) => same(x.teamName ?? '', t.name))) {
      const m = counterpart(o, d);
      if (m && (!m.teamName?.trim() || (!same(m.teamName, t.name) && linked(m.teamName)))) m.teamName = t.name;
    }
  }
}

/** Puts a driver in a team (or none), here and in the leagues the new team is shared with. */
function setTeam(l: League, d: DriverOverride, team: LeagueTeam | null) {
  d.teamName = team?.name ?? '';
  if (team) syncShared(l, team);
}

/** A picked image as a team logo file: pictures are scaled to at most 512 px, SVGs kept as they are. */
async function uploadLogo(file: File): Promise<string> {
  let body: Blob = file;
  if (!/svg/i.test(file.type)) {
    const img = await createImageBitmap(file);
    const k = Math.min(1, 512 / Math.max(img.width, img.height));
    const c = document.createElement('canvas');
    c.width = Math.max(1, Math.round(img.width * k));
    c.height = Math.max(1, Math.round(img.height * k));
    c.getContext('2d')!.drawImage(img, 0, 0, c.width, c.height);
    body = await new Promise<Blob>((ok, fail) => c.toBlob((b) => (b ? ok(b) : fail(new Error('Could not read the image'))), 'image/png'));
  }
  const r = await fetch('/api/team-logo', { method: 'POST', body });
  if (!r.ok) throw new Error(await r.text());
  return ((await r.json()) as { file: string }).file;
}

/** A logo from an export (base64), stored like an uploaded one. */
async function storeImage(b64: string): Promise<string> {
  const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  const r = await fetch('/api/team-logo', { method: 'POST', body: bytes });
  if (!r.ok) throw new Error(await r.text());
  return ((await r.json()) as { file: string }).file;
}

async function imageBase64(file: string): Promise<string | null> {
  const r = await fetch(logoUrl(file));
  if (!r.ok) return null;
  const bytes = new Uint8Array(await r.arrayBuffer());
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** Teams export: the same shape as the commentator app's, so either app can read the other's. */
async function exportTeams(l: League) {
  const teams = teamsOf(l);
  const images: Record<string, string> = {};
  for (const f of new Set(teams.flatMap((t) => [t.logo, t.logoSmall]).filter(Boolean))) {
    const b = await imageBase64(f);
    if (b) images[f] = b;
  }
  download(`${l.name || 'league'} - Teams.json`, {
    apex: 'teams',
    format: 1,
    app: 'Apex Overlays',
    exported_at: new Date().toISOString(),
    league: l.name,
    teams: teams.map((t) => ({ name: t.name, short: t.short, color: t.color, logo: t.logo || null, logo_small: t.logoSmall || null })),
    drivers: l.driverOverrides
      .filter((d) => d.teamName?.trim())
      .map((d) => ({ key: d.customerId ? `cust:${d.customerId}` : `name:${d.name.trim().toLowerCase()}`, cust_id: d.customerId || null, name: d.name, team: d.teamName })),
    images,
  });
}

interface TeamsFile {
  teams?: { name?: string; short?: string; color?: string; logo?: string | null; logo_small?: string | null; logoSmall?: string | null }[];
  drivers?: { key?: string; cust_id?: number | null; name?: string; team?: string }[];
  images?: Record<string, string>;
}

/** Teams from an export file (this app's or the commentator app's), with their logos stored. */
export async function teamsFromFile(file: TeamsFile): Promise<LeagueTeam[]> {
  const stored = new Map<string, string>();
  const logo = async (name?: string | null) => {
    if (!name) return '';
    if (!stored.has(name)) {
      const b64 = file.images?.[name];
      stored.set(name, b64 ? await storeImage(b64).catch(() => '') : '');
    }
    return stored.get(name)!;
  };
  const out: LeagueTeam[] = [];
  for (const t of file.teams ?? []) {
    if (!t?.name?.trim()) continue;
    out.push({
      name: t.name.trim(),
      short: (t.short ?? '').slice(0, 8),
      color: /^#[0-9a-f]{6}$/i.test(t.color ?? '') ? t.color! : '',
      logo: await logo(t.logo),
      logoSmall: await logo(t.logo_small ?? t.logoSmall),
      shared: [],
    });
  }
  return out;
}

async function importTeams(l: League, file: TeamsFile): Promise<string> {
  const teams = await teamsFromFile(file);
  if (!teams.length) throw new Error('That file has no teams in it (a teams export from Apex Overlays or the commentator app).');
  const mine = teamsOf(l);
  for (const t of teams) {
    const had = mine.findIndex((x) => same(x.name, t.name));
    // Links to other leagues are this PC's own: a replaced team keeps the ones it had.
    if (had >= 0) mine[had] = { ...t, shared: mine[had].shared };
    else mine.push(t);
  }
  let joined = 0;
  for (const d of file.drivers ?? []) {
    const id = Number(String(d.key ?? '').replace(/^cust:/, '')) || Number(d.cust_id) || 0;
    const m = (id && l.driverOverrides.find((x) => x.customerId === id)) || l.driverOverrides.find((x) => !!d.name && same(x.name, d.name));
    if (m && d.team) {
      m.teamName = d.team;
      joined++;
    }
  }
  mine.forEach((t) => syncShared(l, t));
  return `${teams.length} team${teams.length === 1 ? '' : 's'} imported${joined ? `, ${joined} driver${joined === 1 ? '' : 's'} put in their team` : ''}.`;
}

function btn(label: string, fn: () => void, cls = '', title = '') {
  const b = h('button', cls, label);
  if (title) b.title = title;
  b.onclick = (e) => {
    e.preventDefault();
    fn();
  };
  return b;
}

function pickFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const i = document.createElement('input');
    i.type = 'file';
    i.accept = accept;
    i.onchange = () => resolve(i.files?.[0] ?? null);
    i.click();
  });
}

const driverLabel = (d: DriverOverride) => `${d.carNumber ? `#${d.carNumber} ` : ''}${d.displayName || d.name || 'Unnamed'}`;
const byName = (a: DriverOverride, b: DriverOverride) => (a.displayName || a.name).localeCompare(b.displayName || b.name);

// -- arrow keys between the boxes, as in a spreadsheet (the commentator app's) ---------------

const NAV = 'input:not([type=file]):not([type=hidden]):not([type=checkbox]), select';
const TEXTUAL = new Set(['text', 'search', 'email', 'url', 'tel', '']);

function nearest(container: HTMLElement, el: HTMLElement, key: string): HTMLElement | null {
  const a = el.getBoundingClientRect();
  const ax = a.left + a.width / 2;
  const ay = a.top + a.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const f of container.querySelectorAll<HTMLElement>(NAV)) {
    if (f === el || (f as HTMLInputElement).disabled || !f.offsetParent) continue;
    const b = f.getBoundingClientRect();
    const dx = b.left + b.width / 2 - ax;
    const dy = b.top + b.height / 2 - ay;
    let score: number;
    if (key === 'ArrowDown' || key === 'ArrowUp') {
      const ahead = key === 'ArrowDown' ? dy : -dy;
      if (ahead < a.height / 2) continue;
      score = ahead + 3 * Math.abs(dx);
    } else {
      const ahead = key === 'ArrowRight' ? dx : -dx;
      if (ahead < 4 || Math.abs(dy) > a.height) continue;
      score = ahead + 6 * Math.abs(dy);
    }
    if (score < bestScore) {
      bestScore = score;
      best = f;
    }
  }
  return best;
}

/** Arrow keys move to the nearest box in that direction (left/right from the ends of the text). */
export function arrowNav(container: HTMLElement) {
  container.addEventListener('keydown', (e) => {
    if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key) || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    const el = e.target as HTMLInputElement;
    if (!el.matches?.(NAV)) return;
    const textual = el.tagName === 'INPUT' && TEXTUAL.has(el.type);
    if (textual && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      const all = el.selectionStart === 0 && el.selectionEnd === el.value.length;
      const atStart = all || (el.selectionStart === 0 && el.selectionEnd === 0);
      const atEnd = all || (el.selectionStart === el.value.length && el.selectionEnd === el.value.length);
      if ((e.key === 'ArrowLeft' && !atStart) || (e.key === 'ArrowRight' && !atEnd)) return;
    }
    if (el.tagName === 'SELECT' && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) return;
    const next = nearest(container, el, e.key);
    if (!next) return;
    e.preventDefault();
    next.focus();
    if (next instanceof HTMLInputElement && TEXTUAL.has(next.type)) next.select();
  });
}

// -- sign-up sheet ----------------------------------------------------------------------------

function csvDialog(l: League, done: (msg: string) => void) {
  pickFile('.csv,text/csv,.txt').then(async (file) => {
    if (!file) return;
    let sheet: { headers: string[]; rows: string[][] };
    try {
      sheet = csv.read(await file.text());
    } catch (e) {
      return alert((e as Error).message);
    }
    const mapping = csv.guess(sheet.headers, sheet.rows);
    const dlg = h('dialog', 'csv-dialog') as HTMLDialogElement;
    const form = h('div', 'csv-map');
    const first = sheet.rows[0] ?? [];
    for (const f of csv.FIELDS) {
      const sel = h('select') as HTMLSelectElement;
      const none = h('option', '', '— not in the sheet —');
      none.value = '';
      sel.append(none);
      sheet.headers.forEach((hd, i) => {
        const o = h('option', '', `${hd || `Column ${i + 1}`}${first[i] ? ` (e.g. ${first[i].slice(0, 24)})` : ''}`);
        o.value = String(i);
        sel.append(o);
      });
      sel.value = mapping[f] != null ? String(mapping[f]) : '';
      sel.onchange = () => {
        if (sel.value === '') delete mapping[f];
        else mapping[f] = Number(sel.value);
      };
      const row = h('label', 'field');
      row.append(h('span', 'field-label', csv.FIELD_LABELS[f]), sel);
      form.append(row);
    }
    // The sheet's class column: each driver's main class, or their 2nd class (a Rookie Cup...).
    const role = h('select') as HTMLSelectElement;
    for (const [v, t] of [
      ['main', 'Their main class'],
      ['secondary', 'Their 2nd class (e.g. a Rookie Cup, a Pro-Am trophy)'],
    ]) {
      const o = h('option', '', t);
      o.value = v;
      role.append(o);
    }
    role.value = 'main';
    const roleRow = h('label', 'field');
    roleRow.append(h('span', 'field-label', 'The sheet’s class column is each driver’s'), role);
    form.append(roleRow);
    const go = btn('Import', () => {
      if (mapping.name == null && mapping.first == null && mapping.custId == null) return alert('Pick the column with the names or the iRacing IDs.');
      const s = csv.apply(l, sheet.rows, mapping, role.value as 'main' | 'secondary');
      teamsOf(l).forEach((t) => syncShared(l, t));
      dlg.close();
      dlg.remove();
      done(`${s.added} added, ${s.updated} updated${s.teams ? `, ${s.teams} new team${s.teams === 1 ? '' : 's'}` : ''}${s.classes ? `, ${s.classes} new class${s.classes === 1 ? '' : 'es'}` : ''}${s.skipped ? `, ${s.skipped} row${s.skipped === 1 ? '' : 's'} without a name or ID skipped` : ''}.`);
    }, 'primary');
    const cancel = btn('Cancel', () => {
      dlg.close();
      dlg.remove();
    });
    const bar = h('div', 'toolbar');
    bar.append(go, cancel);
    dlg.append(
      h('h2', '', 'Import a sign-up sheet'),
      h('p', 'dim', `${sheet.rows.length} row${sheet.rows.length === 1 ? '' : 's'} in the sheet, going into ${l.name || 'this league'}. Check which column is which:`),
      form,
      h('p', 'dim small', 'Drivers already on the roster are updated; new ones are added, with any new teams and classes.'),
      bar,
    );
    document.body.append(dlg);
    dlg.showModal();
  });
}

// -- the page ---------------------------------------------------------------------------------

function teamCard(l: League, t: LeagueTeam, i: number, rerender: () => void): HTMLElement {
  const teams = l.teams;
  const card = h('div', 'team-card');
  card.style.setProperty('--team', t.color || '#3a4150');
  const changed = () => {
    syncShared(l, t);
    save();
    rerender();
  };

  // Logos: the full one (the big box) and a small one (a badge for tight spaces).
  const logoBox = (kind: 'logo' | 'logoSmall') => {
    const big = kind === 'logo';
    const box = h('button', `team-logo ${big ? 'big' : 'small'}`);
    box.title = t[kind] ? `Replace the ${big ? 'full' : 'small'} logo` : big ? 'Upload the full logo' : 'Upload a small logo (a badge or icon): overlays can be set to show it instead';
    if (t[kind]) {
      const img = h('img') as HTMLImageElement;
      img.src = logoUrl(t[kind]);
      img.alt = '';
      box.append(img);
    } else box.append(big ? '+ Full logo' : '+ Small');
    box.onclick = async (e) => {
      e.preventDefault();
      const f = await pickFile('image/png,image/jpeg,image/webp,image/svg+xml');
      if (!f) return;
      try {
        t[kind] = await uploadLogo(f);
        changed();
      } catch (err) {
        alert(`The logo could not be added: ${(err as Error).message}`);
      }
    };
    return box;
  };
  const logos = h('div', 'team-logos');
  logos.append(logoBox('logo'), logoBox('logoSmall'));

  const name = h('input', 'team-name') as HTMLInputElement;
  name.value = t.name;
  name.placeholder = 'Team name';
  name.setAttribute('aria-label', 'Team name');
  name.onchange = () => {
    const v = name.value.trim();
    if (!v || teams.some((x) => x !== t && same(x.name, v))) {
      alert(v ? `There is already a team called ${v}.` : 'A team needs a name.');
      name.value = t.name;
      return;
    }
    // Renaming moves its drivers with it, here and in the leagues it is shared with.
    const old = t.name;
    for (const d of l.driverOverrides) if (same(d.teamName ?? '', old)) d.teamName = v;
    for (const id of t.shared) {
      const o = leagueById(id);
      const copy = o && teamsOf(o).find((x) => same(x.name, old));
      if (!o || !copy) continue;
      copy.name = v;
      for (const d of o.driverOverrides) if (same(d.teamName ?? '', old)) d.teamName = v;
    }
    t.name = v;
    changed();
  };
  const short = h('input', 'narrow') as HTMLInputElement;
  short.value = t.short;
  short.maxLength = 8;
  short.placeholder = 'Short';
  short.title = 'Short name, used where space is tight';
  short.onchange = () => {
    t.short = short.value.trim();
    changed();
  };
  const color = h('input') as HTMLInputElement;
  color.type = 'color';
  color.value = t.color || '#3a4150';
  color.title = 'Team colour';
  color.onchange = () => {
    t.color = color.value;
    changed();
  };
  const row = h('div', 'row');
  row.append(short, color);
  if (t.color) row.append(btn('No colour', () => ((t.color = ''), changed()), 'mini', 'Use the class colour instead'));
  if (t.logo) row.append(btn('Remove full logo', () => ((t.logo = ''), changed()), 'mini'));
  if (t.logoSmall) row.append(btn('Remove small logo', () => ((t.logoSmall = ''), changed()), 'mini'));
  row.append(
    btn('✕', () => {
      if (!confirm(`Delete ${t.name}? Its drivers stay in the league without a team.`)) return;
      for (const d of l.driverOverrides) if (same(d.teamName ?? '', t.name)) d.teamName = '';
      // Linked leagues keep their copy, no longer linked.
      for (const id of t.shared) {
        const copy = leagueById(id) && teamsOf(leagueById(id)!).find((x) => same(x.name, t.name));
        if (copy) copy.shared = copy.shared.filter((x) => x !== l.id);
      }
      teams.splice(i, 1);
      save();
      rerender();
    }, 'mini danger', 'Delete team'),
  );
  const fields = h('div', 'team-fields');
  fields.append(name, row);
  const head = h('div', 'team-card-head');
  head.append(logos, fields);

  // Drivers: chips with ✕, and a drop-down to add one (from another team, or none).
  const members = l.driverOverrides.filter((d) => same(d.teamName ?? '', t.name)).sort(byName);
  const chips = h('div', 'chips team-members');
  for (const d of members) {
    const c = h('span', 'chip', driverLabel(d));
    c.append(btn('✕', () => {
      d.teamName = '';
      save();
      rerender();
    }, 'clear', 'Take out of the team'));
    chips.append(c);
  }
  if (!members.length) chips.append(h('span', 'dim small', 'No drivers yet'));
  const add = h('select') as HTMLSelectElement;
  add.setAttribute('aria-label', `Add a driver to ${t.name}`);
  const ph = h('option', '', '+ Add driver…');
  ph.value = '';
  add.append(ph);
  l.driverOverrides
    .filter((d) => !same(d.teamName ?? '', t.name))
    .sort(byName)
    .forEach((d) => {
      const o = h('option', '', `${driverLabel(d)}${d.teamName ? ` (${d.teamName})` : ''}`);
      o.value = String(l.driverOverrides.indexOf(d));
      add.append(o);
    });
  add.onchange = () => {
    if (add.value === '') return;
    setTeam(l, l.driverOverrides[Number(add.value)], t);
    save();
    rerender();
  };
  card.append(head, chips, add);

  // Also in: other leagues this team races in (kept in step).
  const others = state.config!.leagues.filter((x) => x.id !== l.id);
  if (others.length) {
    const shared = h('div', 'team-shared');
    const hint = h('span', 'dim small', 'Also in:');
    hint.title = 'Tick a league to put the team, and its drivers who race there too, in that league as well. Changes here reach it from then on. ✕ takes the team out of that league.';
    shared.append(hint);
    for (const o of others) {
      const on = t.shared.includes(o.id);
      const chip = h('label', `chip${on ? ' active' : ''}`);
      chip.title = on ? `Untick: stop keeping ${o.name}’s copy in step (it keeps its copy)` : `Tick: put ${t.name} in ${o.name} too`;
      const box = h('input') as HTMLInputElement;
      box.type = 'checkbox';
      box.checked = on;
      box.hidden = true;
      box.onchange = () => {
        if (box.checked) {
          t.shared.push(o.id);
          syncShared(l, t);
        } else {
          t.shared = t.shared.filter((x) => x !== o.id);
          const copy = teamsOf(o).find((x) => same(x.name, t.name));
          if (copy) copy.shared = copy.shared.filter((x) => x !== l.id);
        }
        save();
        rerender();
      };
      chip.append(box, o.name || '(unnamed league)');
      if (on) {
        chip.append(btn('✕', () => {
          if (!confirm(`Take ${t.name} out of ${o.name}? Its drivers there are left without a team. ${l.name} keeps it.`)) return;
          o.teams = teamsOf(o).filter((x) => !same(x.name, t.name));
          for (const d of o.driverOverrides) if (same(d.teamName ?? '', t.name)) d.teamName = '';
          t.shared = t.shared.filter((x) => x !== o.id);
          save();
          rerender();
        }, 'clear', `Take ${t.name} out of ${o.name}`));
      }
      shared.append(chip);
    }
    card.append(shared);
  }
  return card;
}

export const teamsPage: Page = {
  id: 'teams',
  title: 'Team builder',
  render(el) {
    const cfg = state.config!;
    const rerender = () => {
      const y = window.scrollY;
      this.render(el);
      window.scrollTo(0, y);
    };
    const l = cfg.leagues.find((x) => x.id === selectedLeague());
    const top = h('section', 'card');
    const back = btn('← League builder', () => (location.hash = '#leagues'));
    const picker = h('select') as HTMLSelectElement;
    const ph = h('option', '', cfg.leagues.length ? 'Pick a league…' : 'No leagues yet');
    ph.value = '';
    picker.append(ph);
    [...cfg.leagues].sort((a, b) => a.name.localeCompare(b.name)).forEach((x) => {
      const o = h('option', '', x.name || '(unnamed league)');
      o.value = x.id;
      picker.append(o);
    });
    picker.value = l?.id ?? '';
    picker.onchange = () => {
      selectLeague(picker.value);
      rerender();
    };
    const pickRow = h('label', 'inline');
    pickRow.append('League ', picker);
    const headRow = h('div', 'toolbar');
    headRow.append(back, pickRow);
    top.append(headRow, h('h2', '', 'Teams'));
    if (!l) {
      top.append(h('p', 'dim', 'Pick a league above, or build one in the League builder first.'));
      el.replaceChildren(top);
      return;
    }
    const before = l.teams?.length ?? 0;
    const teams = teamsOf(l);
    // Teams only named on the roster so far are kept as teams.
    if (teams.length !== before) save();
    const status = h('p', 'small');
    const say = (msg: string) => {
      rerender();
      const s = el.querySelector('.teams-status');
      if (s) s.textContent = msg;
    };
    const bar = h('div', 'toolbar');
    bar.append(
      btn('+ Add team', () => {
        let n = teams.length + 1;
        while (teams.some((t) => same(t.name, `Team ${n}`))) n++;
        teams.push(newTeam(`Team ${n}`));
        save();
        rerender();
      }, 'primary'),
      btn('⤓ Export teams', () => exportTeams(l).catch((e) => alert((e as Error).message)), '', 'Teams, their logos and who drives for each (also opens in the commentator app)'),
      btn('⤒ Import teams…', async () => {
        const f = await pickFile('.json,application/json');
        if (!f) return;
        try {
          const msg = await importTeams(l, JSON.parse(await f.text()) as TeamsFile);
          save();
          say(msg);
        } catch (e) {
          alert((e as Error).message);
        }
      }, '', 'A teams export from Apex Overlays or the commentator app: teams with the same name are replaced, drivers join their team'),
      btn('⤒ Import a sign-up sheet (CSV)…', () => csvDialog(l, (msg) => {
        save();
        say(msg);
      }), '', 'Drivers, teams and classes from any spreadsheet, e.g. a sign-up sheet: iRacing ID, name, car number, team, team colour, class'),
    );
    status.className = 'small teams-status';
    top.append(
      h('p', 'dim', `The teams of ${l.name || 'this league'}. A team’s colour can colour its drivers’ car numbers in the relative, standings, battle box and safety alert (Number colour → Team, in each overlay’s settings; drivers in a team with no colour keep their class colour). The short name is used where space is tight. Changes are saved as you go.`),
      bar,
      status,
    );
    const cards = h('div', 'team-cards');
    teams.forEach((t, i) => cards.append(teamCard(l, t, i, rerender)));
    if (!teams.length) cards.append(h('p', 'dim', 'No teams yet: add one, import a sheet, or fill in the Team column on the League builder roster.'));
    arrowNav(cards);
    const free = l.driverOverrides.filter((d) => !d.teamName?.trim()).sort(byName);
    const freeCard = h('section', 'card');
    const freeChips = h('div', 'chips');
    for (const d of free) freeChips.append(h('span', 'chip', driverLabel(d)));
    if (!free.length) freeChips.append(h('span', 'dim small', l.driverOverrides.length ? 'Everyone is in a team.' : 'No drivers on the roster yet.'));
    freeCard.append(h('h2', '', 'Not in a team'), freeChips);
    const wrap = h('section', 'card team-wrap');
    wrap.append(cards);
    el.replaceChildren(top, wrap, freeCard);
  },
};
