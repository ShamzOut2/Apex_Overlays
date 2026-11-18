// Settings / control panel (served at /). Works in the desktop app and in any browser.

import { h, text } from './lib/dom';
import { leaguesPage } from './control/leagues';
import { teamsPage } from './control/teams';
import { raceControlPage } from './control/racecontrol';
import { marketPage } from './control/market';
import { renderUpdatePrompt } from './control/updatePrompt';
import { renderClosePrompt } from './control/closePrompt';
import { gameName, PAGES, type Page } from './control/pages';
import { client, listen, pageTopics, save, state } from './control/state';
import { applyUiTheme } from './lib/theme';
import { labsLogo } from './control/brand';
import { pageBody, pageHead, setPageHead } from './control/chrome';
import { sessionType } from './lib/sessionFields';

// In groups: what is on screen (the overlays first, the page the window opens on), the race
// weekend, and the app itself.
const byId = (id: string) => PAGES.find((p) => p.id === id)!;
const pages: Page[] = [byId('overlays'), marketPage, byId('theme'), byId('strategy'), leaguesPage, raceControlPage, byId('general'), byId('streaming')];

const nav = h('nav');
const brand = h('div', 'brand');
const logo = h('span', 'logo');
logo.append(h('b', '', 'Apex'), ' Overlays');
const mark = h('img', 'brand-mark') as HTMLImageElement;
mark.src = '/assets/brand/apex-overlays-icon-64.png';
mark.alt = '';
// Under the name: the version and who makes it.
const brandSub = h('span', 'brand-sub', 'Apex Labs');
const brandText = h('span', 'brand-text');
brandText.append(logo, brandSub);
brand.append(mark, brandText);
// The connection card under the name: the game, the session and the game switch.
const conn = h('section', 'conn');
conn.setAttribute('aria-label', 'Connection');
const connHead = h('div', 'conn-head');
const connState = h('span', 'conn-state');
const connBadge = h('span', 'conn-badge');
connHead.append(connState, connBadge);
const connInfo = h('div', 'conn-info');
const main = h('main');
main.append(pageHead, pageBody);
nav.append(brand, conn);
// Pages in groups; a heading before the first page of each group.
const GROUPS: Record<string, string> = { overlays: 'On screen', strategy: 'Race weekend', general: 'App' };
const GROUP_OF: Record<string, string> = { overlays: 'On screen', market: 'On screen', theme: 'On screen', strategy: 'Race weekend', leagues: 'Race weekend', teams: 'Race weekend', racecontrol: 'Race weekend', general: 'App settings', streaming: 'App' };
const links = new Map<string, HTMLAnchorElement>();
// Line icons for the sidebar (24×24, drawn with the current text colour).
const ICONS: Record<string, string> = {
  general: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>',
  overlays: '<rect x="3" y="3" width="13" height="13" rx="2"/><path d="M8 21h11a2 2 0 0 0 2-2V8"/>',
  theme: '<path d="M12 3a9 9 0 1 0 0 18c1 0 1.6-.8 1.6-1.6 0-.4-.2-.8-.4-1.1-.3-.3-.4-.7-.4-1.1 0-.9.7-1.6 1.6-1.6H16a5 5 0 0 0 5-5C21 6.6 17 3 12 3Z"/><circle cx="7.5" cy="10.5" r="1"/><circle cx="10.5" cy="7" r="1"/><circle cx="15" cy="7.5" r="1"/>',
  strategy: '<path d="M10 2h4"/><path d="M12 14l3-3"/><circle cx="12" cy="14" r="8"/>',
  leagues: '<path d="M8 21h8M12 17v4M7 4h10v5a5 5 0 0 1-10 0V4Z"/><path d="M17 5h3v2a3 3 0 0 1-3 3M7 5H4v2a3 3 0 0 0 3 3"/>',
  racecontrol: '<path d="M4 22V4"/><path d="M4 4h14l-2 4 2 4H4"/>',
  garage61: '<path d="M3 21V9l9-6 9 6v12"/><path d="M7 21v-8h10v8M7 17h10"/>',
  market: '<path d="M4 7h16l-1.5 11a2 2 0 0 1-2 1.7h-9a2 2 0 0 1-2-1.7L4 7Z"/><path d="M9 7V5a3 3 0 0 1 6 0v2"/><path d="M12 11v5m-2.5-2.5L12 16l2.5-2.5"/>',
  streaming: '<rect x="2" y="5" width="15" height="14" rx="2"/><path d="m17 10 5-3v10l-5-3"/>',
};
for (const p of pages) {
  if (GROUPS[p.id]) nav.append(h('div', 'nav-label', GROUPS[p.id]));
  const a = h('a');
  if (ICONS[p.id]) a.insertAdjacentHTML('beforeend', `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[p.id]}</svg>`);
  a.append(h('span', '', p.title));
  a.href = `#${p.id}`;
  nav.append(a);
  links.set(p.id, a);
}
// The game switch, in the connection card.
const gameSwitch = h('div', 'game-switch');
gameSwitch.title = 'Which game to read. Auto: whichever is running';
const GAMES: [string, string, string][] = [
  ['auto', 'Auto', 'Whichever game is running'],
  ['iracing', 'iRacing', 'iRacing only'],
  ['lmu', 'LMU', 'Le Mans Ultimate only'],
];
const gameButtons = GAMES.map(([id, label, tip]) => {
  const b = h('button', '', label);
  b.title = tip;
  b.onclick = () => {
    if (!state.config) return;
    state.config.game = id as 'auto' | 'iracing' | 'lmu';
    save(true);
    drawGameSwitch();
  };
  return { id, b };
});
gameSwitch.append(...gameButtons.map((g) => g.b));
function drawGameSwitch() {
  const chosen = state.config?.game ?? 'auto';
  const running = state.status?.connected ? state.status.source : '';
  for (const { id, b } of gameButtons) {
    b.classList.toggle('sel', id === chosen);
    // In Auto, the game found running is marked.
    b.classList.toggle('live', chosen === 'auto' && id === running);
  }
}
conn.append(connHead, connInfo, gameSwitch);
// Show / hide every overlay, with its hotkey.
const visRow = h('label', 'nav-visible');
const visText = h('span', '', 'Overlays on screen');
const visHint = h('small');
visText.append(visHint);
const visSwitch = h('input') as HTMLInputElement;
visSwitch.type = 'checkbox';
visSwitch.className = 'switch small';
visSwitch.setAttribute('aria-label', 'Show overlays on screen');
visSwitch.onchange = () => state.status && client.send({ op: 'setVisible', value: visSwitch.checked });
visRow.append(visText, visSwitch);
function drawVisible() {
  visSwitch.checked = state.status?.visible ?? true;
  const key = (state.config?.hotkeys?.toggleVisibility ?? '').replace('CmdOrCtrl', 'Ctrl').replace('Shift', '⇧').split('+').join(' ');
  text(visHint, key ? `Hotkey ${key}` : 'Show or hide all');
}
const navSpacer = h('div', 'nav-spacer');
const navFoot = h('div', 'nav-foot');
navFoot.append(labsLogo('nav-labs'));
nav.append(navSpacer, visRow, navFoot);
document.body.append(nav, main);
// The desktop app opens this page as a small "update available" or "close?" window.
const promptKind = new URLSearchParams(location.search).get('prompt');
const promptMode = promptKind === 'update' || promptKind === 'close';
if (promptMode) {
  nav.hidden = true;
  document.body.classList.add('prompt');
}

