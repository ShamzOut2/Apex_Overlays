// Global colour theme, applied as CSS variables to the overlays AND the settings
// window. Built-in presets can be applied, tweaked and saved as custom themes.

import { merge, type SettingDef, type Values } from './settings';

const STANDARD_FONT = "'Segoe UI', 'Inter', system-ui, sans-serif";

/** Fonts for the overlays (the settings window keeps its own). `stretch` picks a width of a
 *  variable font, e.g. Bahnschrift's condensed cut. */
export const FONTS: { id: string; label: string; family: string; stretch?: string }[] = [
  { id: 'standard', label: 'Standard (Segoe UI)', family: STANDARD_FONT },
  { id: 'bahnschrift', label: 'Bahnschrift Condensed (as the fuel calculator)', family: "'Bahnschrift', 'Segoe UI', system-ui, sans-serif", stretch: 'condensed' },
];

export const THEME: SettingDef[] = [
  {
    key: 'font',
    label: 'Overlay font',
    type: 'select',
    default: 'standard',
    options: [...FONTS.map((f): [string, string] => [f.id, f.label]), ['custom', 'Custom (type a font name)']],
    help: 'Every overlay uses it; the settings window keeps its own font',
    group: 'Text',
  },
  { key: 'fontFamily', when: (v) => v.font === 'custom', label: 'Custom font', type: 'text', default: STANDARD_FONT, help: 'Any font installed on this PC, e.g. Arial Narrow; list a few, separated by commas', group: 'Text' },
  { key: 'fontSize', label: 'Base font size', type: 'number', default: 13, min: 8, max: 32, step: 0.5, unit: 'px', group: 'Text' },
  { key: 'fontWeight', label: 'Font weight', type: 'select', default: '500', options: [['400', 'Regular'], ['500', 'Medium'], ['600', 'Semi bold'], ['700', 'Bold']], group: 'Text' },
  { key: 'textColor', label: 'Text', type: 'color', default: '#f2f4f8', group: 'Colours' },
  { key: 'dimColor', label: 'Secondary text', type: 'color', default: '#98a2b3', group: 'Colours' },
  { key: 'background', label: 'Background', type: 'color', default: '#0b0e13', group: 'Colours' },
  { key: 'headerBackground', label: 'Header background', type: 'color', default: '#161b24', group: 'Colours' },
  { key: 'accent', label: 'Accent', type: 'color', default: '#ff3d57', group: 'Colours' },
  { key: 'playerColor', label: 'Your car highlight', type: 'color', default: '#ffd84d', group: 'Colours' },
  { key: 'lapAheadColor', label: 'Car a lap ahead of you', type: 'color', default: '#ff7a7a', group: 'Colours' },
  { key: 'lapBehindColor', label: 'Car a lap behind you', type: 'color', default: '#6cb4ff', group: 'Colours' },
  { key: 'pitColor', label: 'Car in pits', type: 'color', default: '#7d8593', group: 'Colours' },
  { key: 'bestColor', label: 'Fastest lap', type: 'color', default: '#c27bff', group: 'Colours' },
  { key: 'goodColor', label: 'Gaining / good', type: 'color', default: '#3ddc84', group: 'Colours' },
  { key: 'badColor', label: 'Losing / warning', type: 'color', default: '#ff5c5c', group: 'Colours' },
  { key: 'warnColor', label: 'Caution', type: 'color', default: '#ffb020', group: 'Colours' },
  { key: 'backgroundOpacity', label: 'Background opacity (all overlays)', type: 'number', default: 80, min: 0, max: 100, unit: '%', group: 'Transparency' },
  { key: 'overlayOpacity', label: 'Overlay opacity (all overlays)', type: 'number', default: 100, min: 10, max: 100, unit: '%', group: 'Transparency' },
  { key: 'radius', label: 'Corner radius', type: 'number', default: 6, min: 0, max: 24, unit: 'px', group: 'Shape' },
  { key: 'rowAlt', label: 'Alternate row shading', type: 'number', default: 4, min: 0, max: 30, unit: '%', group: 'Shape' },
  { key: 'rowPadding', label: 'Row padding', type: 'number', default: 0.22, min: 0, max: 1, step: 0.02, unit: 'em', group: 'Shape' },
  { key: 'border', label: 'Outline', type: 'bool', default: true, group: 'Shape' },
  { key: 'shadow', label: 'Text shadow (better on bright scenes)', type: 'bool', default: true, group: 'Shape' },
];

