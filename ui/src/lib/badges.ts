// Small badges for the settings window.

import { h } from './dom';

/** Marks an overlay, setting or button that is only for Le Mans Ultimate. */
export function lmuBadge(): HTMLElement {
  const b = h('span', 'lmu-badge', 'LMU');
  b.title = 'Le Mans Ultimate only';
  return b;
}

/** `el` with an LMU badge after its content. */
export function withLmuBadge<T extends HTMLElement>(el: T): T {
  el.append(lmuBadge());
  return el;
}