// Pages opened from another page rather than the sidebar (and the sidebar entry they belong to).
const subPages: [Page, string][] = [[teamsPage, 'leagues']];
const current = () => pages.find((p) => `#${p.id}` === location.hash) ?? subPages.find(([p]) => `#${p.id}` === location.hash)?.[0] ?? pages[0];

function render() {
  if (!state.config) {
    pageBody.replaceChildren(h('p', 'dim', 'Connecting to Apex Overlays…'));
    return;
  }
  applyUiTheme(state.config.theme, document.documentElement);
  if (promptKind === 'close') return renderClosePrompt(main);
  if (promptMode) return renderUpdatePrompt(main);
  const p = current();
  pageTopics(p.id);
  const parent = subPages.find(([s]) => s === p)?.[1];
  links.forEach((a, id) => a.classList.toggle('sel', id === p.id || id === parent));
  // The header: the overlays page names the layout in use.
  const cfg = state.config;
  const layout = p.id === 'overlays' ? cfg.layouts?.find((l) => l.id === cfg.activeLayout)?.name : '';
  setPageHead(layout ? `Layout · ${layout}` : (GROUP_OF[p.id] ?? ''), p.title);
  p.render(pageBody);
}

/** Time left to the minute ("1:20", "45 min"): the card changes once a minute, not every
 *  second (each change redraws the window). */
