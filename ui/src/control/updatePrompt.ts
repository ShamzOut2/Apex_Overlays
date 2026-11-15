// The "update available" pop-up: the settings page opened by the desktop app in a small
// window (?prompt=update) when a new version is found at start-up.

import { h } from '../lib/dom';
import { releaseHistory } from './releaseNotes';
import { labsLogo } from './brand';
import { client, state } from './state';

function button(label: string, onClick: () => void, cls = '') {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    onClick();
  };
  return b;
}

const close = () => {
  client.send({ op: 'closePrompt' });
  // Outside the desktop app (or if the host is gone), at least try to close the tab.
  setTimeout(() => window.close(), 300);
};

export function renderUpdatePrompt(el: HTMLElement) {
  const u = state.update;
  const box = h('section', 'card update-prompt');
  const version = u?.version ?? '';
  box.append(h('h2', '', version ? `Apex Overlays ${version} is available` : 'Apex Overlays update'));
  const bar = h('div', 'toolbar');
  switch (u?.status) {
    case 'available':
      box.append(h('p', 'dim', `You have version ${u.current ?? '?'}. Updating takes a few seconds; Apex Overlays restarts by itself and keeps all your settings.`));
      {
        const releases = u.releases?.length ? u.releases : u.notes ? [{ version, notes: u.notes }] : [];
        if (releases.length > 1) box.append(h('h3', '', `What’s new — ${releases.length} updates since your version`), releaseHistory(releases));
        else if (releases.length) box.append(h('h3', '', 'What’s new'), releaseHistory(releases));
      }
      bar.append(
        button('Update now', () => client.send({ op: 'installUpdate' }), 'primary'),
        button('Remind me tomorrow', () => client.send({ op: 'postponeUpdate', version, days: 1 })),
        button('Remind me in a week', () => client.send({ op: 'postponeUpdate', version, days: 7 })),
        button('Skip this version', () => client.send({ op: 'postponeUpdate', version, days: null }), 'subtle'),
      );
      break;
    case 'downloading':
      box.append(h('p', '', `Downloading… ${u.progress ?? 0}%`));
      break;
    case 'installing':
      box.append(h('p', '', 'Installing — Apex Overlays will restart in a moment.'));
      break;
    case 'error':
      box.append(h('p', 'bad', `The update failed: ${u.message ?? 'unknown error'}`));
      bar.append(button('Try again', () => client.send({ op: 'installUpdate' }), 'primary'), button('Close', close));
      break;
    case 'checking':
      box.append(h('p', 'dim', 'Checking for updates…'));
      break;
    default:
      box.append(h('p', 'dim', 'You are on the latest version.'));
      bar.append(button('Close', close));
  }
  box.append(bar, labsLogo('prompt-labs'));
  el.replaceChildren(box);
}
