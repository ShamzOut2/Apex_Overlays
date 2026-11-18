// Column renderers shared by relative, standings and battle.

import { h, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { columnOn, type SettingDef, type Values } from '../lib/settings';
import { BADGE_COLORS, classBadge } from '../lib/classBadges';
import { chip, contrast, type Col } from '../lib/table';
import type { Driver, Extras } from '../lib/types';
import type { Units } from '../lib/format';

/** ISO code → flag emoji (rendered by the bundled flag font). Handles GB-SCT style UK nations. */
const flags = new Map<string, string>();

/** The flag emoji for a country code (cached: tables ask for every row on every update). */
export function flagEmoji(code: string): string {
  let f = flags.get(code);
  if (f === undefined) {
    f = makeFlag(code);
    if (flags.size > 400) flags.clear();
    flags.set(code, f);
  }
  return f;
}

function makeFlag(code: string): string {
  if (!code) return '';
  // iRacing's "Global" flair.
  if (code === 'GLOBE') return '\u{1F310}';
  if (code.toUpperCase().startsWith('GB-')) {
    const tag = `gb${code.slice(3).toLowerCase()}`;
    return `\u{1F3F4}${[...tag].map((c) => String.fromCodePoint(0xe0000 + c.charCodeAt(0))).join('')}\u{E007F}`;
  }
  if (!/^[A-Za-z]{2}$/.test(code)) return '';
  return [...code.toUpperCase()].map((c) => String.fromCodePoint(0x1f1e6 + c.charCodeAt(0) - 65)).join('');
}

const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

/** Class logo lookup, probed once per name: the user's images in <data dir>/logos/<name>.svg|png|webp. */
const logos = new Map<string, string | null>();
function probeLogo(make: string, done: () => void) {
  const key = slug(make);
  if (logos.has(key)) return;
  logos.set(key, null);
  const urls = ['svg', 'png', 'webp'].map((e) => `/logos/${key}.${e}`);
  const next = (i: number) => {
    if (i >= urls.length) return;
    const img = new Image();
    img.onload = () => {
      logos.set(key, img.src);
      done();
    };
    img.onerror = () => next(i + 1);
    img.src = urls[i];
  };
  next(0);
}

/**
 * Class logo next to a class name: the user's image logos/class-<name>.png (e.g. class-gt3.png),
 * or nothing. Returns the image element to show, created once per name.
 */
export function classLogo(name: string, onFound: () => void): string | null {
  const key = `class-${name}`;
  if (!logos.has(slug(key))) probeLogo(key, onFound);
  return logos.get(slug(key)) ?? null;
}

/** Makes whose logo could not be loaded, and when to try again. */
const noLogo = new Map<string, number>();

function makeCell(c: HTMLElement, make: string, style: string, plate = true) {
  if (!make) return text(c, '');
  // One request finds the logo (your own image or the bundled one); if it fails the short
  // name is shown and the logo is tried again a little later.
  const key = slug(make);
  const url = style === 'logo' && key && !((noLogo.get(key) ?? 0) > Date.now()) ? `/make-logo/${key}` : null;
  if (url) {
    let img = c.firstElementChild as HTMLImageElement | null;
    if (!img || img.tagName !== 'IMG') {
      img = h('img', 'make-logo');
      c.replaceChildren(img);
    }
    if (img.getAttribute('src') !== url) {
      // Hidden until it has loaded, so a missing logo never flashes a broken image.
      img.style.visibility = 'hidden';
      img.onload = () => (img!.style.visibility = '');
      img.onerror = () => {
        noLogo.set(key, Date.now() + 30_000);
        c.replaceChildren();
      };
      img.src = url;
    }
    toggle(img, 'plate', plate);
    img.alt = make;
    return;
  }
  let b = c.firstElementChild as HTMLElement | null;
  if (!b || b.tagName !== 'SPAN') {
    b = h('span', 'make-badge');
    c.replaceChildren(b);
  }
  text(b, style === 'full' ? make : make.replace(/[^A-Za-z0-9]/g, '').slice(0, 3).toUpperCase());
}

/** Driver name with an optional chequered flag once they have finished the race. */
const FLAG_TITLES: Record<string, string> = {
  yellow: 'Caused a yellow (off track / stopped)',
  black: 'Black flag',
  disqualify: 'Disqualified',
  repair: 'Meatball — damage, must pit',
  slow: 'Slow-down penalty — they must give time back, so you may pass',
  blue: 'Blue flag — being lapped',
};

type NameRow = Partial<Extras> & { onPitRoad?: boolean; inWorld?: boolean };

/**
 * Driver name with the chequered flag once finished, flags for this car (caused a yellow,
 * black, meatball, blue, ...), a penalty box (DT / S&G 40s / +5s) and, right of the name,
 * PIT while on pit road (OUT when the car has left the world, e.g. in the garage).
 */
export function nameCell(c: HTMLElement, name: string, r: NameRow, s: Values) {
  let fin = c.querySelector(':scope > .fin') as HTMLElement | null;
  let marks = c.querySelector(':scope > .marks') as HTMLElement | null;
  let pen = c.querySelector(':scope > .pen') as HTMLElement | null;
  let label = c.querySelector(':scope > .nm') as HTMLElement | null;
  let pit = c.querySelector(':scope > .pit-tag') as HTMLElement | null;
  if (!fin || !marks || !pen || !label || !pit) {
    fin = h('span', 'fin');
    fin.title = 'Finished';
    marks = h('span', 'marks');
    pen = h('span', 'pen');
    label = h('span', 'nm');
    pit = h('span', 'pit-tag');
    // Flags and penalties left or right of the name (before the PIT tag).
    if (s.markersSide === 'right') c.replaceChildren(fin, label, marks, pen, pit);
    else c.replaceChildren(fin, marks, pen, label, pit);
  }
  const pitText = s.pitTag === false ? '' : r.onPitRoad ? 'PIT' : r.inWorld === false ? 'OUT' : '';
  text(pit, pitText);
  if (pit.hidden !== !pitText) pit.hidden = !pitText;
  const finished = s.finishFlag !== false && !!r.finished;
  if (fin.hidden !== !finished) fin.hidden = !finished;
  // Only when shown next to the name (else outside the overlay, or not at all).
  markers(marks, pen, r, (s.markersAt ?? 'name') !== 'name');
  text(label, name);
}

/** The flags (yellow, black, meatball, blue, SLOW…) and the penalty box of a driver. */
function markers(marks: HTMLElement, pen: HTMLElement, r: NameRow, off = false) {
  const flags = !off ? (r.flags ?? []) : [];
  const key = flags.join();
  if (marks.dataset.k !== key) {
    marks.dataset.k = key;
    marks.replaceChildren(
      ...flags.map((f) => {
        // The slow-down is a word badge (SLOW), the rest small flags.
        const m = f === 'slow' ? h('span', 'slow-tag', 'SLOW') : h('span', `mk mk-${f}`);
        m.title = FLAG_TITLES[f] ?? f;
        return m;
      }),
    );
  }
  const slow = marks.querySelector(':scope > .slow-tag') as HTMLElement | null;
  if (slow && r.slowdown) slow.title = `${FLAG_TITLES.slow} · ${Math.round(r.slowdown.seconds)}s so far, about ${r.slowdown.lost.toFixed(1)}s given up`;
  const penalty = !off ? (r.penalty ?? '') : '';
  text(pen, penalty);
  if (pen.hidden !== !penalty) pen.hidden = !penalty;
  toggle(pen, 'served', !!r.penaltyServed);
}

/** A last column with each driver's flags and penalty box (standings: "at the end of the row"). */
export function markersCell(c: HTMLElement, r: NameRow) {
  let marks = c.querySelector(':scope > .marks') as HTMLElement | null;
  let pen = c.querySelector(':scope > .pen') as HTMLElement | null;
  if (!marks || !pen) {
    marks = h('span', 'marks');
    pen = h('span', 'pen');
    c.replaceChildren(marks, pen);
  }
  markers(marks, pen, r);
}

/** The driver column's width: what the overlay leaves (or the longest name with Auto width),
 *  or a set width (Driver column width setting). */
function nameWidth(s: Values): string {
  const w = Number(s.nameWidth);
  return s.nameFixed === true && w > 0 ? `${w}em` : 'minmax(4em, 1fr)';
}

export function driverCols<R extends Driver & Partial<Extras>>(s: Values): Col<R>[] {
  const nameStyle = s.nameStyle as string;
  // No licence column: the iRating pill shows the licence by its colour instead.
  const licTint = s.irLicenseColor !== false && !columnOn(s, 'license');
  return [
    { id: 'number', label: '#', width: '2.9em', align: 'center', render: (c, r) => numberChip(c, r.number, r, s) },
    { id: 'flag', label: 'Nat', width: '2em', align: 'center', render: (c, r) => text(c, s.flagStyle === 'code' && r.country !== 'GLOBE' ? r.country.replace(/^GB-/, '') : flagEmoji(r.country)) },
    { id: 'name', label: 'Driver', width: nameWidth(s), render: (c, r) => nameCell(c, f.name(r.name, nameStyle), r as NameRow, s) },
    { id: 'team', label: 'Team', width: 'minmax(3em, 0.8fr)', render: (c, r) => text(c, r.team === r.name ? '' : s.teamStyle === 'short' && r.teamShort ? r.teamShort : r.team) },
    { id: 'class2', label: '2nd', width: 'minmax(3em, auto)', align: 'center', render: (c, r) => (r.class2Name ? chip(c, r.class2Name, r.class2Color) : c.firstChild && c.replaceChildren()) },
    { id: 'teamLogo', label: '', width: '2.6em', align: 'center', render: (c, r) => teamLogoCell(c, r, s.teamLogoKind === 'small') },
    { id: 'make', label: 'Make', width: s.makeStyle === 'full' ? '5.5em' : '3em', align: 'center', render: (c, r) => makeCell(c, r.make, s.makeStyle as string, s.makePlate === true) },
    {
      id: 'license',
      label: 'Lic',
      width: '3.4em',
      align: 'center',
      // No licence (games other than iRacing): an empty cell rather than a blank chip.
      render: (c, r) => (r.license ? chip(c, r.license.split(' ')[0] + ' ' + (r.license.split(' ')[1] ?? '').slice(0, 3), r.licenseColor) : c.firstChild && c.replaceChildren()),
    },
    { id: 'irating', label: 'iR', width: s.irGain !== false ? '5em' : '3.2em', align: 'center', render: (c, r) => iratingCell(c, r.irating, s.irGain !== false ? r.irDelta : null, licTint ? r.licenseColor : null) },
    { id: 'car', label: 'Car', width: 'minmax(3em, 0.7fr)', render: (c, r) => text(c, r.car) },
    {
      id: 'class',
      label: 'Class',
      width: s.classStyle === 'badge' ? '4em' : '3.6em',
      align: 'center',
      render: (c, r) => {
        const url = s.classStyle === 'badge' ? classBadge(r.className, r.classId, r.classColor, [r.car], s.classBadgeColor as string, () => c.replaceChildren()) : null;
        if (!url) {
          if (c.firstElementChild?.tagName === 'IMG') c.replaceChildren();
          return chip(c, r.className, r.classColor);
        }
        let img = c.firstElementChild as HTMLImageElement | null;
        if (!img || img.tagName !== 'IMG') {
          img = h('img', 'class-badge');
          c.replaceChildren(img);
        }
        if (img.getAttribute('src') !== url) img.src = url;
        img.alt = r.className;
      },
    },
  ];
}

/** iRating in a frosted pill, with the estimated race gain (green ▲) or loss (red ▼) beside it.
 *  With `tint` (the driver's licence colour) the pill takes that colour. */
export function iratingCell(c: HTMLElement, ir: number, delta: number | null | undefined, tint?: string | null) {
  let pill = c.querySelector(':scope > .ir-pill') as HTMLElement | null;
  let val = pill?.querySelector(':scope > .ir-val') as HTMLElement | null;
  let d = pill?.querySelector(':scope > .ir-delta') as HTMLElement | null;
  if (!pill || !val || !d) {
    pill = h('span', 'ir-pill');
    val = h('span', 'ir-val');
    d = h('span', 'ir-delta');
    pill.append(val, d);
    c.replaceChildren(pill);
  }
  text(val, ir > 0 ? f.iRating(ir) : '');
  const bg = tint && /^#[0-9a-f]{6}$/i.test(tint) ? tint : '';
  if (pill.style.getPropertyValue('--lic') !== bg) {
    if (bg) {
      pill.style.setProperty('--lic', bg);
      pill.style.setProperty('--lic-fg', contrast(bg));
    } else {
      pill.style.removeProperty('--lic');
      pill.style.removeProperty('--lic-fg');
    }
    toggle(pill, 'lic', !!bg);
  }
  if (pill.hidden !== !(ir > 0)) pill.hidden = !(ir > 0);
  const v = delta == null ? 0 : Math.round(delta);
  text(d, v > 0 ? `▲${v}` : v < 0 ? `▼${-v}` : '');
  if (d.hidden !== !v) d.hidden = !v;
  toggle(d, 'up', v > 0);
  toggle(d, 'down', v < 0);
}

function signedCell(c: HTMLElement, v: number | null, digits: number) {
  text(c, v == null ? '' : f.signed(v, digits) || '0');
  toggle(c, 'good', v != null && v > 0);
  toggle(c, 'bad', v != null && v < 0);
}

/** Columns from the per-car extras (stint, ratings, points, ...). */
/** Where the car number's background colour comes from (Number colour setting). */
export const NUMBER_COLOR_SETTING = {
  key: 'numberColor',
  label: 'Number colour',
  type: 'select' as const,
  default: 'class',
  options: [
    ['class', 'Main class colour'],
    ['secondary', 'Secondary class colour (League builder)'],
    ['team', 'Team colour (League builder → Team builder)'],
  ] as [string, string][],
  help: 'Drivers without a secondary class or team colour keep the normal look',
};

type Colored = { classColor: string; class2Color?: string; teamColor?: string };

/** A colour from the main class, the secondary class or the team ('' when the driver has no secondary class). */
export function colorFrom(r: Colored, from: unknown): string {
  if (from === 'secondary') return r.class2Color ?? '';
  if (from === 'team' && r.teamColor) return r.teamColor;
  return r.classColor;
}

/** The car number's background (Number colour setting). */
export const numberBg = (r: Colored, s: Values) => colorFrom(r, s.numberColor);

/** A car number chip in its colour; just the number when there is none. */
export function numberChip(c: HTMLElement, label: string, r: Colored, s: Values) {
  const bg = numberBg(r, s);
  if (bg) chip(c, label, bg);
  else chip(c, label, 'transparent', 'inherit');
}

/** The colour stripe by each row (Stripe colour setting): the number's colour, a class's or the team's. */
export const stripeColor = (r: Colored, s: Values) => colorFrom(r, s.stripeColor === 'number' ? s.numberColor : s.stripeColor) || 'transparent';

/** Team logo cell: the full or small logo (whichever the team has when only one). */
function teamLogoCell(c: HTMLElement, r: Driver, small: boolean) {
  const url = small ? r.teamLogoSmall || r.teamLogo : r.teamLogo || r.teamLogoSmall;
  let img = c.firstElementChild as HTMLImageElement | null;
  if (!url) {
    if (img) img.hidden = true;
    return;
  }
  if (!img) {
    img = h('img', 'team-logo') as HTMLImageElement;
    img.alt = '';
    c.append(img);
  }
  img.hidden = false;
  // The same address is not set again (it would reload the image).
  if (img.dataset.src !== url) {
    img.dataset.src = url;
    img.src = url;
  }
}

/** Fuel put in at stops: litres (or gallons) for the player in iRacing, % of a tank in LMU. */
function fuelText(v: number, unit: string, u: Units): string {
  if (unit === '%') return `${Math.round(v)}%`;
  const n = f.fuel(v, u);
  return `${n.toFixed(n < 10 ? 1 : 0)}${f.fuelUnit(u) === 'gal' ? 'g' : 'L'}`;
}

export function extraCols<R extends Driver & Extras>(s: Values, units: () => Units = () => 'metric'): Col<R>[] {
  return [
    { id: 'lap', label: 'Lap', width: '2.6em', align: 'center', render: (c, r) => text(c, r.lap ? String(r.lap) : '') },
    {
      id: 'stint',
      label: 'Stint',
      width: '3.6em',
      align: 'center',
      render: (c, r) => {
        text(c, r.stintState === 'pit' ? 'PIT' : r.stintState === 'out' ? (s.outLapText as string) : String(r.stintLap ?? ''));
        toggle(c, 'outlap', r.stintState === 'out');
        toggle(c, 'inpit', r.stintState === 'pit');
      },
    },
    {
      id: 'bestLap',
      label: 'Best',
      width: '4.7em',
      align: 'center',
      render: (c, r) => {
        text(c, r.bestLap ? f.lapTime(r.bestLap) : '');
        toggle(c, 'best', s.highlightBest !== false && r.classBest);
      },
    },
    {
      id: 'lastLap',
      label: 'Last',
      width: '4.7em',
      align: 'center',
      render: (c, r) => {
        text(c, r.lastLap ? f.lapTime(r.lastLap) : '');
        // Purple = fastest lap of the class, green = the driver's personal best.
        toggle(c, 'best', s.highlightBest !== false && r.lastIsPb && r.classBest);
        toggle(c, 'pb', s.highlightBest !== false && r.lastIsPb && !r.classBest);
      },
    },
    { id: 'incidents', label: 'Inc', width: '2.4em', align: 'center', render: (c, r) => text(c, r.incidents == null ? '' : `${r.incidents}x`) },
    { id: 'srDelta', label: 'SR ±', width: '3.2em', align: 'center', render: (c, r) => signedCell(c, r.srDelta, 2) },
    {
      id: 'points',
      label: 'Pts',
      width: '2.8em',
      align: 'center',
      render: (c, r) => {
        text(c, r.points == null ? '' : `${s.pointsPlus ? '+' : ''}${+r.points.toFixed(1)}`);
        toggle(c, 'good', r.points != null && r.points > 0);
      },
    },
    {
      id: 'fuelAdded',
      label: 'Fuel+',
      width: '3.4em',
      align: 'center',
      render: (c, r) => {
        const a = r.fuelAdded;
        text(c, a ? `+${fuelText(a.last, a.unit, units())}` : '');
        toggle(c, 'filling', !!a?.filling);
      },
    },
    {
      id: 'fuelLeft',
      label: 'Fuel',
      width: '3.4em',
      align: 'center',
      render: (c, r) => {
        const l = r.fuelLeft;
        text(c, l ? fuelText(l.value, l.unit, units()) : '');
        // Virtual energy in its own colour; running low (under 10% of a tank) in red.
        toggle(c, 'fuel-ve', !!l?.energy);
        toggle(c, 'fuel-low', !!l && l.unit === '%' && l.value < 10);
        c.title = l ? (l.energy ? 'Virtual energy left' : 'Fuel left') : '';
      },
    },
    {
      id: 'pitTime',
      label: 'Pit time',
      width: '5.4em',
      align: 'center',
      render: (c, r) => {
        const p = r.pitTime;
        // A penalty stop (stop-and-go, served while black-flagged) is marked as such; a stop
        // that was both shows the service time, marked +P.
        const pen = !!p && p.penalty > 0 && p.service < 0.5;
        text(c, !p ? '' : pen ? `S&G ${p.penalty.toFixed(1)}` : `${p.service.toFixed(1)}${p.penalty > 0 ? ' +P' : ''}`);
        toggle(c, 'pit-live', !!p?.live);
        toggle(c, 'pit-pen', pen);
        c.title = !p
          ? ''
          : [p.service > 0 ? `Serviced for ${p.service.toFixed(1)} s` : '', p.penalty > 0 ? `Penalty served: ${p.penalty.toFixed(1)} s` : ''].filter(Boolean).join('; ') +
            (p.live ? ' (this stop)' : ' (last stop)');
      },
    },
    { id: 'fuelTotal', label: 'Fuel Σ', width: '3.4em', align: 'center', render: (c, r) => text(c, r.fuelAdded ? fuelText(r.fuelAdded.total, r.fuelAdded.unit, units()) : '') },
    { id: 'seasonPoints', label: 'Champ', width: '3.2em', align: 'center', render: (c, r) => text(c, r.seasonPoints == null ? '' : String(+r.seasonPoints.toFixed(1))) },
  ];
}

/** The colour of the bar by each driver (relative and standings). */
export const STRIPE_COLOR_SETTING = {
  key: 'stripeColor',
  label: 'Colour bar by each driver',
  type: 'select' as const,
  default: 'class',
  options: [
    ['class', 'Main class colour'],
    ['secondary', 'Secondary class colour (League builder)'],
    ['team', 'Team colour (League builder → Team builder)'],
    ['number', 'As the car number'],
  ] as [string, string][],
  help: 'Drivers without a secondary class or team colour keep their class colour',
  group: 'Colours',
};

/** Settings shared by every column-based overlay. */
export const COLUMN_STYLE_SETTINGS = [
  { ...NUMBER_COLOR_SETTING, when: (v: Values) => columnOn(v, 'number'), group: 'Column style' },
  { key: 'nameFixed', when: (v: Values) => columnOn(v, 'name'), label: 'Set the driver column width', type: 'bool' as const, default: false, group: 'Column style', help: 'Off: the driver column takes the room the overlay leaves (with Auto width, the longest name)' },
  { key: 'nameWidth', when: (v: Values) => columnOn(v, 'name') && v.nameFixed === true, label: 'Driver column width', type: 'number' as const, default: 10, min: 3, max: 40, step: 0.5, unit: 'em', group: 'Column style', help: 'Longer names are cut short with …' },
  { key: 'teamStyle', when: (v: Values) => columnOn(v, 'team'), label: 'Team name', type: 'select' as const, default: 'full', options: [['full', 'Full name'], ['short', 'Short name (Team builder), when set']] as [string, string][], group: 'Column style' },
  { key: 'teamLogoKind', when: (v: Values) => columnOn(v, 'teamLogo'), label: 'Team logo', type: 'select' as const, default: 'full', options: [['full', 'Full logo'], ['small', 'Small logo']] as [string, string][], help: 'From the Team builder. A team with only one of the two shows that one', group: 'Column style' },
  { key: 'flagStyle', when: (v: Values) => columnOn(v, 'flag'), label: 'Flag style', type: 'select' as const, default: 'emoji', options: [['emoji', 'Flag'], ['code', 'Country code']] as [string, string][], group: 'Column style' },
  {
    key: 'makeStyle', when: (v: Values) => columnOn(v, 'make'),
    label: 'Manufacturer style',
    type: 'select' as const,
    default: 'logo',
    options: [['logo', 'Logo (falls back to short name)'], ['short', 'Short name'], ['full', 'Full name']] as [string, string][],
    help: 'Built in for iRacing’s cars. Your own images go in the data folder under logos/ (e.g. porsche.png)',
    group: 'Column style',
  },
  { key: 'makePlate', when: (v: Values) => columnOn(v, 'make') && v.makeStyle === 'logo', label: 'Dark background behind manufacturer logos', type: 'bool' as const, default: false, group: 'Column style', help: 'For light themes or see-through overlays over bright scenery' },
  {
    key: 'classStyle', when: (v: Values) => columnOn(v, 'class'),
    label: 'Class column',
    type: 'select' as const,
    default: 'badge',
    options: [['badge', 'Class badge (GT3, LMP2…)'], ['chip', 'Class name in its colour']] as [string, string][],
    group: 'Column style',
  },
  {
    key: 'classBadgeColor', when: (v: Values) => columnOn(v, 'class') && v.classStyle !== 'chip',
    label: 'Class badge colour',
    type: 'select' as const,
    default: 'match',
    options: BADGE_COLORS,
    help: 'Match: the badge colour closest to the class colour in this session (iRacing’s or your league’s)',
    group: 'Column style',
  },
  {
    key: 'irLicenseColor', when: (v: Values) => columnOn(v, 'irating') && !columnOn(v, 'license'),
    label: 'iRating in the licence colour',
    type: 'bool' as const,
    default: true,
    help: 'With the licence column hidden, the iRating pill takes the licence colour',
    group: 'Column style',
  },
  {
    key: 'irGain', when: (v: Values) => columnOn(v, 'irating'),
    label: 'iRating gain/loss next to iRating',
    type: 'bool' as const,
    default: true,
    help: 'Estimated race result: green ▲ gain, red ▼ loss (official races)',
    group: 'Column style',
  },
  { key: 'outLapText', when: (v: Values) => columnOn(v, 'stint'), label: 'Out-lap label', type: 'text' as const, default: 'OUTLAP', group: 'Column style' },
  { key: 'pointsPlus', when: (v: Values) => columnOn(v, 'points'), label: 'Show points as gains (+25)', type: 'bool' as const, default: true, group: 'Column style' },
  { key: 'finishFlag', when: (v: Values) => columnOn(v, 'name'), label: 'Chequered flag when finished', type: 'bool' as const, default: true, group: 'Column style' },
  { key: 'pitTag', when: (v: Values) => columnOn(v, 'name'), label: 'PIT after the name while on pit road', type: 'bool' as const, default: true, group: 'Column style' },
  { key: 'highlightBest', label: 'Colour best laps (purple class, green personal)', type: 'bool' as const, default: true, group: 'Colours' },
];

/**
 * Where each driver's flags (yellow, black, meatball, blue, SLOW) and penalties (DT, S&G…)
 * show: next to the name, outside the overlay as an extension of it (standings), or not at
 * all; and on which side (of the name, or of the overlay).
 */
export function markerSettings(outside: boolean): SettingDef[] {
  return [
    {
      key: 'markersAt',
      label: 'Flags & penalties: where to show',
      type: 'select',
      default: 'name',
      options: [['name', 'Next to the driver’s name'], ...(outside ? ([['end', 'Outside the overlay (an extension of it)']] as [string, string][]) : []), ['off', 'Don’t show']],
      group: 'Columns',
      help: 'Yellow, black, meatball and blue flags, SLOW, and penalties like DT or S&G',
    },
    {
      key: 'markersSide',
      when: (v) => v.markersAt !== 'off',
      label: 'Flags & penalties: side',
      type: 'select',
      default: 'left',
      options: [
        ['left', 'Left'],
        ['right', 'Right'],
      ],
      group: 'Columns',
      help: outside ? 'Left or right of the name, or on the left or right of the overlay' : 'Left or right of the name',
    },
  ];
}

/** Tyre badge: a white circle with D for dry tyres, a blue one with W for wets. */
export function tireCell(c: HTMLElement, wet: boolean | null) {
  let b = c.firstElementChild as HTMLElement | null;
  let l = b?.firstElementChild as HTMLElement | null;
  if (!b || !l) {
    b = h('span', 'tyre');
    // The letter in a box of its own, trimmed to its capital height, so it sits in the
    // middle of the circle whatever the font.
    l = h('span', 'tl');
    b.append(l);
    c.replaceChildren(b);
  }
  text(l, wet == null ? '' : wet ? 'W' : 'D');
  if (b.hidden !== (wet == null)) b.hidden = wet == null;
  toggle(b, 'wet', wet === true);
}

/**
 * The modern look of the relative and standings (Layout → Look), and the colours of its gap
 * bars and battle lines. `kind`: which of the two (the gap bar is the relative's, the battle
 * line the standings').
 */
export function lookSettings(kind: 'relative' | 'standings'): SettingDef[] {
  const modern = (v: Values) => (v.look ?? 'modern') === 'modern';
  return [
    {
      key: 'look',
      label: 'Look',
      type: 'select',
      default: 'modern',
      options: [
        ['modern', 'Modern (pit wall)'],
        ['classic', 'Classic'],
      ],
      group: 'Layout',
      help: 'Modern: condensed figures, slanted number chips and class headers' + (kind === 'relative' ? ', gap bars and gain/loss arrows' : ', battle lines and podium colours'),
    },
    {
      key: 'figuresFont', when: modern,
      label: 'Figures font',
      type: 'select',
      default: 'display',
      options: [
        ['display', 'Barlow Condensed'],
        ['overlay', 'The overlay font (Theme)'],
      ],
      group: 'Layout',
      help: 'For positions, numbers, gaps and lap times. Names keep the overlay font',
    },
    { key: 'closeGap', when: modern, label: 'Close: within', type: 'number', default: 1, min: 0.1, max: 5, step: 0.1, unit: 's', group: 'Colours', help: kind === 'relative' ? 'Cars this close turn the gap bar and the gap the “close” colour' : 'Cars this close to the car ahead are joined by a battle line' },
    ...(kind === 'relative'
      ? ([
          { key: 'gapBar', when: modern, label: 'Gap bar (fills as a car gets closer)', type: 'bool', default: true, group: 'Colours' },
          { key: 'gapBarRange', when: (v) => modern(v) && v.gapBar !== false, label: 'Gap bar starts at', type: 'number', default: 5, min: 1, max: 30, step: 0.5, unit: 's', group: 'Colours', help: 'Cars further away have no bar' },
          { key: 'gapBarColor', when: (v) => modern(v) && v.gapBar !== false, label: 'Gap bar colour', type: 'color', default: '#ffffff', group: 'Colours' },
          { key: 'closeColor', when: modern, label: 'Close colour (bar and gap)', type: 'color', default: '#3dd6f5', group: 'Colours' },
          { key: 'gapTrend', when: modern, label: 'Gain/loss arrows by the gap', type: 'bool', default: true, group: 'Colours', help: 'Green when the gap is moving your way (catching a car ahead, pulling away from one behind), red when not' },
        ] as SettingDef[])
      : ([
          { key: 'battleLine', when: modern, label: 'Battle lines (cars close together)', type: 'bool', default: true, group: 'Colours' },
          { key: 'closeColor', when: (v) => modern(v) && v.battleLine !== false, label: 'Battle line colour', type: 'color', default: '#3dd6f5', group: 'Colours', help: 'Also the colour of a close interval' },
          { key: 'podium', when: modern, label: 'Gold, silver and bronze for the top three', type: 'bool', default: true, group: 'Colours' },
        ] as SettingDef[])),
  ];
}

/** Applies the look (modern/classic) and its colours to an overlay's root. */
export function applyLook(root: HTMLElement, s: Values) {
  const modern = (s.look ?? 'modern') === 'modern';
  root.classList.toggle('modern', modern);
  if (!modern) return;
  root.classList.toggle('figures-overlay', s.figuresFont === 'overlay');
  root.style.setProperty('--close', (s.closeColor as string) || '#3dd6f5');
  root.style.setProperty('--gapbar', (s.gapBarColor as string) || '#ffffff');
}