function minutesLeft(secs: number) {
  const m = Math.ceil(Math.max(0, secs) / 60);
  return m >= 60 ? `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}` : `${m} min`;
}

function renderStatus() {
  const s = state.status;
  const sess = state.session;
  const up = !!s?.connected;
  conn.className = `conn ${up ? 'up' : 'down'}`;
  const game = gameName(s?.source ?? '');
  text(connState, !s ? 'Not connected to the app' : up ? `${game} live` : `Waiting for ${s.source ? game : 'a game'}…`);
  // The session type (Practice, Race…), the track, and the cars and time left.
  const type = up && sess ? sessionType(sess) : '';
  text(connBadge, type);
  connBadge.hidden = !type;
  const lines: string[] = up && sess ? [[sess.trackName, sess.trackConfig].filter(Boolean).join(' · '), [sess.numCars ? `${sess.numCars} cars` : '', sess.timeRemain != null ? `${minutesLeft(sess.timeRemain)} left` : sess.lapsRemain != null ? `${sess.lapsRemain} laps left` : ''].filter(Boolean).join(' · ')].filter(Boolean) : [!s ? '' : 'iRacing or Le Mans Ultimate, when running'];
  // Only when it changes (the time left ticks with every session update).
  const key = lines.join('\n');
  if (connInfo.dataset.k !== key) {
    connInfo.dataset.k = key;
    connInfo.replaceChildren(...lines.map((l) => h('span', '', l)));
  }
  text(brandSub, s ? `v${s.version} · Apex Labs` : 'Apex Labs');
}

let lastLive = 0;
window.addEventListener('hashchange', render);
// Drop-down menus open towards the side with room (a narrow window wraps the toolbar).
document.addEventListener(
  'toggle',
  (e) => {
    const d = e.target as HTMLElement;
    if (!(d instanceof HTMLDetailsElement) || !d.open) return;
    const pop = d.querySelector<HTMLElement>(':scope > .menu-pop');
    if (!pop) return;
    pop.style.left = '';
    pop.style.right = '';
    const r = pop.getBoundingClientRect();
    const mainLeft = main.getBoundingClientRect().left;
    if (r.left < mainLeft + 8) {
      pop.style.left = '0';
      pop.style.right = 'auto';
    }
  },
  true,
);
// Escape closes the dialog on top (Add overlay, Share a layout, previews…).
window.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape') return;
  const dialogs = document.querySelectorAll('.modal-back');
  if (dialogs.length) {
    e.preventDefault();
    dialogs[dialogs.length - 1].remove();
  }
});
let leagueKey = '';
let wasInVr = false;
let lastGame = '';
listen((t) => {
  if (t === 'config' || t === 'status') {
    drawGameSwitch();
    drawVisible();
  }
  if (t === 'config') render();
  // The close pop-up says more when you are in a session.
  if (t === 'status' && promptKind === 'close') render();
  if (t === 'status') {
    renderStatus();
    // The Game card on General says which game is being read.
    const game = `${state.status?.source}|${state.status?.connected}`;
    if (game !== lastGame && current().id === 'general' && !(document.activeElement instanceof HTMLInputElement)) render();
    lastGame = game;
    // Edit-mode / visibility buttons live on the overlays page.
    if (current().id === 'overlays') render();
  }
  if (t === 'session') {
    renderStatus();
    // The league builder says which league applies to the session you are in.
    const s = state.session;
    const k = `${s?.connected}|${s?.leagueId}|${s?.league}`;
    if (k !== leagueKey) {
      leagueKey = k;
      if (current().id === 'leagues' && !(document.activeElement instanceof HTMLInputElement)) render();
    }
  }
  if ((t === 'trackmap' || t === 'vr') && current().id === 'general') render();
  // Entering or leaving VR: the overlay editor's position boxes switch between the screen and VR positions.
  if (t === 'vr') {
    const inVr = state.vr?.status === 'on';
    if (inVr !== wasInVr && current().id === 'overlays' && !(document.activeElement instanceof HTMLInputElement)) render();
    wasInVr = inVr;
  }
  if (t === 'update' && (promptKind === 'update' || (!promptMode && current().id === 'general'))) render();
  // Race control lists live flags: refresh every couple of seconds unless the user is typing.
  if (t === 'standings' && current().id === 'racecontrol' && Date.now() - lastLive > 2000 && !(document.activeElement instanceof HTMLInputElement)) {
    lastLive = Date.now();
    render();
  }
});
render();
