// Declarative settings schema. Overlays declare their options once; the
// control panel renders forms from it and overlays read merged values.

export interface ColumnDef {
  id: string;
  label: string;
}

export interface ColumnSetting {
  id: string;
  on: boolean;
}

interface Base {
  key: string;
  label: string;
  help?: string;
  group?: string;
  /** Shown only when this holds for the current values (e.g. a custom title only when the
   *  title is set to custom); hidden settings keep their values. */
  when?: (v: Values) => boolean;
}

/** Column `id` is switched on in a columns setting (or there is no such setting). */
export const columnOn = (v: Values, id: string, key = 'columns') => !Array.isArray(v[key]) || enabledColumns(v[key]).includes(id);

export type SettingDef =
  | (Base & { type: 'bool'; default: boolean })
  | (Base & { type: 'number'; default: number; min?: number; max?: number; step?: number; unit?: string })
  | (Base & { type: 'color'; default: string })
  | (Base & { type: 'select'; default: string; options: [string, string][] })
  | (Base & { type: 'text'; default: string; multiline?: boolean; placeholder?: string; /** Offer a list instead (see form.ts `CHOICES`). */ choices?: string })
  | (Base & { type: 'columns'; default: ColumnSetting[]; columns: ColumnDef[] });

export type Values = Record<string, unknown>;

/** Column list helper: every known column on/off in the stored order. */
export function columns(def: ColumnDef[], on: string[]): { columns: ColumnDef[]; default: ColumnSetting[] } {
  return { columns: def, default: def.map((c) => ({ id: c.id, on: on.includes(c.id) })) };
}

function mergeColumns(stored: unknown, def: Extract<SettingDef, { type: 'columns' }>): ColumnSetting[] {
  if (!Array.isArray(stored)) return def.default.map((c) => ({ ...c }));
  const known = new Set(def.columns.map((c) => c.id));
  // Each column once (the first entry wins), unknown ones dropped, new ones appended.
  const seen = new Set<string>();
  const out = (stored as ColumnSetting[])
    .filter((c) => c && known.has(c.id) && !seen.has(c.id) && !!seen.add(c.id))
    .map((c) => ({ id: c.id, on: !!c.on }));
  for (const d of def.default) if (!out.some((c) => c.id === d.id)) out.push({ ...d });
  return out;
}

/** Stored values over schema defaults; unknown keys are dropped, wrong types fall back. */
export function merge(schema: SettingDef[], stored: Values | undefined): Values {
  const v: Values = {};
  for (const s of schema) {
    const raw = stored?.[s.key];
    switch (s.type) {
      case 'bool':
        v[s.key] = typeof raw === 'boolean' ? raw : s.default;
        break;
      case 'number': {
        let n = typeof raw === 'number' && Number.isFinite(raw) ? raw : s.default;
        if (s.min != null) n = Math.max(s.min, n);
        if (s.max != null) n = Math.min(s.max, n);
        v[s.key] = n;
        break;
      }
      case 'columns':
        v[s.key] = mergeColumns(raw, s);
        break;
      case 'select':
        v[s.key] = typeof raw === 'string' && s.options.some((o) => o[0] === raw) ? raw : s.default;
        break;
      default:
        v[s.key] = typeof raw === 'string' ? raw : s.default;
    }
  }
  return v;
}

export function enabledColumns(v: unknown): string[] {
  return (v as ColumnSetting[]).filter((c) => c.on).map((c) => c.id);
}