/** The theme's values, with the font choice worked out: themes saved before the font list
 *  existed keep a font they typed in (as "custom"). */
export function themeValues(stored: Values | undefined): Values {
  const t = merge(THEME, stored);
  if (stored?.font == null && typeof stored?.fontFamily === 'string' && stored.fontFamily.trim() && stored.fontFamily !== STANDARD_FONT) t.font = 'custom';
  return t;
}

/** CSS font family and width for the overlays. */
export function overlayFont(t: Values): { family: string; stretch: string } {
  const f = FONTS.find((x) => x.id === t.font);
  if (f) return { family: f.family, stretch: f.stretch ?? 'normal' };
  return { family: (t.fontFamily as string) || STANDARD_FONT, stretch: 'normal' };
}

type Preset = { name: string; values: Values };

/** Built-in colour themes. Only colours are set; fonts and shapes are left as they are. */
export const PRESETS: Preset[] = [
  { name: 'Apex Overlays (default)', values: {} },
  {
    name: 'Midnight Blue',
    values: { background: '#070b14', headerBackground: '#0f1830', textColor: '#e9f0ff', dimColor: '#8ea0c4', accent: '#3d8bff', playerColor: '#7cf2ff', bestColor: '#c27bff', goodColor: '#40e0a0', badColor: '#ff6b81', lapAheadColor: '#ff8fa3', lapBehindColor: '#7fb2ff' },
  },
  {
    name: 'Racing Green',
    values: { background: '#07110c', headerBackground: '#0f2218', textColor: '#ecf7f0', dimColor: '#8fb09c', accent: '#1fbf62', playerColor: '#f4d35e', bestColor: '#c27bff', goodColor: '#5ee08f', badColor: '#ff6b6b', lapAheadColor: '#ff8a80', lapBehindColor: '#80c8ff' },
  },
  {
    name: 'Papaya',
    values: { background: '#121212', headerBackground: '#1d1d1d', textColor: '#f7f3ee', dimColor: '#a8a29a', accent: '#ff8000', playerColor: '#47c7fc', bestColor: '#c27bff', goodColor: '#3ddc84', badColor: '#ff5c5c', lapAheadColor: '#ff9e80', lapBehindColor: '#47c7fc' },
  },
  {
    name: 'Rosso',
    values: { background: '#140608', headerBackground: '#2a0b10', textColor: '#fff1f1', dimColor: '#c39b9f', accent: '#e10600', playerColor: '#ffd700', bestColor: '#c27bff', goodColor: '#4cd98a', badColor: '#ff7070', lapAheadColor: '#ff9a9a', lapBehindColor: '#8ec5ff' },
  },
  {
    name: 'Monochrome',
    values: { background: '#0c0c0c', headerBackground: '#1a1a1a', textColor: '#f5f5f5', dimColor: '#9a9a9a', accent: '#ffffff', playerColor: '#ffffff', bestColor: '#d0d0d0', goodColor: '#e0e0e0', badColor: '#8a8a8a', lapAheadColor: '#bdbdbd', lapBehindColor: '#bdbdbd', pitColor: '#6a6a6a', warnColor: '#d0d0d0' },
  },
  {
    name: 'High Contrast',
    values: { background: '#000000', headerBackground: '#000000', textColor: '#ffffff', dimColor: '#d0d0d0', accent: '#ffe600', playerColor: '#ffe600', bestColor: '#ff4dff', goodColor: '#00ff6a', badColor: '#ff3b3b', lapAheadColor: '#ff6b6b', lapBehindColor: '#4dc3ff', backgroundOpacity: 92, shadow: false },
  },
  // Team themes: each team's three colours as background, accent (header line, badges) and
  // your-car highlight, with text and status colours chosen to stay readable on them.
  {
    name: 'Gassed Up Motorsports',
    values: { background: '#3e424d', headerBackground: '#30333b', textColor: '#ffffff', dimColor: '#bcc1cc', accent: '#01fddc', playerColor: '#ffb400', bestColor: '#d08cff', goodColor: '#3ddc84', badColor: '#ff5c5c', lapAheadColor: '#ff8a8a', lapBehindColor: '#7cc0ff' },
  },
  {
    name: 'VCR',
    values: { background: '#000000', headerBackground: '#140000', textColor: '#ffffff', dimColor: '#a8a8a8', accent: '#e60000', playerColor: '#e60000', bestColor: '#c27bff', goodColor: '#3ddc84', badColor: '#ff5c5c', lapAheadColor: '#ffa04d', lapBehindColor: '#6cb4ff' },
  },
  {
    name: 'JWB',
    values: { background: '#271831', headerBackground: '#1c1024', textColor: '#ffffff', dimColor: '#c4b6d0', accent: '#ffb400', playerColor: '#ffb400', bestColor: '#e08cff', goodColor: '#3ddc84', badColor: '#ff6b6b', lapAheadColor: '#ff8a8a', lapBehindColor: '#7cc0ff' },
  },
  {
    name: 'Light',
    values: { background: '#f5f7fa', headerBackground: '#e4e8ef', textColor: '#11161d', dimColor: '#5b6472', accent: '#e10600', playerColor: '#b07a00', bestColor: '#8a2be2', goodColor: '#0f9d58', badColor: '#d93025', lapAheadColor: '#c62828', lapBehindColor: '#1565c0', pitColor: '#8a8f98', warnColor: '#c77700', backgroundOpacity: 88, shadow: false },
  },
];

