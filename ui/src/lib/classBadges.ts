// Class badges (GT3, LMP2, TCR, ...): one SVG template per class, coloured to match the
// class colour iRacing (or the league) gives the class. Templates and the badge palette
// live in /assets/classes/badges.json, fetched once when first needed.

import { rgb } from './theme';

interface BadgeData {
  palette: Record<string, { bg: string; fg: string }>;
  groups: Record<string, string>;
  badges: Record<string, string>;
}

let data: BadgeData | null = null;
let loading: Promise<void> | null = null;
const waiting = new Set<() => void>();

function load(onReady: () => void) {
  waiting.add(onReady);
  loading ??= fetch('/assets/classes/badges.json')
    .then((r) => r.json())
    .then((d: BadgeData) => {
      data = d;
      waiting.forEach((f) => f());
      waiting.clear();
    })
    .catch(() => {});
}

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

/** Class name / car keywords → badge, most specific first. */
const RULES: [RegExp, string][] = [
  [/touring car|\btcr\b/, 'TCR'],
  [/porsche.*cup|carrera cup|cup.*992|\bpcup\b/, 'PCUP'],
  [/mx-?5/, 'MX-5'],
  [/gr ?86/, 'GR86'],
  [/hypercar|\blmh\b/, 'HY'],
  [/lmdh|bmw m power|m hybrid/, 'LMDh'],
  [/\bgtp\b/, 'GTP'],
  [/lmp2|p217|oreca 07/, 'LMP2'],
  [/lmp3|js p320|\bp320\b/, 'LMP3'],
  [/\bgtd\b|imsa23/, 'GTD'],
  [/\bgte\b|gtlm/, 'GTE'],
  [/\bgt3\b/, 'GT3'],
  [/\bgt4\b/, 'GT4'],
  [/\bm2(\s?cs)?\b/, 'M2'],
  [/hpd|arx-01|\blmp\b|prototype/, 'LMP'],
  [/rallycross|\brx\b|beetle|fiesta rs|wrx/, 'RX'],
  [/supercars?|\bv8\b|commodore|camaro zl1/, 'V8SC'],
  [/kart/, 'KART'],
  [/super formula lights|sf lights|\bsfl\b/, 'SFL'],
  [/super formula|\bsf23\b|\bsf\b/, 'SF'],
  // Before F1: Lovely Car Data calls the Ray FF1600's class "F1.6", and "Formula 1600" is
  // not Formula 1.
  [/ff1600|\bf1600\b|\bf1\.6\b|formula 1600|formula ford|\bray\b/, 'FF1600'],
  [/\bf1\b(?![.,]?\d)|formula (1|one)\b|w1[23]\b|mp4|fw31|lotus (49|79)/, 'F1'],
  [/\bf3\b(?![.,]?\d)|formula 3\b/, 'F3'],
  [/\bf4\b(?![.,]?\d)|formula 4\b|tatuus/, 'F4'],
  [/formula renault|\bfr ?(2\.0|3\.5)?\b/, 'FR'],
  [/formula vee|\bvee\b|\bfv\b/, 'FV'],
  [/usf ?2000|usf2k/, 'USF2K'],
  [/pro ?2000|pro2k|pro mazda|star mazda/, 'PRO2K'],
  [/skip barber|\bskip\b|rt2000/, 'SKIP'],
  [/ir-?18|indycar|\bindy\b/, 'INDY'],
  [/arca/, 'ARCA'],
  [/xfinity|\bxfin/, 'XFIN'],
  [/gen ?4/, 'GEN4'],
  [/truck/, 'TRUCK'],
  [/dirt legends?/, 'DIRT_LEG'],
  [/legends?/, 'LEGENDS'],
  [/mini ?stock/, 'MINI'],
  [/silver ?crown/, 'SILVER'],
  [/big block/, 'BBM'],
  [/dirt.*late model|late model.*dirt|\bdlm\b|pro late model|limited late model/, 'DLM'],
  [/super late model|\bslm\b|late model/, 'SLM'],
  [/\bump\b/, 'UMP'],
  [/sk mod|tour mod|modified/, 'SK_MOD'],
  [/street stock/, 'STREET'],
  [/\b410\b/, '410'],
  [/\b360\b/, '360'],
  [/\b305\b/, '305'],
  [/sprint/, 'SPRINT'],
  [/midget/, 'MIDGET'],
  [/next ?gen|nascar cup|\bnxg\b|\bcup\b/, 'CUP'],
  [/touring/, 'TOURING'],
  [/imsa/, 'IMSA'],
  // League tiers.
  [/\bpro[\s-]?am(ateurs?)?\b/, 'PRO-AM'],
  [/\bsemi[\s-]?pro(fessionals?)?\b/, 'SEMI-PRO'],
  [/\bpro(fessionals?)?\b/, 'PRO'],
  [/\b(am|amz|amateurs?|amatuers?)\b/, 'AM'],
];

