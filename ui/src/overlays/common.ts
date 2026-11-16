// Column renderers shared by relative, standings and battle.

import { h, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { columnOn, type Values } from '../lib/settings';
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
function nameCell(c: HTMLElement, name: string, r: NameRow, s: Values) {
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
    c.replaceChildren(fin, marks, pen, label, pit);
  }
  const pitText = s.pitTag === false ? '' : r.onPitRoad ? 'PIT' : r.inWorld === false ? 'OUT' : '';
  text(pit, pitText);
  if (pit.hidden !== !pitText) pit.hidden = !pitText;
  const finished = s.finishFlag !== false && !!r.finished;
  if (fin.hidden !== !finished) fin.hidden = !finished;
  const flags = s.driverFlags !== false ? (r.flags ?? []) : [];
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
  const penalty = s.penaltyBox !== false ? (r.penalty ?? '') : '';
  text(pen, penalty);
  if (pen.hidden !== !penalty) pen.hidden = !penalty;
  toggle(pen, 'served', !!r.penaltyServed);
  text(label, name);
}

export function driverCols<R extends Driver & Partial<Extras>>(s: Values): Col<R>[] {
  const nameStyle = s.nameStyle as string;
  // No licence column: the iRating pill shows the licence by its colour instead.
  const licTint = s.irLicenseColor !== false && !columnOn(s, 'license');
  return [
    { id: 'number', label: '#', width: '2.9em', align: 'center', render: (c, r) => numberChip(c, r.number, r, s) },
    { id: 'flag', label: 'Nat', width: '2em', align: 'center', render: (c, r) => text(c, s.flagStyle === 'code' && r.country !== 'GLOBE' ? r.country.replace(/^GB-/, '') : flagEmoji(r.country)) },
    { id: 'name', label: 'Driver', width: 'minmax(4em, 1fr)', render: (c, r) => nameCell(c, f.name(r.name, nameStyle), r as NameRow, s) },
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
  help: 'Secondary class: drivers without one show just the number, with no background. Team colour: drivers in a team with no colour (or in no team) keep their class colour',
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
export const stripeColor = (r: Colored, s: Values) => colorFrom(r, s.stripeColor === 'number' || s.stripeColor == null ? s.numberColor : s.stripeColor) || 'transparent';

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
    { id: 'fuelTotal', label: 'Fuel Σ', width: '3.4em', align: 'center', render: (c, r) => text(c, r.fuelAdded ? fuelText(r.fuelAdded.total, r.fuelAdded.unit, units()) : '') },
    { id: 'seasonPoints', label: 'Champ', width: '3.2em', align: 'center', render: (c, r) => text(c, r.seasonPoints == null ? '' : String(+r.seasonPoints.toFixed(1))) },
  ];
}

/** Settings shared by every column-based overlay. */
export const COLUMN_STYLE_SETTINGS = [
  { ...NUMBER_COLOR_SETTING, when: (v: Values) => columnOn(v, 'number'), group: 'Column style' },
  {
    key: 'stripeColor', when: (v: Values) => v.classStripe !== false,
    label: 'Stripe colour',
    type: 'select' as const,
    default: 'number',
    options: [['number', 'As the car number'], ['class', 'Main class colour'], ['secondary', 'Secondary class colour'], ['team', 'Team colour']] as [string, string][],
    help: 'The coloured stripe by each row. Secondary class: no stripe for drivers without one; team colour: the class colour for a team with no colour',
    group: 'Column style',
  },
  { key: 'teamStyle', when: (v: Values) => columnOn(v, 'team'), label: 'Team name', type: 'select' as const, default: 'full', options: [['full', 'Full name'], ['short', 'Short name (Team builder), when set']] as [string, string][], group: 'Column style' },
  { key: 'teamLogoKind', when: (v: Values) => columnOn(v, 'teamLogo'), label: 'Team logo', type: 'select' as const, default: 'full', options: [['full', 'Full logo'], ['small', 'Small logo']] as [string, string][], help: 'From the Team builder. A team with only one of the two shows that one', group: 'Column style' },
  { key: 'flagStyle', when: (v: Values) => columnOn(v, 'flag'), label: 'Flag style', type: 'select' as const, default: 'emoji', options: [['emoji', 'Flag'], ['code', 'Country code']] as [string, string][], group: 'Column style' },
  {
    key: 'makeStyle', when: (v: Values) => columnOn(v, 'make'),
    label: 'Manufacturer style',
    type: 'select' as const,
    default: 'logo',
    options: [['logo', 'Logo (falls back to short name)'], ['short', 'Short name'], ['full', 'Full name']] as [string, string][],
    help: 'Logos for iRacing’s manufacturers are built in; your own images in the data folder under logos/ (e.g. porsche.png) take priority',
    group: 'Column style',
  },
  { key: 'makePlate', when: (v: Values) => columnOn(v, 'make') && v.makeStyle === 'logo', label: 'Dark background behind manufacturer logos', type: 'bool' as const, default: false, group: 'Column style', help: 'The logos are made for dark overlays; turn this on for light themes or a see-through overlay over bright scenery' },
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
    help: 'With the licence column hidden, each iRating pill takes the colour of the driver’s licence (A blue, B green, C yellow, D orange, R red)',
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
  { key: 'finishFlag', when: (v: Values) => columnOn(v, 'name'), label: 'Chequered flag next to drivers who have finished', type: 'bool' as const, default: true, group: 'Column style' },
  {
    key: 'driverFlags', when: (v: Values) => columnOn(v, 'name'),
    label: 'Flags next to drivers (caused a yellow, black, meatball, blue…)',
    type: 'bool' as const,
    default: true,
    group: 'Column style',
  },
  { key: 'penaltyBox', when: (v: Values) => columnOn(v, 'name'), label: 'Penalty box next to drivers (DT, S&G 40s, +5s)', type: 'bool' as const, default: true, group: 'Column style' },
  { key: 'pitTag', when: (v: Values) => columnOn(v, 'name'), label: 'PIT after the name while on pit road', type: 'bool' as const, default: true, group: 'Column style' },
  { key: 'highlightBest', label: 'Colour lap times (purple = class best, green = personal best)', type: 'bool' as const, default: true, group: 'Colours' },
];

/** Tyre badge: a white circle with D for dry tyres, a blue one with W for wets. */
export function tireCell(c: HTMLElement, wet: boolean | null) {
  let b = c.firstElementChild as HTMLElement | null;
  if (!b) {
    b = h('span', 'tyre');
    c.append(b);
  }
  text(b, wet == null ? '' : wet ? 'W' : 'D');
  if (b.hidden !== (wet == null)) b.hidden = wet == null;
  toggle(b, 'wet', wet === true);
}
