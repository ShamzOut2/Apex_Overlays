// Contract between the overlay host page and individual overlays.

import type { Units } from './format';
import { commonFor, type SettingDef, type Values } from './settings';
import type { TopicMap, TopicName } from './types';

export interface Ctx {
  /** This overlay instance's id. */
  id: string;
  /** Merged settings (schema defaults + user values). */
  settings: Values;
  /** Merged theme values (for canvas overlays that need colours). */
  theme: Values;
  units: Units;
  latest: Partial<TopicMap>;
  send(msg: object): void;
  /** Change how often a topic is delivered (times a second; 0 = as it comes), e.g. less often while idle. */
  limit(topic: TopicName, hz: number): void;
}

export interface View {
  update(topic: TopicName, data: any): void;
  /** Called every animation frame while visible (optional). */
  frame?(): void;
}

export interface OverlayDef {
  kind: string;
  name: string;
  description: string;
  topics: TopicName[];
  /** Default window size in CSS pixels. */
  size: [number, number];
  settings: SettingDef[];
  /** Overrides for the defaults of the common options (scale, autoHeight, ...). */
  commonDefaults?: Values;
  /** Only for Le Mans Ultimate: shown with an LMU badge in the settings. */
  lmu?: boolean;
  /** Topics this overlay needs less often than they are sent: at most this many times a second. */
  rates?: Partial<Record<TopicName, number>>;
  /** Has table columns, so "fit the window width to the columns" applies. */
  fitsWidth?: boolean;
  create(root: HTMLElement, ctx: Ctx): View;
}

/** Full settings schema of an overlay: its own options plus the common ones. */
export function schemaOf(def: OverlayDef): SettingDef[] {
  return [...def.settings, ...commonFor(def.commonDefaults, def.fitsWidth)];
}
