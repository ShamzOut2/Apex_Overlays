// Apex Labs branding: Apex Overlays is a tool by Apex Labs.

import { h } from '../lib/dom';

/** "Apex Labs" logo: white lettering on dark themes, dark lettering on light ones (the
 *  settings window marks light themes with the `ui-light` class). */
export function labsLogo(cls: string): HTMLElement {
  const box = h('div', `labs-logo ${cls}`);
  box.title = 'Apex Overlays is a tool by Apex Labs';
  for (const v of ['dark', 'light']) {
    const img = h('img', `on-${v}`) as HTMLImageElement;
    img.src = `/assets/brand/apex-labs-logo-on-${v}.svg`;
    img.alt = v === 'dark' ? 'Apex Labs' : '';
    box.append(img);
  }
  return box;
}
