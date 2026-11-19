// How an overlay's settings are laid out in the editor: the same six tabs on every overlay,
// each split into sections (the settings' groups), options nested under the switch that
// turns them on, and a hand-picked "Essentials" tab.

import type { SettingDef, Values } from '../lib/settings';

export const TABS = ['Essentials', 'Content', 'Layout', 'Style', 'Behaviour', 'Window'] as const;
export type Tab = (typeof TABS)[number];

/** Which tab a group's section goes in; any other group is Content (what the overlay shows). */
const GROUP_TAB: Record<string, Tab> = {
  Layout: 'Layout',
  Header: 'Layout',
  Orientation: 'Layout',
  View: 'Layout',
  Colours: 'Style',
  Look: 'Style',
  Style: 'Style',
  Format: 'Style',
  'Column style': 'Style',
  Behaviour: 'Behaviour',
  Alerts: 'Behaviour',
  Warnings: 'Behaviour',
  'Pop-up': 'Behaviour',
  Replay: 'Behaviour',
  Visibility: 'Behaviour',
  Transparency: 'Window',
  General: 'Window',
  VR: 'Window',
  Advanced: 'Window',
};

/** Groups named differently by different overlays, shown under one name. */
const SECTION_NAME: Record<string, string> = {
  Elements: 'What to show',
  Fields: 'What to show',
  Style: 'Look',
  Format: 'Look',
  Warnings: 'Alerts',
  Options: 'Settings',
  General: 'Size',
  Visibility: 'When it shows',
};

export const tabOf = (s: SettingDef): Tab => GROUP_TAB[s.group ?? ''] ?? 'Content';
export const sectionOf = (s: SettingDef): string => SECTION_NAME[s.group ?? 'Options'] ?? s.group ?? 'Settings';

/** The settings most people change, per overlay, in this order (then Show and Text size). */
const ESSENTIALS: Record<string, string[]> = {
  relative: ['carsAhead', 'carsBehind', 'columns', 'look', 'gapMode', 'showHeader'],
  standings: ['mode', 'maxRows', 'otherClassRows', 'columns', 'look', 'showClassHeader'],
  trackmap: ['style', 'rotate', 'label', 'colorBy', 'showSectors', 'cornerLabels', 'dotSize'],
  inputs: ['showShiftLights', 'showTrace', 'showBars', 'showWheel', 'showCornerSpeeds', 'sideLayout', 'wheelStyle'],
  shiftlights: ['showRpm', 'rpmPosition', 'shiftLayout', 'shiftLightSize'],
  inputGraph: ['traceSeconds', 'traceSteering', 'showClutch', 'traceGrid'],
  laptimes: ['laps', 'showSectors', 'showDelta', 'showFooter'],
  fuel: ['mode', 'show_atFinish', 'show_race', 'show_fuelToAdd', 'showHistory', 'avgLaps'],
  energy: ['showBar', 'showBattery', 'showPlan', 'hideWithout'],
  battle: ['mode', 'showAhead', 'showBehind', 'showGapBar', 'onlyInBattle'],
  session: ['fields', 'layout', 'showLabels', 'clock24'],
  flags: ['style', 'showLabel', 'maxFlags', 'hideWhenNone'],
  startLights: ['countdown', 'lights', 'goStyle', 'rolling'],
  weather: ['compassSize', 'carUp', 'showTrack', 'showAir', 'showRain'],
  forecast: ['interval', 'tiles', 'showNow', 'showRain'],
  tyres: ['showGauge', 'showReasons', 'onlyWhenRelevant', 'flash'],
  tyredata: ['tempShown', 'showPressure', 'showWear', 'showBrakes', 'condensed'],
  carsettings: ['items', 'layout', 'popup', 'showMeter'],
  databox: ['field', 'label', 'valueSize', 'align'],
  spotter: ['style', 'barWidth', 'showCar', 'holdMs'],
  radar: ['range', 'hideWithin', 'showNumbers', 'colorBy'],
  safety: ['showHazards', 'showOvertake', 'showApproach', 'nameStyle'],
  delta: ['reference', 'showBar', 'showPredicted', 'decimals'],
  pit: ['decel', 'margin', 'showLimit'],
  incidents: ['rows', 'showSummary', 'showReplay', 'hideWhenNone'],
  monitor: ['stats', 'layout', 'warnCpu'],
};
const COMMON_ESSENTIALS = ['visibleWhen', 'textSize'];

/** The keys of an overlay's Essentials tab. */
export function essentials(kind: string): string[] {
  return [...(ESSENTIALS[kind] ?? []), ...COMMON_ESSENTIALS];
}

/** A setting with the options that depend on it nested under it. */
export interface SettingNode {
  s: SettingDef;
  kids: SettingNode[];
}

/**
 * The settings of a section as a tree: a setting that only applies while an earlier switch
 * of the same section is on goes under that switch (the nearest one, if several). Worked out
 * from the settings' own conditions, so nothing has to be declared twice.
 */
export function tree(list: SettingDef[], values: Values): SettingNode[] {
  const parent = new Map<string, string>();
  list.forEach((x, i) => {
    if (!x.when) return;
    for (let j = i - 1; j >= 0; j--) {
      const b = list[j];
      if (b.type !== 'bool') continue;
      try {
        if (x.when({ ...values, [b.key]: true }) && !x.when({ ...values, [b.key]: false })) {
          parent.set(x.key, b.key);
          break;
        }
      } catch {
        /* A condition that cannot be tried this way: the setting stays at the top. */
      }
    }
  });
  const nodes = new Map(list.map((s) => [s.key, { s, kids: [] as SettingNode[] }]));
  const roots: SettingNode[] = [];
  for (const s of list) {
    const n = nodes.get(s.key)!;
    const p = parent.get(s.key);
    if (p && nodes.has(p)) nodes.get(p)!.kids.push(n);
    else roots.push(n);
  }
  return roots;
}

/** The switch that turns a whole section on and off (its only top setting, with the rest under it). */
export function masterOf(roots: SettingNode[], count: number): SettingDef | null {
  return roots.length === 1 && roots[0].s.type === 'bool' && roots[0].kids.length > 0 && count > 1 ? roots[0].s : null;
}

/** Sections folded away (the chevron), per kind of overlay, kept between visits. */
const FOLD_KEY = 'apex.folded';
let folds: Record<string, string[]> | null = null;
const readFolds = () => {
  if (folds) return folds;
  try {
    folds = JSON.parse(localStorage.getItem(FOLD_KEY) ?? '{}') as Record<string, string[]>;
  } catch {
    folds = {};
  }
  return folds;
};
export const isFolded = (kind: string, section: string) => readFolds()[kind]?.includes(section) ?? false;
export function setFolded(kind: string, section: string, on: boolean) {
  const f = readFolds();
  const list = new Set(f[kind] ?? []);
  if (on) list.add(section);
  else list.delete(section);
  f[kind] = [...list];
  try {
    localStorage.setItem(FOLD_KEY, JSON.stringify(f));
  } catch {
    /* Not kept: it is only a convenience. */
  }
}