/** Options every overlay gets. */
export const COMMON: SettingDef[] = [
  { key: 'opacity', label: 'Overlay opacity', type: 'number', default: 100, min: 10, max: 100, step: 1, unit: '%', help: 'Fades the whole overlay, text included', group: 'Transparency' },
  { key: 'bgOpacity', label: 'Background opacity', type: 'number', default: -1, min: -1, max: 100, step: 1, unit: '%', help: '-1 follows the theme; 0 = fully see-through background', group: 'Transparency' },
  { key: 'scale', label: 'Scale', type: 'number', default: 100, min: 40, max: 300, step: 5, unit: '%', group: 'General' },
  {
    key: 'autoHeight',
    label: 'Grow/shrink the window to fit the content',
    type: 'bool',
    default: false,
    help: 'The height follows the number of rows; you only set the width',
    group: 'General',
  },
  {
    key: 'autoWidth',
    label: 'Fit the window width to the columns',
    type: 'bool',
    default: false,
    help: 'Adding or removing columns resizes the overlay (relative, standings and other tables)',
    group: 'General',
  },
  { key: 'hideWhenDisconnected', label: 'Hide when iRacing is not running', type: 'bool', default: true, group: 'Visibility' },
  { key: 'hideWhenNotDriving', label: 'Hide unless I am in the car', type: 'bool', default: false, group: 'Visibility' },
  { key: 'hideInPits', label: 'Hide while on pit road', type: 'bool', default: false, group: 'Visibility' },
  { key: 'raceOnly', label: 'Only show in race sessions', type: 'bool', default: false, group: 'Visibility' },
  // VR (desktop app with SteamVR): where this overlay goes in the headset.
  { key: 'vrShow', label: 'Show in VR', type: 'bool', default: true, group: 'VR', help: 'When VR is on (General → VR)' },
  {
    key: 'vrAnchor', when: (v) => v.vrShow !== false,
    label: 'In VR it is',
    type: 'select',
    default: 'world',
    options: [
      ['world', 'Fixed in the car (stays put as you look around)'],
      ['head', 'Following my head (always in the same place in view)'],
    ],
    group: 'VR',
  },
  {
    key: 'vrPlace', when: (v) => v.vrShow !== false,
    label: 'Placed',
    type: 'select',
    default: 'screen',
    options: [
      ['screen', 'As on my screen'],
      ['custom', 'Where I set below'],
    ],
    group: 'VR',
    help: 'As on my screen: the layout of your monitor on an imaginary screen in front of you (its distance and size are under General → VR)',
  },
  { key: 'vrX', when: (v) => v.vrShow !== false && v.vrPlace === 'custom', label: 'Custom position: left / right', type: 'number', default: 0, min: -2, max: 2, step: 0.01, unit: 'm', group: 'VR', help: 'Custom placement: + is to the right' },
  { key: 'vrY', when: (v) => v.vrShow !== false && v.vrPlace === 'custom', label: 'Custom position: down / up', type: 'number', default: 0, min: -2, max: 2, step: 0.01, unit: 'm', group: 'VR', help: 'Custom placement: + is up' },
  { key: 'vrDistance', when: (v) => v.vrShow !== false && v.vrPlace === 'custom', label: 'Custom position: distance in front', type: 'number', default: 0.9, min: 0.1, max: 5, step: 0.01, unit: 'm', group: 'VR' },
  { key: 'vrYaw', when: (v) => v.vrShow !== false, label: 'Rotation: turn left / right', type: 'number', default: 0, min: -180, max: 180, step: 1, unit: '°', group: 'VR', help: 'However it is placed: turn it to face you when it is off to one side' },
  { key: 'vrPitch', when: (v) => v.vrShow !== false, label: 'Rotation: tilt up / down', type: 'number', default: 0, min: -90, max: 90, step: 1, unit: '°', group: 'VR' },
  { key: 'vrRoll', when: (v) => v.vrShow !== false, label: 'Rotation: roll', type: 'number', default: 0, min: -180, max: 180, step: 1, unit: '°', group: 'VR', help: 'Turns it in its own plane, like a picture on a wall' },
  { key: 'vrWidth', when: (v) => v.vrShow !== false && v.vrPlace === 'custom', label: 'Custom size: width', type: 'number', default: 0.3, min: 0.02, max: 3, step: 0.01, unit: 'm', group: 'VR', help: 'Custom placement: its size in the headset (the height follows)' },
  { key: 'vrOpacity', when: (v) => v.vrShow !== false, label: 'Opacity in VR', type: 'number', default: 100, min: 5, max: 100, step: 1, unit: '%', group: 'VR', help: 'SteamVR overlays only for now' },
  { key: 'customCss', label: 'Custom CSS', type: 'text', multiline: true, default: '', placeholder: '.row.player { outline: 1px solid gold; }', group: 'Advanced' },
];

/** Common options with an overlay's own defaults applied (e.g. standings auto-fits by default). */
export function commonFor(defaults?: Values, fitsWidth = false): SettingDef[] {
  // Fitting the width to the columns only means something for overlays with columns.
  const common = fitsWidth ? COMMON : COMMON.filter((s) => s.key !== 'autoWidth');
  if (!defaults) return common;
  return common.map((s) => (s.key in defaults ? ({ ...s, default: defaults[s.key] } as SettingDef) : s));
}

export const NAME_STYLES: [string, string][] = [
  ['full', 'Full name'],
  ['initialLast', 'J. Smith'],
  ['lastUpper', 'J. SMITH'],
  ['last', 'Smith'],
  ['first', 'John'],
];
