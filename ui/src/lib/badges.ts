// Small badges for the settings window.

import { h } from './dom';
import type { OverlayDef } from './overlay';

/** Marks an overlay, setting or button that is for Le Mans Ultimate. */
export function lmuBadge(title = 'Le Mans Ultimate only'): HTMLElement {
  const b = h('span', 'lmu-badge', 'LMU');
  b.title = title;
  return b;
}

/** Marks an overlay that is only for iRacing. */
export function iracingBadge(): HTMLElement {
  const b = h('span', 'lmu-badge iracing-badge', 'iRacing');
  b.title = 'iRacing only';
  return b;
}

/** `el` with an LMU badge after its content. */
export function withLmuBadge<T extends HTMLElement>(el: T): T {
  el.append(lmuBadge());
  return el;
}

/** The badges for an overlay: LMU, or iRacing for one only that game has. */
export function overlayBadges(def: OverlayDef | undefined): HTMLElement[] {
  if (!def) return [];
  if (def.game === 'iracing') return [iracingBadge()];
  return def.lmu || def.game === 'lmu' ? [lmuBadge()] : [];
}

/** The overlay works only in the other game from the one being set up (`game`: 'iracing', 'lmu' or ''). */
export function otherGameOnly(def: OverlayDef | undefined, game: string | undefined): boolean {
  return !!def?.game && (game === 'iracing' || game === 'lmu') && def.game !== game;
}