/**
 * The badge for a class, from its name (exact, then word by word) or keywords in it and its cars.
 * League classes (ids starting "lg") are named by the league, so only their name is used: a
 * "Division 1" of GT3 cars is not shown as GT3.
 */
export function badgeFor(className: string, cars: string[] = [], classId = ''): string | null {
  if (!data) return null;
  const ids = Object.keys(data.badges);
  const n = norm(className);
  const exact = ids.find((id) => norm(id) === n);
  if (exact) return exact;
  const rule = (hay: string) => RULES.find(([re, id]) => re.test(hay.toLowerCase()) && ids.includes(id))?.[1] ?? null;
  const byName = rule(className);
  if (byName) return byName;
  // Whole words only: the "F1" in "F1.6" is not a word of its own.
  for (const word of className.split(/[^A-Za-z0-9.-]+/)) {
    const w = ids.find((id) => norm(id) === norm(word));
    if (w && norm(word).length >= 2) return w;
  }
  return classId.startsWith('lg') ? null : rule(`${className} ${cars.join(' ')}`);
}

/** iRacing's own class colours (official multiclass, single class) → the matching badge colour. */
const IRACING: Record<string, string> = {
  ffffff: 'white',
  ffda59: 'yellow',
  '33ceff': 'sky',
  ff5888: 'magenta',
  ae6bff: 'purple',
  '53ff77': 'lime',
};

function hsl(hex: string): [number, number, number] | null {
  const c = rgb(hex);
  if (!c) return null;
  const [r, g, b] = c.map((v) => v / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l];
  const s = d / (1 - Math.abs(2 * l - 1));
  const h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [(h * 60 + 360) % 360, s, l];
}

/** The badge palette colour closest to a class colour (by hue for colours, by lightness for greys). */
export function nearestBadgeColor(classColor: string): string {
  const key = classColor.replace('#', '').toLowerCase();
  if (IRACING[key]) return IRACING[key];
  const c = hsl(classColor);
  if (!c || !data) return 'white';
  const [h, s, l] = c;
  if (s < 0.2 || l > 0.92 || l < 0.1) return l > 0.6 ? 'white' : l > 0.25 ? 'grey' : 'black';
  let best = 'white';
  let bestD = Infinity;
  for (const [name, p] of Object.entries(data.palette)) {
    if (name === 'purple-gold') continue;
    const q = hsl(p.bg);
    if (!q || q[1] < 0.2) continue;
    const dh = Math.min(Math.abs(q[0] - h), 360 - Math.abs(q[0] - h)) / 180;
    const d = dh * 3 + Math.abs(q[2] - l);
    if (d < bestD) {
      bestD = d;
      best = name;
    }
  }
  return best;
}

const urls = new Map<string, string>();

/**
 * Data URL of a class badge, or null while loading / when no badge fits the class.
 * `colorMode`: 'match' (closest badge colour to the class colour), 'exact' (badge in the class
 * colour itself) or a palette name ('sky', 'purple-gold', ...).
 */
export function classBadge(className: string, classId: string, classColor: string, cars: string[], colorMode: string, onReady: () => void): string | null {
  if (!data) {
    load(onReady);
    return null;
  }
  const id = badgeFor(className, cars, classId);
  if (!id) return null;
  let bg: string;
  let fg: string;
  if (colorMode === 'exact' && rgb(classColor)) {
    bg = classColor;
    const c = rgb(classColor)!;
    fg = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2] > 150 ? '#111111' : '#FFFFFF';
  } else {
    const p = data.palette[colorMode] ?? data.palette[nearestBadgeColor(classColor)] ?? data.palette.white;
    ({ bg, fg } = p);
  }
  const key = `${id}|${bg}|${fg}`;
  let url = urls.get(key);
  if (!url) {
    const svg = data.badges[id].replaceAll('{bg}', bg).replaceAll('{fg}', fg);
    url = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    urls.set(key, url);
  }
  return url;
}

export const BADGE_COLORS: [string, string][] = [
  ['match', 'Match the class colour'],
  ['exact', 'Exactly the class colour'],
  ['white', 'White'],
  ['black', 'Black'],
  ['grey', 'Grey'],
  ['yellow', 'Yellow'],
  ['orange', 'Orange'],
  ['red', 'Red'],
  ['crimson', 'Crimson'],
  ['magenta', 'Magenta'],
  ['purple', 'Purple'],
  ['purple-gold', 'Purple and gold'],
  ['blue', 'Blue'],
  ['sky', 'Sky blue'],
  ['teal', 'Teal'],
  ['lime', 'Lime'],
];
