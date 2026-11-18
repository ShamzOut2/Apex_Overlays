// The "close Apex Overlays?" pop-up: the settings page opened by the desktop app in a small
// window (?prompt=close) when the settings window is closed.

import { h } from '../lib/dom';
import { client, state } from './state';

function button(label: string, onClick: () => void, cls = '') {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    onClick();
  };
  return b;
}

export function renderClosePrompt(el: HTMLElement) {
  const box = h('section', 'card update-prompt');
  // In a session: the tray is the better choice (the overlays keep running, and the settings
  // window's CPU and memory are freed for the game).
  const inSession = !!state.status?.connected;
  if (inSession) {
    box.append(
      h('h2', '', 'Minimise to the tray?'),
      h('p', 'dim', 'You are in a session. In the tray your overlays keep running and the settings window stops using any CPU or memory, leaving more for the game. Open it again from the tray icon.'),
    );
  } else {
    box.append(h('h2', '', 'Close Apex Overlays?'), h('p', 'dim', 'Close the overlays as well, or keep them on screen? Kept overlays run from the tray icon (right-click it to quit later).'));
  }
  const remember = h('label', 'inline small');
  const cb = h('input');
  cb.type = 'checkbox';
  remember.append(cb, ' Don’t ask again (change it under General)');
  const choose = (quit: boolean) => client.send({ op: 'closeApp', quit, remember: cb.checked });
  const bar = h('div', 'toolbar');
  if (inSession) {
    bar.append(
      button('Minimise to tray', () => choose(false), 'primary'),
      button('Close everything', () => choose(true)),
      button('Cancel', () => client.send({ op: 'closePrompt' }), 'subtle'),
    );
  } else {
    bar.append(
      button('Close everything', () => choose(true), 'primary'),
      button('Keep overlays running', () => choose(false)),
      button('Cancel', () => client.send({ op: 'closePrompt' }), 'subtle'),
    );
  }
  box.append(bar, remember);
  el.replaceChildren(box);
}