export function rgb(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [n >> 16, (n >> 8) & 255, n & 255];
}

export function rgba(hex: string, alpha: number): string {
  const c = rgb(hex);
  return c ? `rgba(${c[0]}, ${c[1]}, ${c[2]}, ${alpha})` : hex;
}

/** Mix two hex colours: t=0 → a, t=1 → b. */
export function mix(a: string, b: string, t: number): string {
  const x = rgb(a);
  const y = rgb(b);
  if (!x || !y) return a;
  return `#${x.map((v, i) => Math.round(v + (y[i] - v) * t).toString(16).padStart(2, '0')).join('')}`;
}

/** WCAG relative luminance (0 = black, 1 = white). */
function luminance(c: [number, number, number]): number {
  const [r, g, b] = c.map((v) => {
    const x = v / 255;
    return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Readable text colour on a coloured background: white unless it would be hard to read (e.g. on yellow). */
const onColors = new Map<string, string>();

/** Dark or light text, whichever reads better on `hex` (cached: overlays ask every frame). */
export function onColor(hex: string, dark = '#0b0e13', light = '#ffffff'): string {
  const key = `${hex}|${dark}|${light}`;
  let v = onColors.get(key);
  if (v === undefined) {
    const c = rgb(hex);
    v = !c || 1.05 / (luminance(c) + 0.05) >= 3 ? light : dark;
    if (onColors.size > 500) onColors.clear();
    onColors.set(key, v);
  }
  return v;
}

export function isLight(hex: string): boolean {
  const c = rgb(hex);
  return !!c && 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2] > 150;
}

export function applyTheme(stored: Values | undefined, el: HTMLElement, overrides: { scale?: number; bgOpacity?: number; opacity?: number } = {}) {
  const t = themeValues(stored);
  const s = el.style;
  const bgOpacity = overrides.bgOpacity != null && overrides.bgOpacity >= 0 ? overrides.bgOpacity : (t.backgroundOpacity as number);
  const text = t.textColor as string;
  const font = overlayFont(t);
  s.setProperty('--font', `'Twemoji Country Flags', ${font.family}`);
  s.setProperty('--font-stretch', font.stretch);
  s.setProperty('--font-size', `${((t.fontSize as number) * (overrides.scale ?? 100)) / 100}px`);
  s.setProperty('--weight', t.fontWeight as string);
  s.setProperty('--text', text);
  s.setProperty('--dim', t.dimColor as string);
  s.setProperty('--bg', rgba(t.background as string, bgOpacity / 100));
  s.setProperty('--bg-solid', t.background as string);
  s.setProperty('--header', rgba(t.headerBackground as string, Math.min(1, bgOpacity / 100 + 0.1)));
  s.setProperty('--accent', t.accent as string);
  s.setProperty('--player', t.playerColor as string);
  s.setProperty('--player-bg', rgba(t.playerColor as string, 0.16));
  s.setProperty('--lap-ahead', t.lapAheadColor as string);
  s.setProperty('--lap-behind', t.lapBehindColor as string);
  s.setProperty('--pit', t.pitColor as string);
  s.setProperty('--best', t.bestColor as string);
  s.setProperty('--good', t.goodColor as string);
  s.setProperty('--bad', t.badColor as string);
  s.setProperty('--warn', t.warnColor as string);
  // Text drawn on top of theme colours (badges, banners) stays readable with bright accents.
  for (const [v, key] of [['accent', 'accent'], ['best', 'bestColor'], ['good', 'goodColor'], ['bad', 'badColor'], ['warn', 'warnColor'], ['player', 'playerColor']]) {
    s.setProperty(`--on-${v}`, onColor(t[key] as string));
  }
  s.setProperty('--radius', `${t.radius}px`);
  // Shading and outlines derive from the text colour so light themes work too.
  s.setProperty('--row-alt', rgba(text, (t.rowAlt as number) / 100));
  s.setProperty('--subtle', rgba(text, 0.06));
  s.setProperty('--track', rgba(text, 0.1));
  s.setProperty('--row-pad', `${t.rowPadding}em`);
  s.setProperty('--border', t.border ? `1px solid ${rgba(text, 0.09)}` : 'none');
  s.setProperty('--shadow', t.shadow ? (isLight(t.background as string) ? '0 1px 1px rgba(255,255,255,0.6)' : '0 1px 2px rgba(0,0,0,0.8)') : 'none');
  s.setProperty('--opacity', String(((t.overlayOpacity as number) / 100) * ((overrides.opacity ?? 100) / 100)));
  return t;
}

/** Colours for the settings window, derived from the same theme. */
export function applyUiTheme(stored: Values | undefined, el: HTMLElement) {
  const t = merge(THEME, stored);
  const bg = t.background as string;
  const text = t.textColor as string;
  const light = isLight(bg);
  const s = el.style;
  s.setProperty('--bg', light ? mix(bg, '#000000', 0.03) : mix(bg, '#000000', 0.15));
  s.setProperty('--panel', light ? '#ffffff' : mix(bg, text, 0.05));
  s.setProperty('--panel-2', light ? mix(bg, '#000000', 0.05) : mix(bg, text, 0.09));
  s.setProperty('--line', light ? mix(bg, '#000000', 0.12) : mix(bg, text, 0.15));
  s.setProperty('--text', text);
  s.setProperty('--dim', t.dimColor as string);
  s.setProperty('--accent', t.accent as string);
  s.setProperty('--good', t.goodColor as string);
  s.setProperty('--bad', t.badColor as string);
  s.setProperty('--accent-text', onColor(t.accent as string));
  s.setProperty('--shade', light ? 'rgba(20, 30, 45, 0.10)' : 'rgba(0, 0, 0, 0.35)');
  s.setProperty('color-scheme', light ? 'light' : 'dark');
}
